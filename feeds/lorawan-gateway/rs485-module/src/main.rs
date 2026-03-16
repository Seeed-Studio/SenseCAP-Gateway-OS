use chrono::Local;
use rumqttc::{AsyncClient, Event, Incoming, MqttOptions, QoS, Transport};
use rumqttc::tokio_rustls::rustls::ClientConfig as RustlsClientConfig;
use rustls_pemfile::{certs, pkcs8_private_keys};
use serde::{Deserialize, Serialize};
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::process::Command;
use std::sync::{Arc, Mutex as StdMutex};
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::time::sleep;
use tokio_serial::{DataBits, Parity, StopBits, SerialPortBuilderExt};

/// Initialize timezone from UCI system configuration
fn init_timezone() {
    match Command::new("uci")
        .args(&["get", "system.@system[0].timezone"])
        .output()
    {
        Ok(output) if output.status.success() => {
            if let Ok(tz) = String::from_utf8(output.stdout) {
                let tz = tz.trim();
                if !tz.is_empty() && tz != "UTC" {
                    std::env::set_var("TZ", tz);
                    // Notify that timezone was loaded
                    eprintln!("Timezone set from UCI: {}", tz);
                }
            }
        }
        _ => {
            // Fallback to UTC if UCI read fails
            std::env::set_var("TZ", "UTC");
        }
    }
}

// Mqtt and Serial Configuration Structures
#[derive(Debug, Clone, PartialEq)]
struct Config {
    mqtt: MqttConfig,
    serial: SerialConfig,
}

// MQTT Configuration Structure
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

// Serial Configuration Structure
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

// RS485 -> MQTT Message Structure
#[derive(Debug, Serialize)]
struct UplinkMessage {
    data: String,
}

// MQTT -> RS485 Message Structure
#[derive(Debug, Deserialize)]
struct DownlinkMessage {
    data: String,
}

// Logger Structure
struct Logger {
    file: StdMutex<Option<File>>,
}

// Logger Implementation
impl Logger {
    // Create a new Logger instance
    fn new() -> Self {
        Logger {
            file: StdMutex::new(None),
        }
    }

    // Initialize the logger by creating log directory and file
    fn init(&self) -> std::io::Result<()> {
        // Create log directory if it doesn't exist
        std::fs::create_dir_all("/tmp/rs485")?;

        // Open log file in append mode
        let file = OpenOptions::new()
            .create(true)   // Create the file if it doesn't exist
            .append(true)   // Append to the file
            .open("/tmp/rs485/log")?;

        // Acquire lock and set initialized file
        let mut file_guard = self.file.lock().unwrap();
        *file_guard = Some(file);
        Ok(())
    }

    fn log(&self, message: &str) {
        // Get current timestamp
        let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S");
        // Format log line
        let log_line = format!("[{}][RS485-MQTT]: {}\n", timestamp, message);
        print!("{}", log_line);

        // Write log line to file if initialized
        if let Ok(mut file_guard) = self.file.lock() {
            if let Some(ref mut file) = *file_guard {
                let _ = file.write_all(log_line.as_bytes());
                let _ = file.flush();
            }
        }
    }
}

// Load configuration from UCI
fn load_config_from_uci() -> Result<Config, Box<dyn std::error::Error + Send + Sync>> {
    // uci get helper function
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

    // MQTT config
    let enabled = uci_get("rs485-module", "mqtt", "enabled")
        .ok()
        .and_then(|s| s.parse::<u8>().ok())
        .unwrap_or(0)
        == 1;   
    let host = uci_get("rs485-module", "mqtt", "host").unwrap_or_default();
    let port = uci_get("rs485-module", "mqtt", "port")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(1883);
    let username = uci_get("rs485-module", "mqtt", "username").ok();
    let password = uci_get("rs485-module", "mqtt", "password").ok();
    let client_id = uci_get("rs485-module", "mqtt", "client_id").unwrap_or_else(|_| "rs485_bridge".to_string());
    let keepalive = uci_get("rs485-module", "mqtt", "keepalive")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(30);
    let uplink_topic = uci_get("rs485-module", "mqtt", "uplink_topic").unwrap_or_else(|_| "rs485/uplink".to_string());
    let downlink_topic = uci_get("rs485-module", "mqtt", "downlink_topic").unwrap_or_else(|_| "rs485/downlink".to_string());
    let qos_level = uci_get("rs485-module", "mqtt", "qos")
        .ok()
        .and_then(|s| s.parse::<u8>().ok())
        .unwrap_or(0);
    let reconnect_delay = uci_get("rs485-module", "mqtt", "reconnect_delay")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(30);

    let transport = uci_get("rs485-module", "mqtt", "transport").unwrap_or_else(|_| "tcp".to_string());
    let auth_mode = uci_get("rs485-module", "mqtt", "auth_mode").unwrap_or_else(|_| "none".to_string());
    let ca_cert = uci_get("rs485-module", "mqtt", "ca_cert").ok();
    let client_cert = uci_get("rs485-module", "mqtt", "client_cert").ok();
    let client_key = uci_get("rs485-module", "mqtt", "client_key").ok();
    let token = uci_get("rs485-module", "mqtt", "token").ok();

    let mqtt_config = MqttConfig {
        enabled,
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

    // Serial config
    let device = format!(
        "/dev/{}",
        uci_get("rs485-module", "serial", "device").unwrap_or_else(|_| "RS485-1".to_string())
    );
    let baudrate = uci_get("rs485-module", "serial", "baudrate")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(9600);
    let databit = uci_get("rs485-module", "serial", "databit")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(8);
    let stopbit = uci_get("rs485-module", "serial", "stopbit").unwrap_or_else(|_| "1".to_string());
    let checkbit = uci_get("rs485-module", "serial", "checkbit").unwrap_or_else(|_| "none".to_string());
    let flowcontrol = uci_get("rs485-module", "serial", "flowcontrol").unwrap_or_else(|_| "none".to_string());
    let timeout = uci_get("rs485-module", "serial", "timeout")
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

    Ok(Config {
        mqtt: mqtt_config,
        serial: serial_config,
    })
}

// Setup MQTT client
fn setup_mqtt_client(
    config: &MqttConfig,
) -> Result<(AsyncClient, rumqttc::EventLoop), Box<dyn std::error::Error + Send + Sync>> {
    // Create MQTT options
    let mut mqttoptions = MqttOptions::new(&config.client_id, &config.host, config.port);

    // Set keep alive interval
    mqttoptions.set_keep_alive(Duration::from_secs(config.keepalive));
    
    // Set basic credentials if provided
    if let (Some(username), Some(password)) = (&config.username, &config.password) {
        mqttoptions.set_credentials(username, password);
    }

    // Configure transport based on protocol
    match config.transport.as_str() {
        "ssl" | "tls" => {
            // TLS/SSL configuration
            let mut root_cert_store = rumqttc::tokio_rustls::rustls::RootCertStore::empty();
            
            // Configure based on auth mode
            match config.auth_mode.as_str() {
                "tls-server" => {
                    // Server verification only - load CA cert
                    if let Some(ca_cert) = &config.ca_cert {
                        let ca_cert_bytes = ca_cert.as_bytes();
                        let mut cursor = std::io::Cursor::new(ca_cert_bytes);
                        for cert in certs(&mut cursor) {
                            root_cert_store.add(cert?)?;
                        }
                    } else {
                        // Use system CA certificates
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
                    // Mutual TLS - load CA, client cert and key
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
                    
                    // Load client certificate and private key
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
                    // Default TLS with system certs
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
            // WebSocket - construct URL format
            let ws_url = format!("ws://{}:{}/mqtt", config.host, config.port);
            mqttoptions = MqttOptions::new(&config.client_id, &ws_url, config.port);
            mqttoptions.set_keep_alive(Duration::from_secs(config.keepalive));
            if let Some(username) = &config.username {
                mqttoptions.set_credentials(username, config.password.as_deref().unwrap_or(""));
            }
            mqttoptions.set_transport(Transport::Ws);
        }
        "wss" => {
            // WebSocket Secure - construct URL format
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
            
            // Check auth mode for WSS
            let tls_config = match config.auth_mode.as_str() {
                "mutual-tls" => {
                    // Mutual TLS for WebSocket Secure
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
                    // Default: server verification only
                    RustlsClientConfig::builder()
                        .with_root_certificates(root_cert_store)
                        .with_no_client_auth()
                }
            };
            
            mqttoptions.set_transport(Transport::wss_with_config(tls_config.into()));
        }
        "tcp" | _ => {
            // Plain TCP (default)
            mqttoptions.set_transport(Transport::Tcp);
        }
    }
    
    // Create AsyncClient and EventLoop
    let (client, eventloop) = AsyncClient::new(mqttoptions, 10);
    Ok((client, eventloop))
}

// Configure serial port settings
async fn setup_serial(
    config: &SerialConfig,
) -> Result<tokio_serial::SerialStream, Box<dyn std::error::Error + Send + Sync>> {

    // Open serial port with specified settings
    let port = tokio_serial::new(&config.device, config.baudrate)
        .data_bits(config.databit)
        .stop_bits(config.stopbit)
        .parity(config.checkbit)
        .flow_control(config.flowcontrol)
        .timeout(config.timeout)
        .open_native_async()
        .map_err(|e| {
            e
        })?;
    
    Ok(port)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Initialize timezone from UCI system configuration
    init_timezone();

    // Initialize logger
    let logger = Arc::new(Logger::new());
    logger.init()?;
    logger.log("RS485-MQTT Bridge starting...");

    // Load initial configuration from UCI
    let mut config = match load_config_from_uci() {
        Ok(cfg) => cfg,
        Err(e) => {
            logger.log(&format!("Failed to setup config: {}", e));
            return Err(e);
        }
    };

    // Initialize serial port
    let mut serial_port = match setup_serial(&config.serial).await {
        Ok(port) => {
            logger.log(&format!(
                "Opening serial port: {} @ {} baud, {:?} data bits, {:?} stop bits, {:?} parity, {:?} flow control, {:?} timeout",
                config.serial.device, config.serial.baudrate, config.serial.databit, config.serial.stopbit, config.serial.checkbit, config.serial.flowcontrol, config.serial.timeout
            ));
            logger.log("Success opening serial port");
            port
        }
        Err(e) => {
            logger.log(&format!("Failed opening serial port: {}", e));
            return Err(e);
        }
    };

    let mut mqtt_client: Option<AsyncClient> = None;                // MQTT client
    let mut mqtt_eventloop: Option<rumqttc::EventLoop> = None;      // MQTT event loop
    let mut mqtt_state = "not_connect";                             // MQTT connection state   

    loop {
            // Load configuration
            config = match load_config_from_uci() {
                Ok(cfg) => cfg,
                Err(e) => {
                    logger.log(&format!("Failed to load config: {}", e));
                    return Err(e);
                }
            };

            if config.mqtt.enabled {
                if mqtt_state == "not_connect" {
                    logger.log("MQTT enabled, start connection...");
                    match setup_mqtt_client(&config.mqtt) {
                        Ok((client, el)) => {
                            mqtt_client = Some(client);
                            mqtt_eventloop = Some(el);

                            logger.log(&format!("Success connecting to {}:{}", config.mqtt.host, config.mqtt.port));
                            mqtt_state = "success_connect";
                        }
                        Err(e) => {
                            logger.log(&format!("Failed connecting: {}", e));
                            mqtt_state = "failed_connect";
                        }
                    }
                }
                else if mqtt_state == "success_connect" {
                    tokio::select! {
                        // Handle MQTT events
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
                                            // Subscribe to downlink topic
                                            if let Some(ref client) = mqtt_client {
                                                match client.subscribe(&config.mqtt.downlink_topic, config.mqtt.qos_level).await {
                                                    Ok(_) => {
                                                        logger.log(&format!("Subscribed [MQTT->RS485] to topic: {}", config.mqtt.downlink_topic));
                                                    }
                                                    Err(e) => {
                                                        logger.log(&format!("Failed to topic: {}", e));
                                                    }
                                                }
                                            }
                                            logger.log(&format!("Published [RS485->MQTT] to topic: {}", config.mqtt.uplink_topic));
                                        }
                                        // Handle incoming publish messages
                                        Incoming::Publish(p) => {
                                            let payload = String::from_utf8_lossy(&p.payload);
                                            logger.log(&format!("MQTT received: {}", payload));
                                            
                                            if let Ok(msg) = serde_json::from_str::<DownlinkMessage>(&payload) {
                                                let data = msg.data.as_bytes();
                                                match AsyncWriteExt::write_all(&mut serial_port, data).await {
                                                    Ok(_) => {
                                                        logger.log(&format!("Forwarded to RS485: {}", msg.data));
                                                    }
                                                    Err(e) => {
                                                        logger.log(&format!("RS485 write failed: {}", e));
                                                    }
                                                }
                                            } else {
                                                logger.log(&format!("Invalid message format, expected {{\"data\":\"...\"}}, got: {}", payload));
                                            }
                                        }
                                        // Handle subscription acknowledgment
                                        Incoming::SubAck(_) => {
                                            // logger.log("Subscription acknowledged");
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

                        serial_result = async { 
                            let mut serial_buffer = vec![0u8; 1024];
                            serial_port.read(&mut serial_buffer).await.map(|n| (n, serial_buffer))
                        } => {
                            match serial_result {
                                Ok((n, buffer)) if n > 0 => {
                                    let data_str = String::from_utf8_lossy(&buffer[..n]).trim().to_string();
                                    logger.log(&format!("RS485 received: {}", data_str));
                                    
                                    if let Some(ref client) = mqtt_client {
                                        let uplink_msg = UplinkMessage { data: data_str.clone() };
                                        match serde_json::to_string(&uplink_msg) {
                                            Ok(json) => {
                                                match client.publish(&config.mqtt.uplink_topic, config.mqtt.qos_level, false, json.as_bytes()).await {
                                                    Ok(_) => {
                                                        logger.log(&format!("Published to MQTT: {}", json));
                                                    }
                                                    Err(e) => {
                                                        logger.log(&format!("MQTT publish failed: {}", e));
                                                    }
                                                }
                                            }
                                            Err(e) => {
                                                logger.log(&format!("JSON serialization failed: {}", e));
                                            }
                                        }
                                    }
                                }
                                Err(e) if e.kind() != std::io::ErrorKind::WouldBlock => {
                                    logger.log(&format!("Serial read error: {}", e));
                                }
                                _ => {}
                            }
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
                if mqtt_state != "not_connect" {
                    mqtt_client = None;
                    mqtt_eventloop = None;

                    logger.log("MQTT disabled");
                    mqtt_state = "not_connect";
                }
            }

        sleep(Duration::from_millis(1)).await;
    }
}
