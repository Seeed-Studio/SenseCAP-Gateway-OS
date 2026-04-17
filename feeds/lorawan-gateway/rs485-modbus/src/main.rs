use chrono::Local;
use rumqttc::{AsyncClient, Event, Incoming, MqttOptions, QoS, Transport};
use rumqttc::tokio_rustls::rustls::ClientConfig as RustlsClientConfig;
use rustls_pemfile::{certs, pkcs8_private_keys};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::process::Command;
use std::sync::{Arc, Mutex as StdMutex};
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::time::sleep;
use tokio_serial::{DataBits, Parity, StopBits, SerialPortBuilderExt};
use tokio_modbus::prelude::*;
use std::path::Path;

/// Extract port number from device name, e.g. "/dev/RS485-1" -> "1", "/dev/RS485-2" -> "2"
fn get_port_suffix(device: &str) -> String {
    device.rsplit('-').next()
        .and_then(|s| s.parse::<u8>().ok())
        .map(|n| format!("_{}", n))
        .unwrap_or_default()
}

// Configuration Structures
#[derive(Debug, Clone, PartialEq)]
struct Config {
    mqtt: MqttConfig,
    serial: SerialConfig,
    protocol: ProtocolConfig,
}

#[derive(Debug, Clone, PartialEq)]
struct MqttConfig {
    enabled: bool,
    transport: String,
    host: String,
    port: u16,
    username: Option<String>,
    password: Option<String>,
    client_id: String,
    keepalive: u64,
    uplink_topic: String,
    downlink_topic: String,
    qos_level: QoS,
    reconnect_delay: u64,
    auth_mode: String,
    ca_cert: Option<String>,
    client_cert: Option<String>,
    client_key: Option<String>,
    token: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
struct SerialConfig {
    device: String,
    baudrate: u32,
    databit: DataBits,
    stopbit: StopBits,
    checkbit: Parity,
    flowcontrol: tokio_serial::FlowControl,
    timeout: Duration,
}

#[derive(Debug, Clone, PartialEq)]
struct ProtocolConfig {
    device_address: u8,
    function_code: u8,
    register_addresses: Vec<u16>,
    data_length: u16,
    write_value: String,
    standard_mode: bool,
    work_mode: String,
    poll_interval: u64,
    timeout: u64,
}

// MQTT Message Structures
#[derive(Debug, Serialize)]
struct UplinkMessage {
    device: DeviceInfo,
    data: ModbusData,
    #[serde(skip_serializing_if = "Option::is_none")]
    error_code: Option<String>,
}

#[derive(Debug, Serialize)]
struct DeviceInfo {
    sn: String,
    mac: String,
}

#[derive(Debug, Serialize)]
struct ModbusData {
    protocol: String,
    direction: String,
    slave_id: u8,
    function_code: u8,
    starting_address: u16,
    quantity: u16,
    values: serde_json::Value,
}

#[derive(Debug, Deserialize)]
struct DownlinkMessage {
    data: String,
}

/// Read device info from /etc/deviceinfo/ and /sys/class/net/eth0/address
fn get_device_info() -> DeviceInfo {
    let sn = fs::read_to_string("/etc/deviceinfo/sn")
        .unwrap_or_default().trim().to_string();
    let mac = fs::read_to_string("/sys/class/net/eth0/address")
        .unwrap_or_default().trim().to_string();
    DeviceInfo { sn, mac }
}

/// Build a structured MQTT uplink message for Modbus RTU response
fn build_modbus_uplink(
    config: &ProtocolConfig,
    data_str: &str,
    error: Option<String>,
) -> String {
    // Try to parse the data string as JSON values
    let values: serde_json::Value = serde_json::from_str(data_str)
        .unwrap_or_else(|_| json!(data_str));

    let direction = match config.function_code {
        1 | 2 | 3 | 4 => "response",
        _ => "response",
    };

    let msg = UplinkMessage {
        device: get_device_info(),
        data: ModbusData {
            protocol: "Modbus RTU".to_string(),
            direction: direction.to_string(),
            slave_id: config.device_address,
            function_code: config.function_code,
            starting_address: *config.register_addresses.first().unwrap_or(&0),
            quantity: config.data_length,
            values,
        },
        error_code: error,
    };

    serde_json::to_string(&msg).unwrap_or_else(|_| format!("{{\"error\":\"serialization failed\"}}"))
}

/// Build a structured MQTT uplink error message for Modbus RTU
fn build_modbus_error_uplink(
    config: &ProtocolConfig,
    error_msg: &str,
) -> String {
    let msg = UplinkMessage {
        device: get_device_info(),
        data: ModbusData {
            protocol: "Modbus RTU".to_string(),
            direction: "error".to_string(),
            slave_id: config.device_address,
            function_code: config.function_code,
            starting_address: *config.register_addresses.first().unwrap_or(&0),
            quantity: config.data_length,
            values: json!(null),
        },
        error_code: Some(error_msg.to_string()),
    };

    serde_json::to_string(&msg).unwrap_or_else(|_| format!("{{\"error\":\"serialization failed\"}}"))
}

// Logger Structure
struct Logger {
    file: StdMutex<Option<File>>,
}

impl Logger {
    fn new() -> Self {
        Logger {
            file: StdMutex::new(None),
        }
    }

    fn init(&self) -> std::io::Result<()> {
        std::fs::create_dir_all("/tmp/rs485")?;
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open("/tmp/rs485/log")?;
        let mut file_guard = self.file.lock().unwrap();
        *file_guard = Some(file);
        Ok(())
    }

    fn log(&self, message: &str) {
        let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S");
        let log_line = format!("[{}][RS485-Modbus]: {}\n", timestamp, message);
        print!("{}", log_line);
        if let Ok(mut file_guard) = self.file.lock() {
            if let Some(ref mut file) = *file_guard {
                let _ = file.write_all(log_line.as_bytes());
                let _ = file.flush();
            }
        }
    }
}

// Parse a string as u8, supporting both hexadecimal (0x prefix) and decimal formats
fn parse_hex_or_dec_u8(s: &str) -> Option<u8> {
    let s = s.trim();
    if s.starts_with("0x") || s.starts_with("0X") {
        u8::from_str_radix(&s[2..], 16).ok()
    } else {
        s.parse().ok()
    }
}

// Load configuration from UCI
fn load_config_from_uci(port_num: u8) -> Result<Config, Box<dyn std::error::Error + Send + Sync>> {
    let uci_get = |config: &str, section: &str, option: &str| -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        let output = Command::new("uci")
            .args(&["get", &format!("{}.{}.{}", config, section, option)])
            .output()?;
        if output.status.success() {
            Ok(String::from_utf8(output.stdout)?.trim().to_string())
        } else {
            Err(format!("uci get failed: {}.{}.{}", config, section, option).into())
        }
    };

    let sid = format!("port{}", port_num);

    // MQTT config — read from portN section with mqtt_ prefix
    let mqtt_enabled = uci_get("rs485-module", &sid, "mqtt_host")
        .ok()
        .map(|s| !s.is_empty())
        .unwrap_or(false);
    let host = uci_get("rs485-module", &sid, "mqtt_host").unwrap_or_default();
    let port = uci_get("rs485-module", &sid, "mqtt_port")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(1883);
    let username = uci_get("rs485-module", &sid, "mqtt_username").ok();
    let password = uci_get("rs485-module", &sid, "mqtt_password").ok();
    let client_id = uci_get("rs485-module", &sid, "mqtt_client_id").unwrap_or_else(|_| format!("rs485_modbus_{}", port_num));
    let keepalive = uci_get("rs485-module", &sid, "mqtt_keepalive")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(30);
    let uplink_topic = uci_get("rs485-module", &sid, "mqtt_uplink_topic").unwrap_or_else(|_| format!("rs485/CH{}/uplink", port_num));
    let downlink_topic = uci_get("rs485-module", &sid, "mqtt_downlink_topic").unwrap_or_else(|_| format!("rs485/CH{}/downlink", port_num));
    let qos_level = uci_get("rs485-module", &sid, "mqtt_qos")
        .ok()
        .and_then(|s| s.parse::<u8>().ok())
        .unwrap_or(0);
    let reconnect_delay = uci_get("rs485-module", &sid, "mqtt_reconnect_delay")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(30);
    let transport = uci_get("rs485-module", &sid, "mqtt_transport").unwrap_or_else(|_| "tcp".to_string());
    let auth_mode = uci_get("rs485-module", &sid, "mqtt_auth_mode").unwrap_or_else(|_| "none".to_string());
    let ca_cert = uci_get("rs485-module", &sid, "mqtt_ca_cert").ok();
    let client_cert = uci_get("rs485-module", &sid, "mqtt_client_cert").ok();
    let client_key = uci_get("rs485-module", &sid, "mqtt_client_key").ok();
    let token = uci_get("rs485-module", &sid, "mqtt_token").ok();

    let mqtt_config = MqttConfig {
        enabled: mqtt_enabled,
        transport,
        host,
        port,
        username,
        password,
        client_id,
        keepalive,
        uplink_topic,
        downlink_topic,
        qos_level: match qos_level {
            1 => QoS::AtLeastOnce,
            2 => QoS::ExactlyOnce,
            _ => QoS::AtMostOnce,
        },
        reconnect_delay,
        auth_mode,
        ca_cert,
        client_cert,
        client_key,
        token,
    };

    // Serial config — read from portN section, device derived from port number
    let device = format!("/dev/RS485-{}", port_num);
    let baudrate = uci_get("rs485-module", &sid, "baudrate")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(9600);
    let databit = uci_get("rs485-module", &sid, "databit")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(8);
    let stopbit = uci_get("rs485-module", &sid, "stopbit").unwrap_or_else(|_| "1".to_string());
    let checkbit = uci_get("rs485-module", &sid, "checkbit").unwrap_or_else(|_| "none".to_string());
    let flowcontrol = uci_get("rs485-module", &sid, "flowcontrol").unwrap_or_else(|_| "none".to_string());
    let timeout = uci_get("rs485-module", &sid, "timeout")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(1000);

    let serial_config = SerialConfig {
        device,
        baudrate,
        databit: match databit {
            5 => DataBits::Five,
            6 => DataBits::Six,
            7 => DataBits::Seven,
            _ => DataBits::Eight,
        },
        stopbit: match stopbit.as_str() {
            "2" => StopBits::Two,
            _ => StopBits::One,
        },
        checkbit: match checkbit.as_str() {
            "odd" => Parity::Odd,
            "even" => Parity::Even,
            _ => Parity::None,
        },
        flowcontrol: match flowcontrol.as_str() {
            "rtscts" => tokio_serial::FlowControl::Hardware,
            "xonxoff" => tokio_serial::FlowControl::Software,
            _ => tokio_serial::FlowControl::None,
        },
        timeout: Duration::from_millis(timeout),
    };

    // Protocol config — read from portN section with modbus_ prefix
    let device_address = uci_get("rs485-module", &sid, "modbus_device_address")
        .ok()
        .and_then(|s| parse_hex_or_dec_u8(&s))
        .unwrap_or(1);
    let function_code = uci_get("rs485-module", &sid, "modbus_function_code")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(3);
    let register_addresses: Vec<u16> = uci_get("rs485-module", &sid, "modbus_register_address")
        .ok()
        .map(|s| {
            s.split(',')
                .filter_map(|part| part.trim().parse::<u16>().ok())
                .collect::<Vec<u16>>()
        })
        .unwrap_or_default();
    let register_addresses = if register_addresses.is_empty() {
        vec![40001]
    } else {
        register_addresses
    };
    let data_length = uci_get("rs485-module", &sid, "modbus_data_length")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(10);

    let write_value = uci_get("rs485-module", &sid, "modbus_write_value")
        .unwrap_or_else(|_| "0".to_string());
    
    let standard_mode = uci_get("rs485-module", &sid, "modbus_standard_mode")
        .ok()
        .and_then(|s| s.parse::<u8>().ok())
        .unwrap_or(1) == 1;

    let work_mode = uci_get("rs485-module", &sid, "modbus_work_mode")
        .unwrap_or_else(|_| "once".to_string());
    
    let poll_interval = uci_get("rs485-module", &sid, "modbus_poll_interval")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(10);
    
    let timeout = uci_get("rs485-module", &sid, "modbus_timeout")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(10);

    let protocol_config = ProtocolConfig {
        device_address,
        function_code,
        register_addresses,
        data_length,
        write_value,
        standard_mode,
        work_mode,
        poll_interval,
        timeout,
    };

    Ok(Config {
        mqtt: mqtt_config,
        serial: serial_config,
        protocol: protocol_config,
    })
}

// Setup MQTT client
fn setup_mqtt_client(
    config: &MqttConfig,
) -> Result<(AsyncClient, rumqttc::EventLoop), Box<dyn std::error::Error + Send + Sync>> {
    let mut mqttoptions = MqttOptions::new(&config.client_id, &config.host, config.port);
    mqttoptions.set_keep_alive(Duration::from_secs(config.keepalive));
    
    if let (Some(username), Some(password)) = (&config.username, &config.password) {
        mqttoptions.set_credentials(username, password);
    }

    match config.transport.as_str() {
        "ssl" | "tls" => {
            let mut root_cert_store = rumqttc::tokio_rustls::rustls::RootCertStore::empty();
            
            match config.auth_mode.as_str() {
                "tls-server" => {
                    if let Some(ca_cert) = &config.ca_cert {
                        let ca_cert_bytes = ca_cert.as_bytes();
                        let mut cursor = std::io::Cursor::new(ca_cert_bytes);
                        for cert in certs(&mut cursor) {
                            root_cert_store.add(cert?)?;
                        }
                    } else {
                        root_cert_store.extend(
                            webpki_roots::TLS_SERVER_ROOTS.iter().cloned()
                        );
                    }
                    
                    let tls_config = RustlsClientConfig::builder()
                        .with_root_certificates(root_cert_store)
                        .with_no_client_auth();
                    
                    mqttoptions.set_transport(Transport::tls_with_config(tls_config.into()));
                }
                "mutual-tls" => {
                    if let Some(ca_cert) = &config.ca_cert {
                        let ca_cert_bytes = ca_cert.as_bytes();
                        let mut cursor = std::io::Cursor::new(ca_cert_bytes);
                        for cert in certs(&mut cursor) {
                            root_cert_store.add(cert?)?;
                        }
                    } else {
                        root_cert_store.extend(
                            webpki_roots::TLS_SERVER_ROOTS.iter().cloned()
                        );
                    }
                    
                    if let (Some(client_cert_pem), Some(client_key_pem)) = (&config.client_cert, &config.client_key) {
                        let cert_bytes = client_cert_pem.as_bytes();
                        let key_bytes = client_key_pem.as_bytes();
                        
                        let mut cert_cursor = std::io::Cursor::new(cert_bytes);
                        let certs: Vec<_> = certs(&mut cert_cursor).collect::<Result<_, _>>()?;
                        
                        let mut key_cursor = std::io::Cursor::new(key_bytes);
                        let mut keys = pkcs8_private_keys(&mut key_cursor).collect::<Result<Vec<_>, _>>()?;
                        
                        if keys.is_empty() {
                            return Err("No private key found".into());
                        }
                        
                        let tls_config = RustlsClientConfig::builder()
                            .with_root_certificates(root_cert_store)
                            .with_client_auth_cert(certs, keys.remove(0).into())?;
                        
                        mqttoptions.set_transport(Transport::tls_with_config(tls_config.into()));
                    } else {
                        return Err("Mutual TLS requires both client certificate and private key".into());
                    }
                }
                _ => {
                    root_cert_store.extend(
                        webpki_roots::TLS_SERVER_ROOTS.iter().cloned()
                    );
                    
                    let tls_config = RustlsClientConfig::builder()
                        .with_root_certificates(root_cert_store)
                        .with_no_client_auth();
                    
                    mqttoptions.set_transport(Transport::tls_with_config(tls_config.into()));
                }
            }
        }
        "ws" => {
            let ws_url = format!("ws://{}:{}/mqtt", config.host, config.port);
            mqttoptions = MqttOptions::new(&config.client_id, &ws_url, config.port);
            mqttoptions.set_keep_alive(Duration::from_secs(config.keepalive));
            if let Some(username) = &config.username {
                mqttoptions.set_credentials(username, config.password.as_deref().unwrap_or(""));
            }
            mqttoptions.set_transport(Transport::Ws);
        }
        "wss" => {
            let wss_url = format!("wss://{}:{}/mqtt", config.host, config.port);
            mqttoptions = MqttOptions::new(&config.client_id, &wss_url, config.port);
            mqttoptions.set_keep_alive(Duration::from_secs(config.keepalive));
            if let Some(username) = &config.username {
                mqttoptions.set_credentials(username, config.password.as_deref().unwrap_or(""));
            }
            
            let mut root_cert_store = rumqttc::tokio_rustls::rustls::RootCertStore::empty();
            
            if let Some(ca_cert) = &config.ca_cert {
                let ca_cert_bytes = ca_cert.as_bytes();
                let mut cursor = std::io::Cursor::new(ca_cert_bytes);
                for cert in certs(&mut cursor) {
                    root_cert_store.add(cert?)?;
                }
            } else {
                root_cert_store.extend(
                    webpki_roots::TLS_SERVER_ROOTS.iter().cloned()
                );
            }
            
            let tls_config = match config.auth_mode.as_str() {
                "mutual-tls" => {
                    if let (Some(client_cert_pem), Some(client_key_pem)) = (&config.client_cert, &config.client_key) {
                        let cert_bytes = client_cert_pem.as_bytes();
                        let key_bytes = client_key_pem.as_bytes();
                        
                        let mut cert_cursor = std::io::Cursor::new(cert_bytes);
                        let certs: Vec<_> = certs(&mut cert_cursor).collect::<Result<_, _>>()?;
                        
                        let mut key_cursor = std::io::Cursor::new(key_bytes);
                        let mut keys = pkcs8_private_keys(&mut key_cursor).collect::<Result<Vec<_>, _>>()?;
                        
                        if keys.is_empty() {
                            return Err("No private key found".into());
                        }
                        
                        RustlsClientConfig::builder()
                            .with_root_certificates(root_cert_store)
                            .with_client_auth_cert(certs, keys.remove(0).into())?
                    } else {
                        return Err("Mutual TLS requires both client certificate and private key".into());
                    }
                }
                _ => {
                    RustlsClientConfig::builder()
                        .with_root_certificates(root_cert_store)
                        .with_no_client_auth()
                }
            };
            
            mqttoptions.set_transport(Transport::wss_with_config(tls_config.into()));
        }
        "tcp" | _ => {
            mqttoptions.set_transport(Transport::Tcp);
        }
    }
    
    let (client, eventloop) = AsyncClient::new(mqttoptions, 10);
    Ok((client, eventloop))
}

// Setup serial port
async fn setup_serial(
    config: &SerialConfig,
) -> Result<tokio_serial::SerialStream, Box<dyn std::error::Error + Send + Sync>> {
    let port = tokio_serial::new(&config.device, config.baudrate)
        .data_bits(config.databit)
        .stop_bits(config.stopbit)
        .parity(config.checkbit)
        .flow_control(config.flowcontrol)
        .timeout(config.timeout)
        .open_native_async()?;
    
    Ok(port)
}

// CRC-16 Modbus calculation
fn crc16_modbus(data: &[u8]) -> u16 {
    let mut crc: u16 = 0xFFFF;
    for &byte in data {
        crc ^= byte as u16;
        for _ in 0..8 {
            if crc & 0x0001 != 0 {
                crc = (crc >> 1) ^ 0xA001;
            } else {
                crc >>= 1;
            }
        }
    }
    crc
}

// Group consecutive addresses for optimized batch reading.
// Input:  [40001, 40002, 40003, 40005, 40006, 40010]
// Output: [(40001, [40001,40002,40003]), (40005, [40005,40006]), (40010, [40010])]
fn group_consecutive_addresses(addresses: &[u16]) -> Vec<(u16, Vec<u16>)> {
    if addresses.is_empty() {
        return vec![];
    }
    let mut sorted: Vec<u16> = addresses.to_vec();
    sorted.sort();
    sorted.dedup();

    let mut groups: Vec<(u16, Vec<u16>)> = vec![];
    let mut group_start = sorted[0];
    let mut group_addrs = vec![sorted[0]];

    for i in 1..sorted.len() {
        if sorted[i] == sorted[i - 1] + 1 {
            group_addrs.push(sorted[i]);
        } else {
            groups.push((group_start, group_addrs));
            group_start = sorted[i];
            group_addrs = vec![sorted[i]];
        }
    }
    groups.push((group_start, group_addrs));
    groups
}

// Read Modbus data
async fn read_modbus_data(
    ctx: &mut client::Context,
    config: &ProtocolConfig,
    serial_config: &SerialConfig,
    logger: &Arc<Logger>,
) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    ctx.set_slave(Slave(config.device_address));
    let first_addr = config.register_addresses[0];
    
    // For write operations (FC05/06/15/16), check if non-standard mode is enabled
    if matches!(config.function_code, 5 | 6 | 15 | 16) && !config.standard_mode {
        let addr = first_addr;
        // Non-standard mode: parse space-separated hex data (e.g., "00 5A")
        let write_val_str = config.write_value.trim();
        let parts: Vec<&str> = write_val_str.split_whitespace().collect();
        let mut data_bytes = Vec::new();
        
        for part in &parts {
            match u8::from_str_radix(part, 16) {
                Ok(byte) => data_bytes.push(byte),
                Err(_) => return Err(format!("Invalid hex value: {}", part).into()),
            }
        }
        
        if data_bytes.is_empty() {
            return Err("No data bytes provided in non-standard mode".into());
        }
        
        // Build frame based on function code
        let mut frame = vec![config.device_address, config.function_code];
        
        match config.function_code {
            5 => {
                // FC05: Write Single Coil
                frame.push((addr >> 8) as u8);
                frame.push((addr & 0xFF) as u8);
                frame.extend_from_slice(&data_bytes);
            }
            6 => {
                // FC06: Write Single Register
                frame.push((addr >> 8) as u8);
                frame.push((addr & 0xFF) as u8);
                frame.extend_from_slice(&data_bytes);
            }
            15 => {
                // FC15: Write Multiple Coils
                let quantity = config.data_length as u16;
                let byte_count = data_bytes.len() as u8;
                frame.push((addr >> 8) as u8);
                frame.push((addr & 0xFF) as u8);
                frame.push((quantity >> 8) as u8);
                frame.push((quantity & 0xFF) as u8);
                frame.push(byte_count);
                frame.extend_from_slice(&data_bytes);
            }
            16 => {
                // FC16: Write Multiple Registers
                let quantity = (data_bytes.len() / 2) as u16;
                let byte_count = data_bytes.len() as u8;
                frame.push((addr >> 8) as u8);
                frame.push((addr & 0xFF) as u8);
                frame.push((quantity >> 8) as u8);
                frame.push((quantity & 0xFF) as u8);
                frame.push(byte_count);
                frame.extend_from_slice(&data_bytes);
            }
            _ => {
                return Err(format!("Hex data mode not supported for FC{:02}", config.function_code).into());
            }
        }
        
        // Calculate and append CRC
        let crc = crc16_modbus(&frame);
        frame.push((crc & 0xFF) as u8);
        frame.push((crc >> 8) as u8);
        
        let mut port = tokio_serial::new(&serial_config.device, serial_config.baudrate)
            .data_bits(serial_config.databit)
            .stop_bits(serial_config.stopbit)
            .parity(serial_config.checkbit)
            .flow_control(serial_config.flowcontrol)
            .timeout(serial_config.timeout)
            .open_native_async()
            .map_err(|e| format!("Failed to open port: {}", e))?;
        
        AsyncWriteExt::write_all(&mut port, &frame).await?;
        
        // Read response
        let mut response_buf = vec![0u8; 256];
        match AsyncReadExt::read(&mut port, &mut response_buf).await {
            Ok(n) if n > 0 => {
                let response = &response_buf[..n];
                // logger.log(&format!("Response received: {:02X?}", response));
                drop(port);
                
                // Parse response according to function code (same format as standard frames)
                match config.function_code {
                    5 => {
                        // FC05: Write Single Coil response
                        if n >= 6 {
                            let value = ((response[4] as u16) << 8) | (response[5] as u16);
                            return Ok(format!("Coils: [0x{:04X}]", value));
                        }
                    }
                    6 => {
                        // FC06: Write Single Register response
                        if n >= 6 {
                            let value = ((response[4] as u16) << 8) | (response[5] as u16);
                            return Ok(format!("Registers: [0x{:04X}]", value));
                        }
                    }
                    15 => {
                        // FC15: Write Multiple Coils response
                        if n >= 6 {
                            let count = ((response[4] as u16) << 8) | (response[5] as u16);
                            return Ok(format!("Coils: [count={}]", count));
                        }
                    }
                    16 => {
                        // FC16: Write Multiple Registers response
                        if n >= 6 {
                            let count = ((response[4] as u16) << 8) | (response[5] as u16);
                            return Ok(format!("Registers: [count={}]", count));
                        }
                    }
                    _ => {
                        return Ok(format!("Response: {:02X?}", response));
                    }
                }
            }
            Ok(_) => {
                logger.log("No response data received");
                drop(port);
                return Err("No response data received".into());
            }
            Err(e) => {
                logger.log(&format!("Response read error: {}", e));
                drop(port);
                return Err(format!("Response read error: {}", e).into());
            }
        }
    }

    // Standard Modbus operations (for read operations or standard mode write operations)
    let addr = first_addr;
    match config.function_code {
        3 => {
            // Optimized: group consecutive addresses into single Modbus requests
            let groups = group_consecutive_addresses(&config.register_addresses);
            let mut addr_values: Vec<(u16, Vec<u16>)> = Vec::new();
            for (start_addr, addrs) in &groups {
                let total_count = (*addrs.last().unwrap() - start_addr) + config.data_length;
                let data = ctx.read_holding_registers(*start_addr, total_count).await??;
                for &a in addrs {
                    let offset = (a - start_addr) as usize;
                    let end = (offset + config.data_length as usize).min(data.len());
                    addr_values.push((a, data[offset..end].to_vec()));
                }
            }
            let mut entries: Vec<String> = Vec::new();
            for &a in &config.register_addresses {
                if let Some((_, values)) = addr_values.iter().find(|(k, _)| *k == a) {
                    let vals = values.iter().map(|v| format!("\"0x{:04X}\"", *v)).collect::<Vec<_>>().join(", ");
                    entries.push(format!("    \"{}\": [{}]", a, vals));
                }
            }
            Ok(format!("{{\n{}\n}}", entries.join(",\n")))
        }
        4 => {
            let groups = group_consecutive_addresses(&config.register_addresses);
            let mut addr_values: Vec<(u16, Vec<u16>)> = Vec::new();
            for (start_addr, addrs) in &groups {
                let total_count = (*addrs.last().unwrap() - start_addr) + config.data_length;
                let data = ctx.read_input_registers(*start_addr, total_count).await??;
                for &a in addrs {
                    let offset = (a - start_addr) as usize;
                    let end = (offset + config.data_length as usize).min(data.len());
                    addr_values.push((a, data[offset..end].to_vec()));
                }
            }
            let mut entries: Vec<String> = Vec::new();
            for &a in &config.register_addresses {
                if let Some((_, values)) = addr_values.iter().find(|(k, _)| *k == a) {
                    let vals = values.iter().map(|v| format!("\"0x{:04X}\"", *v)).collect::<Vec<_>>().join(", ");
                    entries.push(format!("    \"{}\": [{}]", a, vals));
                }
            }
            Ok(format!("{{\n{}\n}}", entries.join(",\n")))
        }
        1 => {
            let groups = group_consecutive_addresses(&config.register_addresses);
            let mut addr_values: Vec<(u16, Vec<bool>)> = Vec::new();
            for (start_addr, addrs) in &groups {
                let total_count = (*addrs.last().unwrap() - start_addr) + config.data_length;
                let data = ctx.read_coils(*start_addr, total_count).await??;
                for &a in addrs {
                    let offset = (a - start_addr) as usize;
                    let end = (offset + config.data_length as usize).min(data.len());
                    addr_values.push((a, data[offset..end].to_vec()));
                }
            }
            let mut entries: Vec<String> = Vec::new();
            for &a in &config.register_addresses {
                if let Some((_, values)) = addr_values.iter().find(|(k, _)| *k == a) {
                    let vals = values.iter().map(|v| if *v { "1" } else { "0" }).collect::<Vec<_>>().join(", ");
                    entries.push(format!("    \"{}\": [{}]", a, vals));
                }
            }
            Ok(format!("{{\n{}\n}}", entries.join(",\n")))
        }
        2 => {
            let groups = group_consecutive_addresses(&config.register_addresses);
            let mut addr_values: Vec<(u16, Vec<bool>)> = Vec::new();
            for (start_addr, addrs) in &groups {
                let total_count = (*addrs.last().unwrap() - start_addr) + config.data_length;
                let data = ctx.read_discrete_inputs(*start_addr, total_count).await??;
                for &a in addrs {
                    let offset = (a - start_addr) as usize;
                    let end = (offset + config.data_length as usize).min(data.len());
                    addr_values.push((a, data[offset..end].to_vec()));
                }
            }
            let mut entries: Vec<String> = Vec::new();
            for &a in &config.register_addresses {
                if let Some((_, values)) = addr_values.iter().find(|(k, _)| *k == a) {
                    let vals = values.iter().map(|v| if *v { "1" } else { "0" }).collect::<Vec<_>>().join(", ");
                    entries.push(format!("    \"{}\": [{}]", a, vals));
                }
            }
            Ok(format!("{{\n{}\n}}", entries.join(",\n")))
        }
        5 => {
            // Write Single Coil
            let value = config.write_value.trim().parse::<u16>().unwrap_or(0) != 0;
            ctx.write_single_coil(addr, value).await??;
            Ok(format!("{{\n    \"{}\": [{}]\n}}", addr, value))
        }
        6 => {
            // Write Single Register
            let value = if config.write_value.trim().starts_with("0x") || config.write_value.trim().starts_with("0X") {
                u16::from_str_radix(&config.write_value.trim()[2..], 16).unwrap_or(0)
            } else {
                config.write_value.trim().parse::<u16>().unwrap_or(0)
            };
            ctx.write_single_register(addr, value).await??;
            Ok(format!("{{\n    \"{}\": [\"0x{:04X}\"]\n}}", addr, value))
        }
        15 => {
            let values: Vec<bool> = config.write_value.trim().split(',')
                .filter_map(|s| s.trim().parse::<u16>().ok())
                .map(|v| v != 0)
                .collect();
            if values.is_empty() {
                return Err("No valid values provided for Write Multiple Coils".into());
            }
            ctx.write_multiple_coils(addr, &values).await??;
            Ok(format!("{{\n    \"{}\": [{}]\n}}", addr,
                values.iter().map(|v| if *v { "1" } else { "0" }).collect::<Vec<_>>().join(", ")))
        }
        16 => {
            // Write Multiple Registers
            let values: Vec<u16> = config.write_value.trim().split(',')
                .filter_map(|s| {
                    let s = s.trim();
                    if s.starts_with("0x") || s.starts_with("0X") {
                        u16::from_str_radix(&s[2..], 16).ok()
                    } else {
                        s.parse::<u16>().ok()
                    }
                })
                .collect();
            if values.is_empty() {
                return Err("No valid values provided for Write Multiple Registers".into());
            }
            ctx.write_multiple_registers(addr, &values).await??;
            Ok(format!("{{\n    \"{}\": [{}]\n}}", addr,
                values.iter().map(|v| format!("\"0x{:04X}\"", *v)).collect::<Vec<_>>().join(", ")))
        }
        _ => {
            Err(format!("Unsupported function code: {}", config.function_code).into())
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Parse port number from command line (default: 1)
    let port_num: u8 = std::env::args()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(1);

    let logger = Arc::new(Logger::new());
    logger.init()?;
    logger.log(&format!("RS485-Modbus Bridge starting for port {}...", port_num));

    // Load initial configuration from UCI
    let mut config = match load_config_from_uci(port_num) {
        Ok(cfg) => cfg,
        Err(e) => {
            logger.log(&format!("Failed to setup config: {}", e));
            return Err(e);
        }
    };

    // Initialize Modbus context
    let port = setup_serial(&config.serial).await?;
    logger.log(&format!(
        "Opening serial port: {} @ {} baud, {:?} data bits, {:?} stop bits, {:?} parity, {:?} flow control, {:?} timeout",
        config.serial.device, config.serial.baudrate, config.serial.databit, config.serial.stopbit, config.serial.checkbit, config.serial.flowcontrol, config.serial.timeout
    ));
    let mut modbus_ctx = rtu::attach(port);
    logger.log("Success opening serial port");
    
    let mut mqtt_client: Option<AsyncClient> = None;                // MQTT client
    let mut mqtt_eventloop: Option<rumqttc::EventLoop> = None;      // MQTT event loop
    let mut mqtt_state = "not_connect";                             // MQTT connection state   
    let mut last_periodic_read = tokio::time::Instant::now();       // Last periodic read timestamp

    // Build file paths with port suffix, e.g. /tmp/rs485/modbus_read_1
    let port_suffix = get_port_suffix(&config.serial.device);
    let trigger_read_owned = format!("/tmp/rs485/modbus_read{}", port_suffix);
    let trigger_write_owned = format!("/tmp/rs485/modbus_write{}", port_suffix);
    let result_owned = format!("/tmp/rs485/modbus_result{}", port_suffix);
    let trigger_read_path = trigger_read_owned.as_str();
    let trigger_write_path = trigger_write_owned.as_str();
    let result_path = result_owned.as_str();

    loop {
        // Load configuration
        config = match load_config_from_uci(port_num) {
            Ok(cfg) => cfg,
            Err(e) => {
                logger.log(&format!("Failed to load config: {}", e));
                return Err(e);
            }
        };

        if config.mqtt.enabled {
            if mqtt_state == "not_connect" {
                logger.log("MQTT enabled, connecting...");
                match setup_mqtt_client(&config.mqtt) {
                    Ok((client, el)) => {
                        mqtt_client = Some(client);
                        mqtt_eventloop = Some(el);
                        logger.log(&format!("Connected to {}:{}", config.mqtt.host, config.mqtt.port));
                        mqtt_state = "success_connect";
                    }
                    Err(e) => {
                        logger.log(&format!("Connection failed: {}", e));
                        mqtt_state = "failed_connect";
                    }
                }
            } 
            else if mqtt_state == "success_connect" {
                // Handle work mode based logic
                if config.protocol.work_mode == "once" {
                        // Check for modbus_read trigger file first (higher priority)
                    if Path::new(trigger_read_path).exists() {
                        // logger.log("Modbus read trigger detected");
                        
                        // Add 3 second timeout for Modbus read
                        let read_future = read_modbus_data(&mut modbus_ctx, &config.protocol, &config.serial, &logger);
                        let timeout_future = tokio::time::sleep(Duration::from_millis(config.protocol.timeout * 100));
                        
                        let modbus_result = tokio::select! {
                            result = read_future => Some(result),
                            _ = timeout_future => {
                                logger.log(&format!("Modbus read timeout after {}ms", config.protocol.timeout * 100));
                                None
                            }
                        };
                        
                        match modbus_result {
                            Some(Ok(data)) => {
                                // logger.log(&format!("Modbus data: {}", data));
                                match std::fs::write(result_path, &data) {
                                    Ok(_) => logger.log(&format!("Modbus data received: {}", data)),
                                    Err(e) => logger.log(&format!("Failed to write result file: {}", e)),
                                }
                                
                                // Publish to MQTT if enabled
                                if config.mqtt.enabled {
                                    if let Some(ref client) = mqtt_client {
                                        let json = build_modbus_uplink(&config.protocol, &data, None);
                                        match client.publish(&config.mqtt.uplink_topic, config.mqtt.qos_level, false, json.as_bytes()).await {
                                            Ok(_) => logger.log(&format!("Published to MQTT: {}", json)),
                                            Err(e) => logger.log(&format!("MQTT publish failed: {}", e)),
                                        }
                                    }
                                }
                            }
                            Some(Err(e)) => {
                                logger.log(&format!("Modbus read failed: {}", e));
                                let error_msg = format!("Error: {}", e);
                                match std::fs::write(result_path, &error_msg) {
                                    Ok(_) => logger.log("Error result written"),
                                    Err(e2) => logger.log(&format!("Failed to write error: {}", e2)),
                                }
                                // Publish error to MQTT
                                if config.mqtt.enabled {
                                    if let Some(ref client) = mqtt_client {
                                        let json = build_modbus_error_uplink(&config.protocol, &error_msg);
                                        let _ = client.publish(&config.mqtt.uplink_topic, config.mqtt.qos_level, false, json.as_bytes()).await;
                                    }
                                }
                            }
                            None => {
                                // Timeout occurred
                                let error_msg = "Error: Modbus read timeout";
                                match std::fs::write(result_path, &error_msg) {
                                    Ok(_) => logger.log("Error result written"),
                                    Err(e) => logger.log(&format!("Failed to write error: {}", e)),
                                }
                                // Publish timeout error to MQTT
                                if config.mqtt.enabled {
                                    if let Some(ref client) = mqtt_client {
                                        let json = build_modbus_error_uplink(&config.protocol, error_msg);
                                        let _ = client.publish(&config.mqtt.uplink_topic, config.mqtt.qos_level, false, json.as_bytes()).await;
                                    }
                                }
                            }
                        }
                    
                        let _ = std::fs::remove_file(trigger_read_path);
                    }
                } 
                else if config.protocol.work_mode == "periodic" && matches!(config.protocol.function_code, 1 | 2 | 3 | 4) {
                    // Periodic mode: Read at intervals
                    let elapsed = last_periodic_read.elapsed();
                    if elapsed >= Duration::from_secs(config.protocol.poll_interval) {
                        last_periodic_read = tokio::time::Instant::now();
                        
                        let read_future = read_modbus_data(&mut modbus_ctx, &config.protocol, &config.serial, &logger);
                        let timeout_duration = Duration::from_millis(config.protocol.timeout * 100);
                        
                        let modbus_result = tokio::select! {
                            result = read_future => Some(result),
                            _ = tokio::time::sleep(timeout_duration) => {
                                logger.log(&format!("Modbus read timeout after {}ms", config.protocol.timeout * 100));
                                None
                            }
                        };
                        
                        match modbus_result {
                            Some(Ok(data)) => {
                                // logger.log(&format!("Modbus data: {}", data));
                                match std::fs::write(result_path, &data) {
                                    Ok(_) => logger.log(&format!("Modbus data received: {}", data)),
                                    Err(e) => logger.log(&format!("Failed to write result file: {}", e)),
                                }
                                
                                // Publish to MQTT if enabled
                                if config.mqtt.enabled {
                                    if let Some(ref client) = mqtt_client {
                                        let json = build_modbus_uplink(&config.protocol, &data, None);
                                        match client.publish(&config.mqtt.uplink_topic, config.mqtt.qos_level, false, json.as_bytes()).await {
                                            Ok(_) => logger.log(&format!("Published to MQTT: {}", json)),
                                            Err(e) => logger.log(&format!("MQTT publish failed: {}", e)),
                                        }
                                    }
                                }
                            }
                            Some(Err(e)) => {
                                logger.log(&format!("Modbus read failed: {}", e));
                                let error_msg = format!("Error: {}", e);
                                match std::fs::write(result_path, &error_msg) {
                                    Ok(_) => logger.log("Error result written"),
                                    Err(e2) => logger.log(&format!("Failed to write error: {}", e2)),
                                }
                                // Publish error to MQTT
                                if config.mqtt.enabled {
                                    if let Some(ref client) = mqtt_client {
                                        let json = build_modbus_error_uplink(&config.protocol, &error_msg);
                                        let _ = client.publish(&config.mqtt.uplink_topic, config.mqtt.qos_level, false, json.as_bytes()).await;
                                    }
                                }
                            }
                            None => {
                                // Timeout occurred
                                let error_msg = "Error: Modbus read timeout";
                                match std::fs::write(result_path, &error_msg) {
                                    Ok(_) => logger.log("Error result written"),
                                    Err(e) => logger.log(&format!("Failed to write error: {}", e)),
                                }
                                // Publish timeout error to MQTT
                                if config.mqtt.enabled {
                                    if let Some(ref client) = mqtt_client {
                                        let json = build_modbus_error_uplink(&config.protocol, error_msg);
                                        let _ = client.publish(&config.mqtt.uplink_topic, config.mqtt.qos_level, false, json.as_bytes()).await;
                                    }
                                }
                            }
                        }
                    }
                }
                
                // Then handle MQTT events with timeout
                tokio::select! {
                    mqtt_result = async {
                        if let Some(ref mut el) = mqtt_eventloop {
                            el.poll().await
                        } else {
                            std::future::pending().await
                        }
                    } => {
                        match mqtt_result {
                            Ok(Event::Incoming(incoming)) => {
                                match incoming {
                                    // Handle connection acknowledgment
                                    Incoming::ConnAck(_) => {
                                        // Protocol mode: MQTT is uplink-only (RS485 -> Modbus -> MQTT)
                                        // No downlink subscription needed
                                        logger.log(&format!("MQTT connected, uplink topic: {}", config.mqtt.uplink_topic));
                                    }
                                    // Handle disconnection
                                    Incoming::Disconnect => {
                                        logger.log("MQTT disconnected");
                                        mqtt_state = "failed_connect";
                                    }
                                    // Handle other incoming events
                                    _ => {
                                        // logger.log(&format!("MQTT event: {:?}", other));
                                    }
                                }
                            }
                            Ok(Event::Outgoing(_)) => {
                                // Outgoing events are normal
                            }
                            Err(e) => {
                                logger.log(&format!("MQTT error: {}", e));
                                mqtt_state = "failed_connect";
                            }
                        }
                    }
                    _ = tokio::time::sleep(Duration::from_millis(1000)) => {
                        // Timeout to ensure trigger file checked regularly
                    }
                }
            } 
            else if mqtt_state == "failed_connect" {
                tokio::select! {
                    // Reconnect timer
                    _ = tokio::time::sleep(Duration::from_secs(config.mqtt.reconnect_delay)) => {
                        logger.log("Retrying connection...");
                        match setup_mqtt_client(&config.mqtt) {
                            Ok((client, el)) => {
                                mqtt_client = Some(client);
                                mqtt_eventloop = Some(el);

                                logger.log(&format!("Success connecting to {}:{}", config.mqtt.host, config.mqtt.port));
                                mqtt_state = "success_connect";
                            }
                            Err(e) => {
                                mqtt_client = None;
                                mqtt_eventloop = None;

                                logger.log(&format!("Reconnect failed: {}", e));
                                mqtt_state = "failed_connect";
                            }
                        }
                    }
                }
            }
        } 
        else {
            // MQTT disabled, only check trigger file
            if config.protocol.work_mode == "once" {
                if Path::new(trigger_read_path).exists() {
                    // logger.log("Modbus read trigger detected");
                    
                    // Add 3 second timeout for Modbus read
                    let read_future = read_modbus_data(&mut modbus_ctx, &config.protocol, &config.serial, &logger);
                    let timeout_future = tokio::time::sleep(Duration::from_millis(config.protocol.timeout * 100));
                    
                    let modbus_result = tokio::select! {
                        result = read_future => Some(result),
                        _ = timeout_future => {
                            logger.log(&format!("Modbus read timeout after {}ms", config.protocol.timeout * 100));
                            None
                        }
                    };

                    match modbus_result {
                        Some(Ok(data)) => {
                            // logger.log(&format!("Modbus data: {}", data));
                            match std::fs::write(result_path, &data) {
                                Ok(_) => logger.log(&format!("Modbus data received: {}", data)),
                                Err(e) => logger.log(&format!("Failed to write result file: {}", e)),
                            }
                        }
                        Some(Err(e)) => {
                            logger.log(&format!("Modbus read failed: {}", e));
                            let error_msg = format!("Error: {}", e);
                            match std::fs::write(result_path, &error_msg) {
                                Ok(_) => logger.log("Error result written"),
                                Err(e) => logger.log(&format!("Failed to write error: {}", e)),
                            }
                        }
                        None => {
                            // Timeout occurred
                            let error_msg = "Error: Modbus read timeout";
                            match std::fs::write(result_path, &error_msg) {
                                Ok(_) => logger.log("Error result written"),
                                Err(e) => logger.log(&format!("Failed to write error: {}", e)),
                            }
                        }
                    }
                        
                    let _ = std::fs::remove_file(trigger_read_path);
                }
            } 
            else if config.protocol.work_mode == "periodic" && matches!(config.protocol.function_code, 1 | 2 | 3 | 4) {
                // Periodic mode: Read at intervals
                let elapsed = last_periodic_read.elapsed();
                if elapsed >= Duration::from_secs(config.protocol.poll_interval) {
                    last_periodic_read = tokio::time::Instant::now();
                    
                    let read_future = read_modbus_data(&mut modbus_ctx, &config.protocol, &config.serial, &logger);
                    let timeout_duration = Duration::from_millis(config.protocol.timeout * 100);
                    
                    let modbus_result = tokio::select! {
                        result = read_future => Some(result),
                        _ = tokio::time::sleep(timeout_duration) => {
                            logger.log(&format!("Modbus read timeout after {}ms", config.protocol.timeout * 100));
                            None
                        }
                    };
                    
                    match modbus_result {
                        Some(Ok(data)) => {
                            // logger.log(&format!("Modbus data: {}", data));
                            match std::fs::write(result_path, &data) {
                                Ok(_) => logger.log(&format!("Modbus data received: {}", data)),
                                Err(e) => logger.log(&format!("Failed to write result file: {}", e)),
                            }
                        }
                        Some(Err(e)) => {
                            logger.log(&format!("Modbus read failed: {}", e));
                            let error_msg = format!("Error: {}", e);
                            match std::fs::write(result_path, &error_msg) {
                                Ok(_) => logger.log("Error result written"),
                                Err(e) => logger.log(&format!("Failed to write error: {}", e)),
                            }
                        }
                        None => {
                            // Timeout occurred
                            let error_msg = "Error: Modbus read timeout";
                            match std::fs::write(result_path, &error_msg) {
                                Ok(_) => logger.log("Error result written"),
                                Err(e) => logger.log(&format!("Failed to write error: {}", e)),
                            }
                        }
                    }
                }
            }
            
            if mqtt_state != "not_connect" {
                mqtt_client = None;
                mqtt_eventloop = None;
                logger.log("MQTT disabled");
                mqtt_state = "not_connect";
            }
        }

        // Check for modbus_write trigger file (works regardless of MQTT state)
        if Path::new(trigger_write_path).exists() {
            let write_future = read_modbus_data(&mut modbus_ctx, &config.protocol, &config.serial, &logger);
            let timeout_future = tokio::time::sleep(Duration::from_secs(3));
            
            let modbus_result = tokio::select! {
                result = write_future => Some(result),
                _ = timeout_future => {
                    logger.log("Modbus write operation timeout after 3 seconds");
                    None
                }
            };
            
            match modbus_result {
                Some(Ok(data)) => {
                    logger.log(&format!("Modbus write successful: {}", data));
                    if let Err(e) = std::fs::write(result_path, &data) {
                        logger.log(&format!("Failed to write result file: {}", e));
                    }
                }
                Some(Err(e)) => {
                    let error_msg = format!("Error: Modbus write failed - {}", e);
                    logger.log(&error_msg);
                    let _ = std::fs::write(result_path, error_msg);
                }
                None => {
                    let error_msg = "Error: Modbus write timeout";
                    logger.log(error_msg);
                    let _ = std::fs::write(result_path, error_msg);
                }
            }
            
            // Remove trigger file
            let _ = std::fs::remove_file(trigger_write_path);
        }

        sleep(Duration::from_millis(100)).await;
    }
}
