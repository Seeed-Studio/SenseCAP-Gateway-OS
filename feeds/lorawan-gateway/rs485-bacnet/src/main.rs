use chrono::Local;
use rumqttc::{AsyncClient, Event, Incoming, MqttOptions, QoS, Transport};
use rumqttc::tokio_rustls::rustls::ClientConfig as RustlsClientConfig;
use rustls_pemfile::{certs, pkcs8_private_keys};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::BTreeSet;
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::process::Command;
use std::sync::{Arc, Mutex as StdMutex};
use std::time::{Duration, Instant};
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

const DEFAULT_BACNET_LOCAL_MAC: u8 = 10;
const DEFAULT_BACNET_APDU_TIMEOUT_MS: u64 = 15000;
const DEFAULT_BACNET_DISCOVERY_TIMEOUT_MS: u64 = 5000;
const DEFAULT_BACNET_DISCOVERY_DELAY_MS: u64 = 1000;
const BACNET_DEVICE_CACHE_TTL_SECS: u64 = 10 * 60;
const MIN_MQTT_KEEPALIVE_SECS: u64 = 120;
const MQTT_PUBLISH_FLUSH_WINDOW_MS: u64 = 1000;
const MQTT_MAX_SAFE_PAYLOAD_BYTES: usize = 9 * 1024;
const DEVICE_OBJECT_TYPE: &str = "8";
const PROP_OBJECT_NAME: &str = "77";

#[derive(Debug, Default)]
struct BacnetRuntimeState {
    summary_only_devices: BTreeSet<u32>,
    cached_devices: Vec<u32>,
    cached_devices_at: Option<Instant>,
}

#[derive(Debug, Clone, Default, Deserialize)]
struct BacnetReadRequest {
    #[serde(default)]
    target_device_instance: Option<u32>,
    #[serde(default)]
    object_type: Option<String>,
    #[serde(default)]
    object_instance: Option<u32>,
    #[serde(default)]
    property: Option<String>,
    #[serde(default)]
    array_index: Option<u32>,
}

impl BacnetReadRequest {
    fn normalized(mut self) -> Self {
        self.object_type = self.object_type.take().and_then(normalize_request_token);
        self.property = self.property.take().and_then(normalize_request_token);
        self
    }

    fn has_any_fields(&self) -> bool {
        self.target_device_instance.is_some()
            || self.object_type.is_some()
            || self.object_instance.is_some()
            || self.property.is_some()
            || self.array_index.is_some()
    }

    fn has_property_request(&self) -> bool {
        self.object_type.is_some()
            || self.object_instance.is_some()
            || self.property.is_some()
            || self.array_index.is_some()
    }

    fn validate(&self) -> Result<(), String> {
        if self.has_property_request()
            && (self.object_type.is_none()
                || self.object_instance.is_none()
                || self.property.is_none())
        {
            return Err(
                "targeted BACnet reads require object_type, object_instance, and property"
                    .to_string(),
            );
        }

        Ok(())
    }

    fn describe(&self) -> String {
        if self.has_property_request() {
            let mut description = format!(
                "device={:?}, object_type={}, object_instance={}, property={}",
                self.target_device_instance,
                self.object_type.as_deref().unwrap_or(""),
                self.object_instance.unwrap_or_default(),
                self.property.as_deref().unwrap_or(""),
            );

            if let Some(array_index) = self.array_index {
                description.push_str(&format!(", array_index={}", array_index));
            }

            description
        } else if let Some(device_instance) = self.target_device_instance {
            format!("device={}", device_instance)
        } else {
            "summary".to_string()
        }
    }
}

#[derive(Debug, Clone, Default, Deserialize)]
struct BacnetWriteRequest {
    #[serde(default)]
    target_device_instance: Option<u32>,
    #[serde(default)]
    object_type: Option<String>,
    #[serde(default)]
    object_instance: Option<u32>,
    #[serde(default)]
    property: Option<String>,
    #[serde(default)]
    priority: Option<u8>,
    #[serde(default)]
    array_index: Option<i32>,
    #[serde(default)]
    tag: Option<String>,
    #[serde(default)]
    value: Option<String>,
}

impl BacnetWriteRequest {
    fn normalized(mut self) -> Self {
        self.object_type = self.object_type.take().and_then(normalize_request_token);
        self.property = self.property.take().and_then(normalize_request_token);
        self.tag = self.tag.take().and_then(normalize_request_token);
        self.value = self.value.take().and_then(normalize_request_token);
        self
    }

    fn validate(&self) -> Result<(), String> {
        if self.target_device_instance.is_none()
            || self.object_type.is_none()
            || self.object_instance.is_none()
            || self.property.is_none()
            || self.tag.is_none()
            || self.value.is_none()
        {
            return Err(
                "BACnet writes require target_device_instance, object_type, object_instance, property, tag, and value"
                    .to_string(),
            );
        }

        if let Some(priority) = self.priority {
            if priority > 16 {
                return Err("BACnet write priority must be between 0 and 16".to_string());
            }
        }

        if let Some(array_index) = self.array_index {
            if array_index < -1 {
                return Err("BACnet write array_index must be -1 or greater".to_string());
            }
        }

        let tag = self.tag.as_deref().unwrap_or_default();
        if tag.parse::<u8>().map_or(true, |value| value > 14) {
            return Err("BACnet write tag must be a BACnet application tag number from 0 to 14".to_string());
        }

        Ok(())
    }

    fn priority(&self) -> u8 {
        self.priority.unwrap_or(16)
    }

    fn write_array_index(&self) -> i32 {
        self.array_index.unwrap_or(-1)
    }

    fn read_array_index(&self) -> Option<u32> {
        self.write_array_index().try_into().ok()
    }

    fn describe(&self) -> String {
        format!(
            "device={}, object_type={}, object_instance={}, property={}, priority={}, array_index={}, tag={}, value={}",
            self.target_device_instance.unwrap_or_default(),
            self.object_type.as_deref().unwrap_or(""),
            self.object_instance.unwrap_or_default(),
            self.property.as_deref().unwrap_or(""),
            self.priority(),
            self.write_array_index(),
            self.tag.as_deref().unwrap_or(""),
            self.value.as_deref().unwrap_or("")
        )
    }
}

fn normalize_request_token(value: String) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

impl BacnetRuntimeState {
    fn cache_devices(&mut self, devices: &[u32]) {
        self.cached_devices = devices.to_vec();
        self.cached_devices_at = Some(Instant::now());
    }

    fn cached_devices(&self) -> Option<Vec<u32>> {
        let cached_at = self.cached_devices_at?;
        if self.cached_devices.is_empty() {
            return None;
        }
        if cached_at.elapsed() > Duration::from_secs(BACNET_DEVICE_CACHE_TTL_SECS) {
            return None;
        }

        Some(self.cached_devices.clone())
    }

    fn last_known_devices(&self) -> Option<Vec<u32>> {
        if self.cached_devices.is_empty() {
            None
        } else {
            Some(self.cached_devices.clone())
        }
    }

    fn mark_summary_only(&mut self, device_instance: u32) {
        self.summary_only_devices.insert(device_instance);
    }

    fn prefers_summary_only(&self, device_instance: u32) -> bool {
        self.summary_only_devices.contains(&device_instance)
    }
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

fn configure_bacnet_command<'a>(
    command: &'a mut TokioCommand,
    config: &Config,
) -> &'a mut TokioCommand {
    command
        .env("BACNET_DATALINK", "mstp")
        .env("BACNET_IFACE", &config.serial.device)
        .env("BACNET_MSTP_BAUD", config.serial.baudrate.to_string())
        .env("BACNET_MSTP_MAC", config.bacnet.mac_address.to_string())
        .env("BACNET_MAX_MASTER", config.bacnet.max_master.to_string())
        .env(
            "BACNET_MAX_INFO_FRAMES",
            config.bacnet.max_info_frames.to_string(),
        )
        .env(
            "BACNET_APDU_TIMEOUT",
            DEFAULT_BACNET_APDU_TIMEOUT_MS.to_string(),
        )
}

fn parse_discovered_devices(stdout: &str) -> Vec<u32> {
    let mut devices = BTreeSet::new();

    for raw_line in stdout.lines() {
        let line = raw_line.trim();

        if line.is_empty() || line.starts_with(';') {
            continue;
        }

        let parsed = if line.contains(';') {
            line.split(';')
                .next()
                .and_then(|token| token.trim().parse::<u32>().ok())
        } else {
            line.split_whitespace()
                .next()
                .and_then(|token| token.parse::<u32>().ok())
        };

        if let Some(instance) = parsed {
            devices.insert(instance);
        }
    }

    devices.into_iter().collect()
}

fn is_bacnet_cli_noise_line(line: &str) -> bool {
    line.starts_with("MS/TP Interface:")
        || line.starts_with("RS485 Interface:")
        || line.starts_with("RS485 Baud Rate")
        || line.starts_with("MS/TP MAC:")
        || line.starts_with("MS/TP Max_Master:")
        || line.starts_with("MS/TP Max_Info_Frames:")
        || line.starts_with("MS/TP RxBuf[")
        || line.starts_with("MS/TP SlaveModeEnabled:")
        || line.starts_with("MS/TP ZeroConfigEnabled:")
        || line.starts_with("MS/TP CheckAutoBaud:")
        || line.starts_with("DLMSTP:")
}

fn sanitize_bacnet_cli_output(stdout: &str) -> String {
    stdout
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .filter(|line| !is_bacnet_cli_noise_line(line))
        .collect::<Vec<_>>()
        .join("\n")
}

fn load_trigger_read_request(
    request_path: &str,
    logger: &Logger,
) -> Result<Option<BacnetReadRequest>, String> {
    if !Path::new(request_path).exists() {
        return Ok(None);
    }

    let request_contents = fs::read_to_string(request_path)
        .map_err(|e| format!("failed to read BACnet request: {}", e))?;
    let _ = fs::remove_file(request_path);

    if request_contents.trim().is_empty() {
        return Ok(None);
    }

    let request = serde_json::from_str::<BacnetReadRequest>(&request_contents)
        .map_err(|e| format!("invalid BACnet request JSON: {}", e))?
        .normalized();

    if !request.has_any_fields() {
        return Ok(None);
    }

    request.validate()?;
    logger.log(&format!(
        "Received BACnet manual read request: {}",
        request.describe()
    ));

    Ok(Some(request))
}

fn load_trigger_write_request(
    request_path: &str,
    logger: &Logger,
) -> Result<BacnetWriteRequest, String> {
    if !Path::new(request_path).exists() {
        return Err("missing BACnet write request".to_string());
    }

    let request_contents = fs::read_to_string(request_path)
        .map_err(|e| format!("failed to read BACnet write request: {}", e))?;
    let _ = fs::remove_file(request_path);

    if request_contents.trim().is_empty() {
        return Err("empty BACnet write request".to_string());
    }

    let request = serde_json::from_str::<BacnetWriteRequest>(&request_contents)
        .map_err(|e| format!("invalid BACnet write request JSON: {}", e))?
        .normalized();

    request.validate()?;
    logger.log(&format!(
        "Received BACnet manual write request: {}",
        request.describe()
    ));

    Ok(request)
}

async fn flush_mqtt_after_publish(
    mqtt_eventloop: Option<&mut rumqttc::EventLoop>,
    logger: &Logger,
    topic: &str,
) -> bool {
    let Some(eventloop) = mqtt_eventloop else {
        return true;
    };

    let deadline = tokio::time::Instant::now()
        + Duration::from_millis(MQTT_PUBLISH_FLUSH_WINDOW_MS);

    while tokio::time::Instant::now() < deadline {
        match tokio::time::timeout(Duration::from_millis(200), eventloop.poll()).await {
            Ok(Ok(Event::Incoming(Incoming::ConnAck(_)))) => {
                logger.log(&format!("MQTT connected, uplink topic: {}", topic));
            }
            Ok(Ok(Event::Incoming(Incoming::Disconnect))) => {
                logger.log("MQTT disconnected");
                return false;
            }
            Ok(Ok(Event::Outgoing(_))) => {
                return true;
            }
            Ok(Ok(_)) => {}
            Ok(Err(e)) => {
                logger.log(&format!("MQTT error: {}", e));
                return false;
            }
            Err(_) => {
                break;
            }
        }
    }

    true
}

async fn publish_mqtt_payload(
    client: &AsyncClient,
    mqtt_eventloop: Option<&mut rumqttc::EventLoop>,
    topic: &str,
    qos: QoS,
    payload: &str,
    logger: &Logger,
) -> bool {
    match client.publish(topic, qos, false, payload.as_bytes()).await {
        Ok(_) => {
            if flush_mqtt_after_publish(mqtt_eventloop, logger, topic).await {
                logger.log(&format!("Published to MQTT: {}", topic));
                true
            } else {
                false
            }
        }
        Err(e) => {
            logger.log(&format!("MQTT publish failed: {}", e));
            false
        }
    }
}

fn limit_bacnet_mqtt_payload(
    config: &BacnetConfig,
    payload: String,
    logger: &Logger,
) -> String {
    let payload_size = payload.as_bytes().len();
    if payload_size <= MQTT_MAX_SAFE_PAYLOAD_BYTES {
        return payload;
    }

    let error_msg = format!(
        "BACnet MQTT payload too large: {} bytes exceeds safe limit {} bytes; full result kept locally only",
        payload_size, MQTT_MAX_SAFE_PAYLOAD_BYTES
    );
    logger.log(&error_msg);
    build_bacnet_error_uplink(config, &error_msg)
}

fn describe_bacnet_data_for_log(data: &str) -> String {
    let size = data.as_bytes().len();
    let device_count = serde_json::from_str::<serde_json::Value>(data)
        .ok()
        .and_then(|v| v.as_object().map(|o| o.len()))
        .unwrap_or(0);

    format!("{} bytes, {} device(s)", size, device_count)
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
        .unwrap_or(30)
        .max(MIN_MQTT_KEEPALIVE_SECS);
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
        .unwrap_or(DEFAULT_BACNET_LOCAL_MAC);
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
/// Returns the discovered device instance IDs.
async fn discover_devices(config: &Config, logger: &Logger) -> Vec<u32> {
    logger.log("Running BACnet Who-Is discovery...");

    if config.bacnet.mac_address == 1 {
        logger.log(
            "Warning: local BACnet MAC is set to 1. MS/TP MAC addresses must be unique on the bus, and field devices commonly use 1.",
        );
    }

    let mut command = TokioCommand::new("/usr/bin/bacwhois");
    configure_bacnet_command(&mut command, config)
        .arg("--timeout")
        .arg(DEFAULT_BACNET_DISCOVERY_TIMEOUT_MS.to_string())
        .arg("--delay")
        .arg(DEFAULT_BACNET_DISCOVERY_DELAY_MS.to_string());

    let result = tokio::time::timeout(
        Duration::from_secs(20),
        command.output(),
    )
    .await;

    match result {
        Ok(Ok(output)) => {
            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
                logger.log(&format!("bacwhois exited with status {}: {}", output.status, stderr));
                return Vec::new();
            }

            let stdout = String::from_utf8_lossy(&output.stdout);
            let devices = parse_discovered_devices(&stdout);

            if devices.is_empty() && !stdout.trim().is_empty() {
                let flattened = stdout
                    .lines()
                    .map(|line| line.trim())
                    .filter(|line| !line.is_empty())
                    .collect::<Vec<_>>()
                    .join(" | ");
                logger.log(&format!(
                    "bacwhois returned output, but no device instances were parsed: {}",
                    flattened
                ));
            }

            logger.log(&format!(
                "Discovered {} BACnet device(s): {:?}",
                devices.len(),
                devices
            ));
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
    let mut command = TokioCommand::new("/usr/bin/bacepics");
    configure_bacnet_command(&mut command, config)
        .arg(device_instance.to_string());

    let result = tokio::time::timeout(
        Duration::from_secs(45),
        command.output(),
    )
    .await;

    match result {
        Ok(Ok(output)) => {
            if output.status.success() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let sanitized = sanitize_bacnet_cli_output(&stdout);
                if !sanitized.is_empty() {
                    Some(sanitized)
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
    object_type: &str,
    object_instance: u32,
    property: &str,
    array_index: Option<u32>,
    logger: &Logger,
) -> Option<String> {
    let mut command = TokioCommand::new("/usr/bin/bacrp");
    configure_bacnet_command(&mut command, config)
        .arg(device_instance.to_string())
        .arg(object_type)
        .arg(object_instance.to_string())
        .arg(property);

    if let Some(array_index) = array_index {
        command.arg(array_index.to_string());
    }

    let result = tokio::time::timeout(
        Duration::from_secs(15),
        command.output(),
    )
    .await;

    match result {
        Ok(Ok(output)) => {
            if output.status.success() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let sanitized = sanitize_bacnet_cli_output(&stdout);
                if !sanitized.is_empty() {
                    Some(sanitized)
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

async fn write_property(
    config: &Config,
    request: &BacnetWriteRequest,
    logger: &Logger,
) -> Result<String, String> {
    let device_instance = request.target_device_instance.unwrap_or_default();
    let object_type = request.object_type.as_deref().unwrap_or_default();
    let object_instance = request.object_instance.unwrap_or_default();
    let property = request.property.as_deref().unwrap_or_default();
    let tag = request.tag.as_deref().unwrap_or_default();
    let value = request.value.as_deref().unwrap_or_default();

    let mut command = TokioCommand::new("/usr/bin/bacwp");
    configure_bacnet_command(&mut command, config)
        .arg(device_instance.to_string())
        .arg(object_type)
        .arg(object_instance.to_string())
        .arg(property)
        .arg(request.priority().to_string())
        .arg(request.write_array_index().to_string())
        .arg(tag)
        .arg(value);

    let result = tokio::time::timeout(
        Duration::from_secs(20),
        command.output(),
    )
    .await;

    match result {
        Ok(Ok(output)) => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let stderr = String::from_utf8_lossy(&output.stderr);
            let stdout = sanitize_bacnet_cli_output(&stdout);
            let stderr = sanitize_bacnet_cli_output(&stderr);

            if output.status.success() {
                if stdout.is_empty() {
                    Ok("OK".to_string())
                } else {
                    Ok(stdout)
                }
            } else {
                let details = if stderr.is_empty() { stdout } else { stderr };
                logger.log(&format!("bacwp error: {}", details));
                Err(if details.is_empty() {
                    "bacwp failed".to_string()
                } else {
                    details
                })
            }
        }
        Ok(Err(e)) => {
            logger.log(&format!("bacwp execution failed: {}", e));
            Err(format!("bacwp execution failed: {}", e))
        }
        Err(_) => {
            logger.log("bacwp timed out");
            Err("bacwp timed out".to_string())
        }
    }
}

async fn read_device_summary(
    config: &Config,
    device_instance: u32,
    logger: &Logger,
) -> Option<String> {
    if let Some(object_name) = read_property(
        config,
        device_instance,
        DEVICE_OBJECT_TYPE,
        device_instance,
        PROP_OBJECT_NAME,
        None,
        logger,
    )
    .await
    {
        return Some(format!(
            "object-identifier: (device, {})\nobject-name: {}",
            device_instance, object_name
        ));
    }

    None
}

async fn collect_requested_bacnet_data(
    config: &Config,
    logger: &Logger,
    runtime_state: &mut BacnetRuntimeState,
    request: &BacnetReadRequest,
) -> Option<String> {
    let (devices, used_cached_devices) = if let Some(device_instance) = request.target_device_instance {
        logger.log(&format!(
            "Running targeted BACnet read for remote device {}",
            device_instance
        ));
        (vec![device_instance], false)
    } else {
        get_target_devices(config, logger, runtime_state).await
    };

    if devices.is_empty() {
        logger.log("No BACnet devices available for the requested read");
        return None;
    }

    let mut report = serde_json::Map::new();

    for device_id in &devices {
        let data = if request.has_property_request() {
            match read_property(
                config,
                *device_id,
                request.object_type.as_deref().unwrap_or_default(),
                request.object_instance.unwrap_or_default(),
                request.property.as_deref().unwrap_or_default(),
                request.array_index,
                logger,
            )
            .await
            {
                Some(value) => Some(serde_json::json!({
                    "object_type": request.object_type.as_deref().unwrap_or_default(),
                    "object_instance": request.object_instance.unwrap_or_default(),
                    "property": request.property.as_deref().unwrap_or_default(),
                    "array_index": request.array_index,
                    "value": value,
                })),
                None => None,
            }
        } else {
            read_device_summary(config, *device_id, logger)
                .await
                .map(serde_json::Value::String)
        };

        match data {
            Some(value) => {
                report.insert(device_id.to_string(), value);
                if request.target_device_instance.is_some() {
                    runtime_state.cache_devices(&[*device_id]);
                }
            }
            None => {
                logger.log(&format!("No requested data from device {}", device_id));
            }
        }
    }

    if report.is_empty() {
        if used_cached_devices {
            logger.log("Cached BACnet device list produced no targeted data. Keeping cache for discovery fallback.");
        }
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

async fn collect_requested_bacnet_write(
    config: &Config,
    logger: &Logger,
    runtime_state: &mut BacnetRuntimeState,
    request: &BacnetWriteRequest,
) -> Option<String> {
    let device_instance = request.target_device_instance.unwrap_or_default();
    logger.log(&format!(
        "Running BACnet write for remote device {}",
        device_instance
    ));

    let write_output = match write_property(config, request, logger).await {
        Ok(output) => output,
        Err(e) => {
            logger.log(&format!("BACnet write failed for device {}: {}", device_instance, e));
            return None;
        }
    };

    runtime_state.cache_devices(&[device_instance]);

    let readback_value = read_property(
        config,
        device_instance,
        request.object_type.as_deref().unwrap_or_default(),
        request.object_instance.unwrap_or_default(),
        request.property.as_deref().unwrap_or_default(),
        request.read_array_index(),
        logger,
    )
    .await;

    if readback_value.is_none() {
        logger.log(&format!(
            "BACnet write succeeded for device {}, but readback failed",
            device_instance
        ));
    }

    let mut report = serde_json::Map::new();
    report.insert(
        device_instance.to_string(),
        serde_json::json!({
            "operation": "write",
            "object_type": request.object_type.as_deref().unwrap_or_default(),
            "object_instance": request.object_instance.unwrap_or_default(),
            "property": request.property.as_deref().unwrap_or_default(),
            "priority": request.priority(),
            "array_index": request.write_array_index(),
            "tag": request.tag.as_deref().unwrap_or_default(),
            "value": request.value.as_deref().unwrap_or_default(),
            "write_output": write_output,
            "readback_value": readback_value,
        }),
    );

    match serde_json::to_string(&report) {
        Ok(json) => Some(json),
        Err(e) => {
            logger.log(&format!("JSON serialization failed: {}", e));
            None
        }
    }
}

async fn get_target_devices(
    config: &Config,
    logger: &Logger,
    runtime_state: &mut BacnetRuntimeState,
) -> (Vec<u32>, bool) {
    if let Some(cached_devices) = runtime_state.cached_devices() {
        logger.log(&format!(
            "Using cached BACnet device list: {:?}",
            cached_devices
        ));
        return (cached_devices, true);
    }

    let devices = discover_devices(config, logger).await;
    if !devices.is_empty() {
        runtime_state.cache_devices(&devices);
        return (devices, false);
    }

    if let Some(last_known_devices) = runtime_state.last_known_devices() {
        logger.log(&format!(
            "Discovery returned no devices. Falling back to last known BACnet device list: {:?}",
            last_known_devices
        ));
        return (last_known_devices, true);
    }

    (Vec::new(), false)
}

/// Collect BACnet data from all discovered devices and build a JSON report.
async fn collect_bacnet_data(
    config: &Config,
    logger: &Logger,
    runtime_state: &mut BacnetRuntimeState,
) -> Option<String> {
    let (devices, used_cached_devices) = get_target_devices(config, logger, runtime_state).await;

    if devices.is_empty() {
        logger.log("No BACnet devices found on the network");
        return None;
    }

    let mut report = serde_json::Map::new();

    for device_id in &devices {
        if runtime_state.prefers_summary_only(*device_id) {
            logger.log(&format!(
                "Skipping bacepics for device {} because it previously timed out.",
                device_id
            ));

            match read_device_summary(config, *device_id, logger).await {
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

            continue;
        }

        match read_device_epics(config, *device_id, logger).await {
            Some(data) => {
                if data.as_bytes().len() > MQTT_MAX_SAFE_PAYLOAD_BYTES {
                    runtime_state.mark_summary_only(*device_id);
                    logger.log(&format!(
                        "bacepics data for device {} is {} bytes. Future periodic polls will use summary reads.",
                        device_id,
                        data.as_bytes().len()
                    ));
                }

                report.insert(
                    device_id.to_string(),
                    serde_json::Value::String(data),
                );
            }
            None => {
                runtime_state.mark_summary_only(*device_id);
                logger.log(&format!(
                    "bacepics returned no data for device {}. Falling back to targeted ReadProperty requests.",
                    device_id
                ));

                match read_device_summary(config, *device_id, logger).await {
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
        }
    }

    if report.is_empty() {
        if used_cached_devices {
            logger.log("Cached BACnet device list produced no data. Keeping cache for discovery fallback.");
        }
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

    let mut last_poll = tokio::time::Instant::now();
    let mut bacnet_runtime_state = BacnetRuntimeState::default();
    let trigger_read_owned = format!("/tmp/rs485/bacnet_read_{}", port_num);
    let trigger_write_owned = format!("/tmp/rs485/bacnet_write_{}", port_num);
    let request_owned = format!("/tmp/rs485/bacnet_request_{}", port_num);
    let write_request_owned = format!("/tmp/rs485/bacnet_write_request_{}", port_num);
    let result_owned = format!("/tmp/rs485/bacnet_result_{}", port_num);
    let periodic_result_owned = format!("/tmp/rs485/bacnet_last_result_{}", port_num);
    let trigger_read_path = trigger_read_owned.as_str();
    let trigger_write_path = trigger_write_owned.as_str();
    let request_path = request_owned.as_str();
    let write_request_path = write_request_owned.as_str();
    let result_path = result_owned.as_str();
    let periodic_result_path = periodic_result_owned.as_str();

    // Main event loop
    loop {
        // Handle trigger-file based writes (for LuCI UI)
        if Path::new(trigger_write_path).exists() {
            logger.log("BACnet write trigger detected");
            let _ = std::fs::remove_file(trigger_write_path);
            last_poll = tokio::time::Instant::now();

            let write_request = match load_trigger_write_request(write_request_path, &logger) {
                Ok(request) => request,
                Err(e) => {
                    let error_msg = format!("Error: {}", e);
                    let _ = std::fs::write(result_path, &error_msg);
                    logger.log(&error_msg);

                    if let Some(ref client) = mqtt_client {
                        let json = build_bacnet_error_uplink(&config.bacnet, &error_msg);
                        if !publish_mqtt_payload(
                            client,
                            mqtt_eventloop.as_mut(),
                            &config.mqtt.uplink_topic,
                            config.mqtt.qos_level,
                            &json,
                            &logger,
                        )
                        .await
                        {
                            mqtt_state = "failed_connect";
                        }
                    }

                    continue;
                }
            };

            match collect_requested_bacnet_write(
                &config,
                &logger,
                &mut bacnet_runtime_state,
                &write_request,
            )
            .await
            {
                Some(data) => {
                    logger.log(&format!(
                        "BACnet write completed: {}",
                        describe_bacnet_data_for_log(&data)
                    ));
                    if let Err(e) = std::fs::write(result_path, &data) {
                        logger.log(&format!("Failed to write result: {}", e));
                    }

                    if let Some(ref client) = mqtt_client {
                        let device_count = serde_json::from_str::<serde_json::Value>(&data)
                            .ok()
                            .and_then(|v| v.as_object().map(|o| o.len()))
                            .unwrap_or(0);
                        let json = limit_bacnet_mqtt_payload(
                            &config.bacnet,
                            build_bacnet_uplink(&config.bacnet, &data, device_count, None),
                            &logger,
                        );
                        if !publish_mqtt_payload(
                            client,
                            mqtt_eventloop.as_mut(),
                            &config.mqtt.uplink_topic,
                            config.mqtt.qos_level,
                            &json,
                            &logger,
                        )
                        .await
                        {
                            mqtt_state = "failed_connect";
                        }
                    }
                }
                None => {
                    let error_msg = "Error: BACnet write failed";
                    let _ = std::fs::write(result_path, error_msg);
                    logger.log(error_msg);

                    if let Some(ref client) = mqtt_client {
                        let json = build_bacnet_error_uplink(&config.bacnet, error_msg);
                        if !publish_mqtt_payload(
                            client,
                            mqtt_eventloop.as_mut(),
                            &config.mqtt.uplink_topic,
                            config.mqtt.qos_level,
                            &json,
                            &logger,
                        )
                        .await
                        {
                            mqtt_state = "failed_connect";
                        }
                    }
                }
            }
        }

        // Handle trigger-file based reads (for LuCI UI)
        if Path::new(trigger_read_path).exists() {
            logger.log("BACnet read trigger detected");
            let _ = std::fs::remove_file(trigger_read_path);
            last_poll = tokio::time::Instant::now();

            let read_request = match load_trigger_read_request(request_path, &logger) {
                Ok(request) => request,
                Err(e) => {
                    let error_msg = format!("Error: {}", e);
                    let _ = std::fs::write(result_path, &error_msg);
                    logger.log(&error_msg);

                    if let Some(ref client) = mqtt_client {
                        let json = build_bacnet_error_uplink(&config.bacnet, &error_msg);
                        if !publish_mqtt_payload(
                            client,
                            mqtt_eventloop.as_mut(),
                            &config.mqtt.uplink_topic,
                            config.mqtt.qos_level,
                            &json,
                            &logger,
                        )
                        .await
                        {
                            mqtt_state = "failed_connect";
                        }
                    }

                    continue;
                }
            };

            let read_result = if let Some(ref request) = read_request {
                collect_requested_bacnet_data(&config, &logger, &mut bacnet_runtime_state, request)
                    .await
            } else {
                collect_bacnet_data(&config, &logger, &mut bacnet_runtime_state).await
            };

            match read_result {
                Some(data) => {
                    logger.log(&format!(
                        "BACnet data collected: {}",
                        describe_bacnet_data_for_log(&data)
                    ));
                    if let Err(e) = std::fs::write(result_path, &data) {
                        logger.log(&format!("Failed to write result: {}", e));
                    }

                    // Publish to MQTT if connected
                    if let Some(ref client) = mqtt_client {
                        let device_count = serde_json::from_str::<serde_json::Value>(&data)
                            .ok()
                            .and_then(|v| v.as_object().map(|o| o.len()))
                            .unwrap_or(0);
                        let json = limit_bacnet_mqtt_payload(
                            &config.bacnet,
                            build_bacnet_uplink(&config.bacnet, &data, device_count, None),
                            &logger,
                        );
                        if !publish_mqtt_payload(
                            client,
                            mqtt_eventloop.as_mut(),
                            &config.mqtt.uplink_topic,
                            config.mqtt.qos_level,
                            &json,
                            &logger,
                        )
                        .await
                        {
                            mqtt_state = "failed_connect";
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
                        if !publish_mqtt_payload(
                            client,
                            mqtt_eventloop.as_mut(),
                            &config.mqtt.uplink_topic,
                            config.mqtt.qos_level,
                            &json,
                            &logger,
                        )
                        .await
                        {
                            mqtt_state = "failed_connect";
                        }
                    }
                }
            }
        }

        // Periodic polling
        let elapsed = last_poll.elapsed();
        if elapsed >= Duration::from_secs(config.bacnet.poll_interval) {
            last_poll = tokio::time::Instant::now();

            if let Some(data) = collect_bacnet_data(&config, &logger, &mut bacnet_runtime_state).await {
                // Keep periodic polling data separate from one-shot manual reads.
                let _ = std::fs::write(periodic_result_path, &data);

                // Publish to MQTT if connected
                if let Some(ref client) = mqtt_client {
                    let device_count = serde_json::from_str::<serde_json::Value>(&data)
                        .ok()
                        .and_then(|v| v.as_object().map(|o| o.len()))
                        .unwrap_or(0);
                    let json = limit_bacnet_mqtt_payload(
                        &config.bacnet,
                        build_bacnet_uplink(&config.bacnet, &data, device_count, None),
                        &logger,
                    );
                    if !publish_mqtt_payload(
                        client,
                        mqtt_eventloop.as_mut(),
                        &config.mqtt.uplink_topic,
                        config.mqtt.qos_level,
                        &json,
                        &logger,
                    )
                    .await
                    {
                        mqtt_state = "failed_connect";
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
