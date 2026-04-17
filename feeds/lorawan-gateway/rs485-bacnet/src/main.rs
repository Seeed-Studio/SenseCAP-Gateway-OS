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
use tokio::process::Command as TokioCommand;
use tokio::time::sleep;
use std::path::Path;

// Configuration Structures
#[derive(Debug, Clone)]
struct Config {
    mqtt: MqttConfig,
    serial: SerialConfig,
    bacnet: BacnetConfig,
}

#[derive(Debug, Clone)]
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
    qos_level: QoS,
    reconnect_delay: u64,
    auth_mode: String,
    ca_cert: Option<String>,
    client_cert: Option<String>,
    client_key: Option<String>,
    token: Option<String>,
}

#[derive(Debug, Clone)]
struct SerialConfig {
    device: String,
    baudrate: u32,
}

#[derive(Debug, Clone)]
struct BacnetConfig {
    mac_address: u8,
    max_master: u8,
    max_info_frames: u8,
    device_instance: u32,
    device_name: String,
    poll_interval: u64,
}

// MQTT Message Structures
#[derive(Debug, Serialize)]
struct UplinkMessage {
    device: DeviceInfo,
    data: BacnetData,
    #[serde(skip_serializing_if = "Option::is_none")]
    error_code: Option<String>,
}

#[derive(Debug, Serialize)]
struct DeviceInfo {
    sn: String,
    mac: String,
}

#[derive(Debug, Serialize)]
struct BacnetData {
    protocol: String,
    direction: String,
    local_mac: u8,
    local_instance: u32,
    devices_discovered: usize,
    devices: serde_json::Value,
}

/// Read device info from /etc/deviceinfo/ and /sys/class/net/eth0/address
fn get_device_info() -> DeviceInfo {
    let sn = fs::read_to_string("/etc/deviceinfo/sn")
        .unwrap_or_default().trim().to_string();
    let mac = fs::read_to_string("/sys/class/net/eth0/address")
        .unwrap_or_default().trim().to_string();
    DeviceInfo { sn, mac }
}

/// Build a structured MQTT uplink message for BACnet MS/TP
fn build_bacnet_uplink(
    config: &BacnetConfig,
    devices_data: &str,
    device_count: usize,
    error: Option<String>,
) -> String {
    let devices: serde_json::Value = serde_json::from_str(devices_data)
        .unwrap_or_else(|_| json!(devices_data));

    let msg = UplinkMessage {
        device: get_device_info(),
        data: BacnetData {
            protocol: "BACnet MS/TP".to_string(),
            direction: "response".to_string(),
            local_mac: config.mac_address,
            local_instance: config.device_instance,
            devices_discovered: device_count,
            devices,
        },
        error_code: error,
    };

    serde_json::to_string(&msg).unwrap_or_else(|_| "{{\"error\":\"serialization failed\"}}".to_string())
}

/// Build a structured MQTT uplink error message for BACnet MS/TP
fn build_bacnet_error_uplink(
    config: &BacnetConfig,
    error_msg: &str,
) -> String {
    let msg = UplinkMessage {
        device: get_device_info(),
        data: BacnetData {
            protocol: "BACnet MS/TP".to_string(),
            direction: "error".to_string(),
            local_mac: config.mac_address,
            local_instance: config.device_instance,
            devices_discovered: 0,
            devices: json!(null),
        },
        error_code: Some(error_msg.to_string()),
    };

    serde_json::to_string(&msg).unwrap_or_else(|_| "{{\"error\":\"serialization failed\"}}".to_string())
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
        let log_line = format!("[{}][RS485-BACnet]: {}\n", timestamp, message);
        print!("{}", log_line);
        if let Ok(mut file_guard) = self.file.lock() {
            if let Some(ref mut file) = *file_guard {
                let _ = file.write_all(log_line.as_bytes());
                let _ = file.flush();
            }
        }
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
    let client_id = uci_get("rs485-module", &sid, "mqtt_client_id")
        .unwrap_or_else(|_| format!("rs485_bacnet_{}", port_num));
    let keepalive = uci_get("rs485-module", &sid, "mqtt_keepalive")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(30);
    let uplink_topic = uci_get("rs485-module", &sid, "mqtt_uplink_topic")
        .unwrap_or_else(|_| format!("rs485/CH{}/uplink", port_num));
    let qos_level = uci_get("rs485-module", &sid, "mqtt_qos")
        .ok()
        .and_then(|s| s.parse::<u8>().ok())
        .unwrap_or(0);
    let reconnect_delay = uci_get("rs485-module", &sid, "mqtt_reconnect_delay")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(30);
    let transport = uci_get("rs485-module", &sid, "mqtt_transport")
        .unwrap_or_else(|_| "tcp".to_string());
    let auth_mode = uci_get("rs485-module", &sid, "mqtt_auth_mode")
        .unwrap_or_else(|_| "none".to_string());
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

    // Serial config — device derived from port number
    let device = format!("/dev/RS485-{}", port_num);
    let baudrate = uci_get("rs485-module", &sid, "baudrate")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(9600);

    let serial_config = SerialConfig { device, baudrate };

    // BACnet config — read from portN section with bacnet_ prefix
    let mac_address = uci_get("rs485-module", &sid, "bacnet_mac_address")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(1);
    let max_master = uci_get("rs485-module", &sid, "bacnet_max_master")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(127);
    let max_info_frames = uci_get("rs485-module", &sid, "bacnet_max_info_frames")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(1);
    let device_instance = uci_get("rs485-module", &sid, "bacnet_device_instance")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(1234);
    let device_name = uci_get("rs485-module", &sid, "bacnet_device_name")
        .unwrap_or_else(|_| "SenseCAP Gateway".to_string());
    let poll_interval = uci_get("rs485-module", &sid, "bacnet_poll_interval")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(10);

    let bacnet_config = BacnetConfig {
        mac_address,
        max_master,
        max_info_frames,
        device_instance,
        device_name,
        poll_interval,
    };

    Ok(Config {
        mqtt: mqtt_config,
        serial: serial_config,
        bacnet: bacnet_config,
    })
}

// Setup MQTT client (uplink only, no downlink subscription)
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
                        root_cert_store
                            .extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
                    }

                    let tls_config = RustlsClientConfig::builder()
                        .with_root_certificates(root_cert_store)
                        .with_no_client_auth();

                    mqttoptions.set_transport(Transport::tls_with_config(
                        rumqttc::TlsConfiguration::Rustls(Arc::new(tls_config)),
                    ));
                }
                "mutual-tls" => {
                    if let Some(ca_cert) = &config.ca_cert {
                        let ca_cert_bytes = ca_cert.as_bytes();
                        let mut cursor = std::io::Cursor::new(ca_cert_bytes);
                        for cert in certs(&mut cursor) {
                            root_cert_store.add(cert?)?;
                        }
                    } else {
                        root_cert_store
                            .extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
                    }

                    if let (Some(client_cert), Some(client_key)) =
                        (&config.client_cert, &config.client_key)
                    {
                        let cert_bytes = client_cert.as_bytes();
                        let mut cert_cursor = std::io::Cursor::new(cert_bytes);
                        let client_certs: Vec<_> = certs(&mut cert_cursor).collect::<Result<Vec<_>, _>>()?;

                        let key_bytes = client_key.as_bytes();
                        let mut key_cursor = std::io::Cursor::new(key_bytes);
                        let client_keys: Vec<_> = pkcs8_private_keys(&mut key_cursor).collect::<Result<Vec<_>, _>>()?;

                        if let Some(key) = client_keys.into_iter().next() {
                            let tls_config = RustlsClientConfig::builder()
                                .with_root_certificates(root_cert_store)
                                .with_client_auth_cert(
                                    client_certs,
                                    rumqttc::tokio_rustls::rustls::pki_types::PrivateKeyDer::Pkcs8(key),
                                )?;

                            mqttoptions.set_transport(Transport::tls_with_config(
                                rumqttc::TlsConfiguration::Rustls(Arc::new(tls_config)),
                            ));
                        }
                    }
                }
                _ => {
                    root_cert_store
                        .extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
                    let tls_config = RustlsClientConfig::builder()
                        .with_root_certificates(root_cert_store)
                        .with_no_client_auth();
                    mqttoptions.set_transport(Transport::tls_with_config(
                        rumqttc::TlsConfiguration::Rustls(Arc::new(tls_config)),
                    ));
                }
            }
        }
        "ws" => {
            mqttoptions.set_transport(Transport::Ws);
        }
        "wss" => {
            let mut root_cert_store = rumqttc::tokio_rustls::rustls::RootCertStore::empty();
            root_cert_store.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
            let tls_config = RustlsClientConfig::builder()
                .with_root_certificates(root_cert_store)
                .with_no_client_auth();
            mqttoptions.set_transport(Transport::wss_with_config(
                rumqttc::TlsConfiguration::Rustls(Arc::new(tls_config)),
            ));
        }
        _ => {
            // TCP, no TLS
        }
    }

    let (client, eventloop) = AsyncClient::new(mqttoptions, 10);
    Ok((client, eventloop))
}

/// Run bacwhois to discover BACnet devices on the MS/TP network.
/// Returns a list of device instance IDs found.
async fn discover_devices(config: &Config, logger: &Logger) -> Vec<u32> {
    logger.log("Running BACnet Who-Is discovery...");

    let result = tokio::time::timeout(
        Duration::from_secs(10),
        TokioCommand::new("/usr/bin/bacwhois")
            .env("BACNET_IFACE", &config.serial.device)
            .env("BACNET_MSTP_BAUD", config.serial.baudrate.to_string())
            .env("BACNET_MSTP_MAC", config.bacnet.mac_address.to_string())
            .env("BACNET_MAX_MASTER", config.bacnet.max_master.to_string())
            .output(),
    )
    .await;

    match result {
        Ok(Ok(output)) => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let mut devices = Vec::new();

            // bacwhois output format: "device-instance;network;mac;max-apdu;segmentation;vendor-id"
            for line in stdout.lines() {
                let line = line.trim();
                if line.is_empty() {
                    continue;
                }
                // Parse device instance from first field
                if let Some(instance_str) = line.split(';').next() {
                    if let Ok(instance) = instance_str.trim().parse::<u32>() {
                        devices.push(instance);
                    }
                }
            }

            logger.log(&format!("Discovered {} BACnet device(s): {:?}", devices.len(), devices));
            devices
        }
        Ok(Err(e)) => {
            logger.log(&format!("bacwhois failed: {}", e));
            Vec::new()
        }
        Err(_) => {
            logger.log("bacwhois timed out after 10 seconds");
            Vec::new()
        }
    }
}

/// Read all properties from a BACnet device using bacepics.
/// Returns the raw output as a string to be forwarded via MQTT.
async fn read_device_epics(config: &Config, device_instance: u32, logger: &Logger) -> Option<String> {
    let result = tokio::time::timeout(
        Duration::from_secs(30),
        TokioCommand::new("/usr/bin/bacepics")
            .arg(device_instance.to_string())
            .env("BACNET_IFACE", &config.serial.device)
            .env("BACNET_MSTP_BAUD", config.serial.baudrate.to_string())
            .env("BACNET_MSTP_MAC", config.bacnet.mac_address.to_string())
            .env("BACNET_MAX_MASTER", config.bacnet.max_master.to_string())
            .output(),
    )
    .await;

    match result {
        Ok(Ok(output)) => {
            if output.status.success() {
                let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                if !stdout.trim().is_empty() {
                    Some(stdout)
                } else {
                    logger.log(&format!("bacepics returned empty for device {}", device_instance));
                    None
                }
            } else {
                let stderr = String::from_utf8_lossy(&output.stderr);
                logger.log(&format!("bacepics error for device {}: {}", device_instance, stderr));
                None
            }
        }
        Ok(Err(e)) => {
            logger.log(&format!("bacepics execution failed: {}", e));
            None
        }
        Err(_) => {
            logger.log(&format!("bacepics timed out for device {}", device_instance));
            None
        }
    }
}

/// Read a single property from a BACnet device using bacrp.
/// Returns the value as a string.
async fn read_property(
    config: &Config,
    device_instance: u32,
    object_type: u32,
    object_instance: u32,
    property_id: u32,
    logger: &Logger,
) -> Option<String> {
    let result = tokio::time::timeout(
        Duration::from_secs(10),
        TokioCommand::new("/usr/bin/bacrp")
            .arg(device_instance.to_string())
            .arg(object_type.to_string())
            .arg(object_instance.to_string())
            .arg(property_id.to_string())
            .env("BACNET_IFACE", &config.serial.device)
            .env("BACNET_MSTP_BAUD", config.serial.baudrate.to_string())
            .env("BACNET_MSTP_MAC", config.bacnet.mac_address.to_string())
            .env("BACNET_MAX_MASTER", config.bacnet.max_master.to_string())
            .output(),
    )
    .await;

    match result {
        Ok(Ok(output)) => {
            if output.status.success() {
                let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
                if !stdout.is_empty() {
                    Some(stdout)
                } else {
                    None
                }
            } else {
                let stderr = String::from_utf8_lossy(&output.stderr);
                logger.log(&format!("bacrp error: {}", stderr));
                None
            }
        }
        Ok(Err(e)) => {
            logger.log(&format!("bacrp execution failed: {}", e));
            None
        }
        Err(_) => {
            logger.log("bacrp timed out");
            None
        }
    }
}

/// Collect BACnet data from all discovered devices and build a JSON report.
async fn collect_bacnet_data(config: &Config, logger: &Logger) -> Option<String> {
    let devices = discover_devices(config, logger).await;

    if devices.is_empty() {
        logger.log("No BACnet devices found on the network");
        return None;
    }

    let mut report = serde_json::Map::new();

    for device_id in &devices {
        match read_device_epics(config, *device_id, logger).await {
            Some(data) => {
                report.insert(
                    device_id.to_string(),
                    serde_json::Value::String(data),
                );
            }
            None => {
                logger.log(&format!("No data from device {}", device_id));
            }
        }
    }

    if report.is_empty() {
        return None;
    }

    match serde_json::to_string(&report) {
        Ok(json) => Some(json),
        Err(e) => {
            logger.log(&format!("JSON serialization failed: {}", e));
            None
        }
    }
}

#[tokio::main]
async fn main() {
    let logger = Logger::new();
    if let Err(e) = logger.init() {
        eprintln!("Failed to initialize logger: {}", e);
        return;
    }

    logger.log("Starting RS485-BACnet service");

    // Parse port number from command line (default: 1)
    let port_num: u8 = std::env::args()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(1);

    // Load configuration
    let config = match load_config_from_uci(port_num) {
        Ok(c) => c,
        Err(e) => {
            logger.log(&format!("Failed to load config: {}", e));
            return;
        }
    };

    logger.log(&format!("Config loaded: device={}, baud={}, BACnet MAC={}, instance={}",
        config.serial.device, config.serial.baudrate,
        config.bacnet.mac_address, config.bacnet.device_instance));

    // No bacserv needed - the gateway acts as a BACnet client.
    // bacwhois/bacrp CLI tools handle MS/TP datalink internally.

    // Setup MQTT client (uplink only)
    let mut mqtt_client: Option<AsyncClient> = None;
    let mut mqtt_eventloop: Option<rumqttc::EventLoop> = None;
    let mut mqtt_state = "not_connect";

    if config.mqtt.enabled && !config.mqtt.host.is_empty() {
        match setup_mqtt_client(&config.mqtt) {
            Ok((client, el)) => {
                mqtt_client = Some(client);
                mqtt_eventloop = Some(el);
                mqtt_state = "success_connect";
                logger.log(&format!("MQTT connecting to {}:{}", config.mqtt.host, config.mqtt.port));
            }
            Err(e) => {
                logger.log(&format!("MQTT setup failed: {}", e));
                mqtt_state = "failed_connect";
            }
        }
    }

    let mut last_poll = tokio::time::Instant::now()
        - Duration::from_secs(config.bacnet.poll_interval);
    let trigger_read_owned = format!("/tmp/rs485/bacnet_read_{}", port_num);
    let result_owned = format!("/tmp/rs485/bacnet_result_{}", port_num);
    let trigger_read_path = trigger_read_owned.as_str();
    let result_path = result_owned.as_str();

    // Main event loop
    loop {
        // Handle trigger-file based reads (for LuCI UI)
        if Path::new(trigger_read_path).exists() {
            logger.log("BACnet read trigger detected");
            let _ = std::fs::remove_file(trigger_read_path);

            match collect_bacnet_data(&config, &logger).await {
                Some(data) => {
                    logger.log(&format!("BACnet data collected: {}", data));
                    if let Err(e) = std::fs::write(result_path, &data) {
                        logger.log(&format!("Failed to write result: {}", e));
                    }

                    // Publish to MQTT if connected
                    if let Some(ref client) = mqtt_client {
                        let device_count = serde_json::from_str::<serde_json::Value>(&data)
                            .ok()
                            .and_then(|v| v.as_object().map(|o| o.len()))
                            .unwrap_or(0);
                        let json = build_bacnet_uplink(&config.bacnet, &data, device_count, None);
                        match client
                            .publish(
                                &config.mqtt.uplink_topic,
                                config.mqtt.qos_level,
                                false,
                                json.as_bytes(),
                            )
                            .await
                        {
                            Ok(_) => logger.log(&format!("Published to MQTT: {}", config.mqtt.uplink_topic)),
                            Err(e) => logger.log(&format!("MQTT publish failed: {}", e)),
                        }
                    }
                }
                None => {
                    let error_msg = "Error: No BACnet data collected";
                    let _ = std::fs::write(result_path, error_msg);
                    logger.log(error_msg);

                    // Publish error to MQTT if connected
                    if let Some(ref client) = mqtt_client {
                        let json = build_bacnet_error_uplink(&config.bacnet, error_msg);
                        let _ = client.publish(
                            &config.mqtt.uplink_topic,
                            config.mqtt.qos_level,
                            false,
                            json.as_bytes(),
                        ).await;
                    }
                }
            }
        }

        // Periodic polling
        let elapsed = last_poll.elapsed();
        if elapsed >= Duration::from_secs(config.bacnet.poll_interval) {
            last_poll = tokio::time::Instant::now();

            if let Some(data) = collect_bacnet_data(&config, &logger).await {
                // Write to result file
                let _ = std::fs::write(result_path, &data);

                // Publish to MQTT if connected
                if let Some(ref client) = mqtt_client {
                    let device_count = serde_json::from_str::<serde_json::Value>(&data)
                        .ok()
                        .and_then(|v| v.as_object().map(|o| o.len()))
                        .unwrap_or(0);
                    let json = build_bacnet_uplink(&config.bacnet, &data, device_count, None);
                    match client
                        .publish(
                            &config.mqtt.uplink_topic,
                            config.mqtt.qos_level,
                            false,
                            json.as_bytes(),
                        )
                        .await
                    {
                        Ok(_) => logger.log(&format!("Published to MQTT: {}", config.mqtt.uplink_topic)),
                        Err(e) => logger.log(&format!("MQTT publish failed: {}", e)),
                    }
                }
            }
        }

        // Handle MQTT events
        if mqtt_state == "success_connect" {
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
                                Incoming::ConnAck(_) => {
                                    logger.log(&format!("MQTT connected, uplink topic: {}", config.mqtt.uplink_topic));
                                }
                                Incoming::Disconnect => {
                                    logger.log("MQTT disconnected");
                                    mqtt_state = "failed_connect";
                                }
                                _ => {}
                            }
                        }
                        Ok(Event::Outgoing(_)) => {}
                        Err(e) => {
                            logger.log(&format!("MQTT error: {}", e));
                            mqtt_state = "failed_connect";
                        }
                    }
                }
                _ = tokio::time::sleep(Duration::from_millis(1000)) => {
                    // Timeout to ensure polling cycle continues
                }
            }
        } else if mqtt_state == "failed_connect" {
            sleep(Duration::from_secs(config.mqtt.reconnect_delay)).await;
            logger.log("Retrying MQTT connection...");
            match setup_mqtt_client(&config.mqtt) {
                Ok((client, el)) => {
                    mqtt_client = Some(client);
                    mqtt_eventloop = Some(el);
                    logger.log(&format!("MQTT reconnected to {}:{}", config.mqtt.host, config.mqtt.port));
                    mqtt_state = "success_connect";
                }
                Err(e) => {
                    mqtt_client = None;
                    mqtt_eventloop = None;
                    logger.log(&format!("MQTT reconnect failed: {}", e));
                }
            }
        } else {
            // MQTT disabled, just sleep briefly
            sleep(Duration::from_millis(100)).await;
        }

        sleep(Duration::from_millis(100)).await;
    }
}
