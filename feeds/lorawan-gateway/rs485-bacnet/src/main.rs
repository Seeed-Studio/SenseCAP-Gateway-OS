// BACnet MS/TP to MQTT Bridge
//
// This module bridges BACnet MS/TP devices on RS-485 to MQTT
// with support for reading/writing BACnet objects

mod mstp;
mod bacnet;

use chrono::{Local, Datelike};
use rumqttc::{AsyncClient, Event, Incoming, MqttOptions, QoS};
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::Write;
use std::path::Path;
use std::sync::{Arc, Mutex as StdMutex};
use std::sync::OnceLock;
use std::time::{Duration, Instant};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::time::sleep;
use tokio_serial::{DataBits, Parity, StopBits};
use tokio::sync::mpsc;
use log::{info, warn, error, debug};

use mstp::{MstpFrame, MstpMaster, MstpError};
use bacnet::{
    BacnetObjectType, BacnetProperty, BacnetValue, ObjectIdentifier,
    Apdu,
};

// Global startup time for logger
static START_TIME: OnceLock<Instant> = OnceLock::new();

/// Initialize timezone from UCI system configuration
fn init_timezone() {
    match std::process::Command::new("uci")
        .args(&["get", "system.@system[0].timezone"])
        .output()
    {
        Ok(output) if output.status.success() => {
            if let Ok(tz) = String::from_utf8(output.stdout) {
                let tz = tz.trim();
                if !tz.is_empty() && tz != "UTC" {
                    std::env::set_var("TZ", tz);
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

// Global constants for file paths
const TRIGGER_READ_PATH: &str = "/tmp/rs485/bacnet_read";
const TRIGGER_WRITE_PATH: &str = "/tmp/rs485/bacnet_write";
const RESULT_PATH: &str = "/tmp/rs485/bacnet_result";
const LOG_PATH: &str = "/tmp/rs485/log";

// Configuration Structures
#[derive(Debug, Clone, PartialEq)]
struct Config {
    mqtt: MqttConfig,
    serial: SerialConfig,
    bacnet: BacnetConfig,
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
}

#[derive(Debug, Clone, PartialEq)]
struct SerialConfig {
    enabled: bool,
    device: String,
    baud_rate: u32,
    data_bits: u8,
    parity: String,
    stop_bits: u8,
    flow_control: String,
}

#[derive(Debug, Clone, PartialEq)]
struct BacnetConfig {
    enabled: bool,
    device_mac: u8,
    max_master: u8,
    max_info_frames: u8,
    reply_timeout: u64,
    timeout_multiplier: u8,
    token_hold_time: u64,
    polling_interval: u64,
    work_mode: String, // once, poll
    object_type: String,
    object_instance: u32,
    property_identifier: String,
}

// Trigger and Result structures
#[derive(Debug, Serialize, Deserialize)]
struct ReadTrigger {
    device_id: u32,
    object_type: String,
    object_instance: u32,
    property_identifier: String,
    array_index: Option<u32>,
}

#[derive(Debug, Serialize, Deserialize)]
struct WriteTrigger {
    device_id: u32,
    object_type: String,
    object_instance: u32,
    property_identifier: String,
    value: serde_json::Value,
    priority: Option<u8>,
}

#[derive(Debug, Serialize, Deserialize)]
struct BACnetResult {
    timestamp: String,
    operation: String,
    device_id: u32,
    object_type: String,
    object_instance: u32,
    property_identifier: String,
    success: bool,
    value: Option<serde_json::Value>,
    error: Option<String>,
}

// MS/TP Transport Layer
struct MstpTransport {
    master: MstpMaster,
    serial_port: Arc<StdMutex<tokio_serial::SerialStream>>,
}

impl MstpTransport {
    fn new(config: &BacnetConfig, serial_port: tokio_serial::SerialStream) -> Self {
        // Use fixed source MAC (127) for this station; device_mac is used as target only
        let master = MstpMaster::new(
            127,  // this_station (our source MAC)
            config.max_master,
            config.max_info_frames,
        );

        Self {
            master,
            serial_port: Arc::new(StdMutex::new(serial_port)),
        }
    }

    /// Send frame and wait for response
    async fn send_frame(&self, frame: &MstpFrame) -> Result<Option<MstpFrame>, MstpError> {
        let encoded = frame.encode_with_crc();

        // Lock serial port for write and potential read
        let mut port = self.serial_port.lock().unwrap();

        // Send frame
        tokio::io::AsyncWriteExt::write_all(&mut *port, &encoded).await
            .map_err(|_| MstpError::Timeout)?;

        // If expecting reply, wait for response
        if frame.expects_reply() {
            let timeout = Duration::from_millis(self.master.timing.treply_timeout);
            let mut buffer = [0u8; 512];

            let result = tokio::time::timeout(timeout, async {
                let n = tokio::io::AsyncReadExt::read(&mut *port, &mut buffer).await
                    .map_err(|_| MstpError::Timeout)?;
                if n >= 10 {
                    MstpFrame::decode_with_crc(&buffer[..n])
                } else {
                    Err(MstpError::InvalidFrameLength)
                }
            }).await;

            match result {
                Ok(Ok(frame)) => Ok(Some(frame)),
                Ok(Err(_)) | Err(_) => Ok(None),
            }
        } else {
            Ok(None)
        }
    }

    /// Read property from BACnet device
    async fn read_property(
        &mut self,
        device_id: u8,
        object_type: BacnetObjectType,
        instance: u32,
        property: BacnetProperty,
        array_index: Option<u32>,
    ) -> Result<Option<BacnetValue>, MstpError> {
        // Create APDU
        let object_id = ObjectIdentifier::new(object_type, instance);
        let apdu = Apdu::read_property_request(device_id, object_id, property, array_index);

        // Create MS/TP frame
        let frame = MstpFrame::data_expect_reply(device_id, self.master.this_station, apdu.encode());

        // Send and receive
        if let Some(response_frame) = self.send_frame(&frame).await? {
            debug!("Received MS/TP response: {} bytes", response_frame.data.len());

            // Parse response APDU
            if let Ok(response_apdu) = Apdu::decode(&response_frame.data) {
                debug!("APDU service: {:?}", response_apdu.service);

                // Parse property value from ReadPropertyAck
                if let Some(value) = response_apdu.parse_read_property_ack() {
                    info!("Read property value: {:?}", value);
                    Ok(Some(value))
                } else {
                    warn!("Failed to parse property value from APDU");
                    Ok(None)
                }
            } else {
                error!("Failed to decode APDU");
                Err(MstpError::InvalidFrameType)
            }
        } else {
            error!("No response from device");
            Err(MstpError::Timeout)
        }
    }

    /// Write property to BACnet device
    async fn write_property(
        &mut self,
        device_id: u8,
        object_type: BacnetObjectType,
        instance: u32,
        property: BacnetProperty,
        value: BacnetValue,
        priority: Option<u8>,
    ) -> Result<bool, MstpError> {
        // Create APDU
        let object_id = ObjectIdentifier::new(object_type, instance);
        let apdu = Apdu::write_property_request(device_id, object_id, property, value, priority);

        // Create MS/TP frame (no reply expected for write)
        let frame = MstpFrame::data_no_reply(device_id, self.master.this_station, apdu.encode());

        // Send
        self.send_frame(&frame).await?;
        Ok(true)
    }

    /// Poll for token and handle any incoming frames
    async fn poll_token(&mut self) -> Result<(), MstpError> {
        // Wait for incoming frame
        let mut port = self.serial_port.lock().unwrap();

        let mut buffer = [0u8; 512];
        let timeout = Duration::from_millis(self.master.timing.tusage_timeout);

        let result = tokio::time::timeout(timeout, async {
            tokio::io::AsyncReadExt::read(&mut *port, &mut buffer).await
                .map_err(|_| MstpError::Timeout)
        }).await;

        if let Ok(Ok(n)) = result {
            if n >= 10 {
                if let Ok(frame) = MstpFrame::decode_with_crc(&buffer[..n]) {
                    self.master.receive_frame(&frame)?;
                }
            }
        }

        Ok(())
    }
}

// Log message to file
fn log_message(message: &str) {
    let timestamp = get_timestamp();
    let log_entry = format!("[{}] {}\n", timestamp, message);

    if let Ok(mut file) = File::open(LOG_PATH) {
        let _ = file.write_all(log_entry.as_bytes());
    }
}

fn get_timestamp() -> String {
    let now = Local::now();
    // Check if system time is valid (year >= 2024 means time is synced)
    if now.year() >= 2024 {
        now.format("%Y-%m-%d %H:%M:%S").to_string()
    } else {
        // Use relative uptime when system time is not synced
        let start = START_TIME.get().unwrap();
        let uptime_secs = start.elapsed().as_secs();
        format!("+{}s", uptime_secs)
    }
}

// Load configuration from UCI
fn load_config() -> Result<Config, Box<dyn std::error::Error>> {
    // Default configuration
    let mut config = Config {
        mqtt: MqttConfig {
            enabled: true,
            transport: "tcp".to_string(),
            host: "10.0.0.104".to_string(),
            port: 1883,
            username: None,
            password: None,
            client_id: format!("rs485-bacnet-{}", hostname::get().unwrap().to_string_lossy()),
            keepalive: 60,
            uplink_topic: "rs485/bacnet/uplink".to_string(),
            downlink_topic: "rs485/bacnet/downlink".to_string(),
            qos_level: QoS::AtMostOnce,
        },
        serial: SerialConfig {
            enabled: true,
            device: "/dev/RS485-1".to_string(),
            baud_rate: 9600,
            data_bits: 8,
            parity: "none".to_string(),
            stop_bits: 1,
            flow_control: "none".to_string(),
        },
        bacnet: BacnetConfig {
            enabled: true,
            device_mac: 1,
            max_master: 127,
            max_info_frames: 1,
            reply_timeout: 200,
            timeout_multiplier: 3,
            token_hold_time: 10,
            polling_interval: 5,
            work_mode: "once".to_string(),
            object_type: "analogInput".to_string(),
            object_instance: 0,
            property_identifier: "presentValue".to_string(),
        },
    };

    // Load from UCI /etc/config/rs485-module
    // Helper function to get UCI value
    let uci_get = |key: &str| -> Option<String> {
        match std::process::Command::new("uci")
            .args(["-q", "get", key])
            .output()
        {
            Ok(output) => {
                let value = String::from_utf8_lossy(&output.stdout).trim().to_string();
                if value.is_empty() || value.starts_with("Entry not found") {
                    None
                } else {
                    Some(value)
                }
            }
            Err(_) => None,
        }
    };

    // Load MQTT configuration
    if let Some(host) = uci_get("rs485-module.mqtt.broker") {
        config.mqtt.host = host;
    } else if let Some(host) = uci_get("rs485-module.mqtt.host") {
        config.mqtt.host = host;
    }
    if let Some(port) = uci_get("rs485-module.mqtt.port") {
        config.mqtt.port = port.parse().unwrap_or(1883);
    }
    if let Some(topic) = uci_get("rs485-module.mqtt.uplink_topic") {
        config.mqtt.uplink_topic = topic;
    }
    if let Some(topic) = uci_get("rs485-module.mqtt.downlink_topic") {
        config.mqtt.downlink_topic = topic;
    }
    if let Some(user) = uci_get("rs485-module.mqtt.username") {
        config.mqtt.username = Some(user);
    }
    if let Some(pass) = uci_get("rs485-module.mqtt.password") {
        config.mqtt.password = Some(pass);
    }

    // Load Serial configuration
    if let Some(device) = uci_get("rs485-module.serial.device") {
        config.serial.device = device;
    }
    if let Some(baud) = uci_get("rs485-module.serial.baudrate") {
        config.serial.baud_rate = baud.parse().unwrap_or(9600);
    }

    // Load BACnet protocol configuration
    // Check if BACnet is enabled in bacnet section
    if let Some(enabled) = uci_get("rs485-module.bacnet.enabled") {
        config.bacnet.enabled = enabled == "1";
    }

    if let Some(mac) = uci_get("rs485-module.bacnet.device_mac") {
        config.bacnet.device_mac = mac.parse().unwrap_or(1);
    }
    if let Some(mode) = uci_get("rs485-module.bacnet.poll_mode") {
        config.bacnet.work_mode = mode;
    }
    if let Some(interval) = uci_get("rs485-module.bacnet.poll_interval") {
        config.bacnet.polling_interval = interval.parse().unwrap_or(5);
    }
    if let Some(obj_type) = uci_get("rs485-module.bacnet.object_type") {
        config.bacnet.object_type = obj_type;
    }
    if let Some(instance) = uci_get("rs485-module.bacnet.object_instance") {
        config.bacnet.object_instance = instance.parse().unwrap_or(0);
    }
    if let Some(prop) = uci_get("rs485-module.bacnet.property_identifier") {
        config.bacnet.property_identifier = prop;
    }

    info!("Loaded configuration: MQTT={}, BACnet enabled={}, device MAC={}",
          config.mqtt.host, config.bacnet.enabled, config.bacnet.device_mac);

    Ok(config)
}

// Initialize MQTT client
async fn init_mqtt_client(
    config: &Config,
    downlink_tx: mpsc::UnboundedSender<String>,
) -> Result<AsyncClient, Box<dyn std::error::Error>> {
    let mut mqttoptions = MqttOptions::new(
        config.mqtt.client_id.clone(),
        config.mqtt.host.clone(),
        config.mqtt.port,
    );

    mqttoptions.set_keep_alive(Duration::from_secs(config.mqtt.keepalive));

    if let (Some(username), Some(password)) = (&config.mqtt.username, &config.mqtt.password) {
        mqttoptions.set_credentials(username, password);
    }

    let (client, mut eventloop) = AsyncClient::new(mqttoptions, 10);
    let client_clone = client.clone();
    let downlink_topic = config.mqtt.downlink_topic.clone();

    // Handle connection and incoming messages in background
    tokio::spawn(async move {
        let mut connected = false;
        loop {
            if let Ok(event) = eventloop.poll().await {
                match event {
                    Event::Incoming(Incoming::Disconnect) => {
                        warn!("MQTT disconnected");
                        connected = false;
                    }
                    Event::Incoming(Incoming::ConnAck(_)) => {
                        info!("MQTT connected");
                        if !connected {
                            connected = true;
                            // Subscribe to downlink topic after connection
                            let client = client_clone.clone();
                            let topic = downlink_topic.clone();
                            tokio::spawn(async move {
                                if let Err(e) = client.subscribe(&topic, QoS::AtLeastOnce).await {
                                    error!("Failed to subscribe to downlink topic: {}", e);
                                } else {
                                    info!("Subscribed to downlink topic: {}", topic);
                                }
                            });
                        }
                    }
                    Event::Incoming(Incoming::Publish(packet)) => {
                        let topic = packet.topic.clone();
                        let payload = String::from_utf8_lossy(&packet.payload).to_string();

                        // Handle downlink messages
                        if topic == downlink_topic {
                            info!("Received downlink message: {}", payload);
                            if let Err(e) = downlink_tx.send(payload) {
                                error!("Failed to send downlink message to channel: {}", e);
                            }
                        }
                    }
                    Event::Outgoing(_) => {
                        // Ignore outgoing acks
                    }
                    _ => {}
                }
            }
        }
    });

    Ok(client)
}

// Initialize serial port
async fn init_serial_port(config: &SerialConfig) -> Result<tokio_serial::SerialStream, Box<dyn std::error::Error>> {
    use tokio_serial::SerialPortBuilderExt;

    let port = tokio_serial::new(&config.device, config.baud_rate)
        .data_bits(DataBits::Eight)
        .parity(match config.parity.as_str() {
            "none" => Parity::None,
            "even" => Parity::Even,
            "odd" => Parity::Odd,
            _ => Parity::None,
        })
        .stop_bits(StopBits::One)
        .timeout(Duration::from_secs(5))
        .open_native_async()?;

    Ok(port)
}

// Handle read trigger
async fn handle_read_trigger(
    config: &Config,
    transport: &mut MstpTransport,
    mqtt_client: &Option<AsyncClient>,
) -> Result<(), Box<dyn std::error::Error>> {
    // Read trigger file
    let trigger_content = std::fs::read_to_string(TRIGGER_READ_PATH)?;
    let trigger: ReadTrigger = serde_json::from_str(&trigger_content)?;

    info!("Processing read trigger: {:?}", trigger);
    log_message(&format!("Processing read trigger: device_id={}, object_type={}",
        trigger.device_id, trigger.object_type));

    // Parse object type and property
    let object_type = BacnetObjectType::from_str(&trigger.object_type)
        .ok_or("Invalid object type")?;
    let property = BacnetProperty::from_str(&trigger.property_identifier)
        .ok_or("Invalid property identifier")?;

    // Perform read
    let result = transport.read_property(
        trigger.device_id as u8,
        object_type,
        trigger.object_instance,
        property,
        trigger.array_index,
    ).await;

    // Create result
    let bacnet_result = match result {
        Ok(value) => BACnetResult {
            timestamp: Local::now().to_rfc3339(),
            operation: "read".to_string(),
            device_id: trigger.device_id,
            object_type: trigger.object_type,
            object_instance: trigger.object_instance,
            property_identifier: trigger.property_identifier,
            success: true,
            value: value.map(|v| serde_json::to_value(v).ok()).flatten(),
            error: None,
        },
        Err(e) => BACnetResult {
            timestamp: Local::now().to_rfc3339(),
            operation: "read".to_string(),
            device_id: trigger.device_id,
            object_type: trigger.object_type,
            object_instance: trigger.object_instance,
            property_identifier: trigger.property_identifier,
            success: false,
            value: None,
            error: Some(format!("{:?}", e)),
        },
    };

    // Write result
    let result_json = serde_json::to_string_pretty(&bacnet_result)?;
    std::fs::write(RESULT_PATH, &result_json)?;

    // Publish to MQTT if enabled
    if let Some(client) = mqtt_client {
        client.publish(
            &config.mqtt.uplink_topic,
            QoS::AtLeastOnce,
            false,
            result_json.as_bytes(),
        ).await?;
    }

    Ok(())
}

// Handle write trigger
async fn handle_write_trigger(
    config: &Config,
    transport: &mut MstpTransport,
    mqtt_client: &Option<AsyncClient>,
) -> Result<(), Box<dyn std::error::Error>> {
    // Read trigger file
    let trigger_content = std::fs::read_to_string(TRIGGER_WRITE_PATH)?;
    let trigger: WriteTrigger = serde_json::from_str(&trigger_content)?;

    info!("Processing write trigger: {:?}", trigger);
    log_message(&format!("Processing write trigger: device_id={}, value={:?}",
        trigger.device_id, trigger.value));

    // Parse object type and property
    let object_type = BacnetObjectType::from_str(&trigger.object_type)
        .ok_or("Invalid object type")?;
    let property = BacnetProperty::from_str(&trigger.property_identifier)
        .ok_or("Invalid property identifier")?;

    // Parse value
    let bacnet_value = match &trigger.value {
        serde_json::Value::Number(n) => {
            if let Some(u) = n.as_u64() {
                BacnetValue::Unsigned(u as u32)
            } else if let Some(i) = n.as_i64() {
                BacnetValue::Signed(i as i32)
            } else if let Some(f) = n.as_f64() {
                BacnetValue::Real(f as f32)
            } else {
                return Err("Invalid number value".into());
            }
        }
        serde_json::Value::Bool(b) => BacnetValue::Boolean(*b),
        serde_json::Value::String(s) => BacnetValue::String(s.clone()),
        _ => return Err("Unsupported value type".into()),
    };

    // Perform write
    let result = transport.write_property(
        trigger.device_id as u8,
        object_type,
        trigger.object_instance,
        property,
        bacnet_value,
        trigger.priority,
    ).await;

    // Create result
    let bacnet_result = match result {
        Ok(_) => BACnetResult {
            timestamp: Local::now().to_rfc3339(),
            operation: "write".to_string(),
            device_id: trigger.device_id,
            object_type: trigger.object_type,
            object_instance: trigger.object_instance,
            property_identifier: trigger.property_identifier,
            success: true,
            value: Some(trigger.value),
            error: None,
        },
        Err(e) => BACnetResult {
            timestamp: Local::now().to_rfc3339(),
            operation: "write".to_string(),
            device_id: trigger.device_id,
            object_type: trigger.object_type,
            object_instance: trigger.object_instance,
            property_identifier: trigger.property_identifier,
            success: false,
            value: None,
            error: Some(format!("{:?}", e)),
        },
    };

    // Write result
    let result_json = serde_json::to_string_pretty(&bacnet_result)?;
    std::fs::write(RESULT_PATH, &result_json)?;

    // Publish to MQTT if enabled
    if let Some(client) = mqtt_client {
        client.publish(
            &config.mqtt.uplink_topic,
            QoS::AtLeastOnce,
            false,
            result_json.as_bytes(),
        ).await?;
    }

    Ok(())
}

// Perform poll read
async fn perform_poll_read(
    config: &Config,
    transport: &mut MstpTransport,
    mqtt_client: &Option<AsyncClient>,
) -> Result<(), Box<dyn std::error::Error>> {
    // Parse default object type and property
    let object_type = BacnetObjectType::from_str(&config.bacnet.object_type)
        .ok_or("Invalid object type")?;
    let property = BacnetProperty::from_str(&config.bacnet.property_identifier)
        .ok_or("Invalid property identifier")?;

    // Perform read
    let result = transport.read_property(
        config.bacnet.device_mac as u8, // Use device MAC as target
        object_type,
        config.bacnet.object_instance,
        property,
        None,
    ).await;

    // Create result
    let bacnet_result = match result {
        Ok(value) => BACnetResult {
            timestamp: Local::now().to_rfc3339(),
            operation: "poll".to_string(),
            device_id: config.bacnet.device_mac as u32,
            object_type: config.bacnet.object_type.clone(),
            object_instance: config.bacnet.object_instance,
            property_identifier: config.bacnet.property_identifier.clone(),
            success: true,
            value: value.map(|v| serde_json::to_value(v).ok()).flatten(),
            error: None,
        },
        Err(e) => {
            error!("Poll read error: {:?}", e);
            BACnetResult {
                timestamp: Local::now().to_rfc3339(),
                operation: "poll".to_string(),
                device_id: config.bacnet.device_mac as u32,
                object_type: config.bacnet.object_type.clone(),
                object_instance: config.bacnet.object_instance,
                property_identifier: config.bacnet.property_identifier.clone(),
                success: false,
                value: None,
                error: Some(format!("{:?}", e)),
            }
        }
    };

    // Publish to MQTT if enabled
    if let Some(client) = mqtt_client {
        let result_json = serde_json::to_string_pretty(&bacnet_result)?;
        client.publish(
            &config.mqtt.uplink_topic,
            QoS::AtLeastOnce,
            false,
            result_json.as_bytes(),
        ).await?;
    }

    Ok(())
}

// Handle MQTT downlink message
async fn handle_mqtt_downlink(
    config: &Config,
    transport: &mut MstpTransport,
    mqtt_client: &Option<AsyncClient>,
    payload: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    // Parse downlink message
    let downlink: WriteTrigger = serde_json::from_str(payload)?;

    info!("Processing MQTT downlink: {:?}", downlink);
    log_message(&format!("Processing MQTT downlink: device_id={}, value={:?}",
        downlink.device_id, downlink.value));

    // Parse object type and property
    let object_type = BacnetObjectType::from_str(&downlink.object_type)
        .ok_or("Invalid object type")?;
    let property = BacnetProperty::from_str(&downlink.property_identifier)
        .ok_or("Invalid property identifier")?;

    // Parse value
    let bacnet_value = match &downlink.value {
        serde_json::Value::Number(n) => {
            if let Some(u) = n.as_u64() {
                BacnetValue::Unsigned(u as u32)
            } else if let Some(i) = n.as_i64() {
                BacnetValue::Signed(i as i32)
            } else if let Some(f) = n.as_f64() {
                BacnetValue::Real(f as f32)
            } else {
                return Err("Invalid number value".into());
            }
        }
        serde_json::Value::Bool(b) => BacnetValue::Boolean(*b),
        serde_json::Value::String(s) => BacnetValue::String(s.clone()),
        _ => return Err("Unsupported value type".into()),
    };

    // Perform write
    let result = transport.write_property(
        downlink.device_id as u8,
        object_type,
        downlink.object_instance,
        property,
        bacnet_value,
        downlink.priority,
    ).await;

    // Create result
    let bacnet_result = match result {
        Ok(_) => BACnetResult {
            timestamp: Local::now().to_rfc3339(),
            operation: "write".to_string(),
            device_id: downlink.device_id,
            object_type: downlink.object_type,
            object_instance: downlink.object_instance,
            property_identifier: downlink.property_identifier,
            success: true,
            value: Some(downlink.value),
            error: None,
        },
        Err(e) => BACnetResult {
            timestamp: Local::now().to_rfc3339(),
            operation: "write".to_string(),
            device_id: downlink.device_id,
            object_type: downlink.object_type,
            object_instance: downlink.object_instance,
            property_identifier: downlink.property_identifier,
            success: false,
            value: None,
            error: Some(format!("{:?}", e)),
        },
    };

    // Publish to MQTT if enabled
    if let Some(client) = mqtt_client {
        let result_json = serde_json::to_string_pretty(&bacnet_result)?;
        client.publish(
            &config.mqtt.uplink_topic,
            QoS::AtLeastOnce,
            false,
            result_json.as_bytes(),
        ).await?;
        info!("Published write result to MQTT");
    }

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize timezone from UCI system configuration
    init_timezone();

    // Initialize startup time for logger
    let _ = START_TIME.get_or_init(|| Instant::now());

    env_logger::Builder::from_default_env()
        .filter_level(log::LevelFilter::Info)
        .init();

    info!("BACnet MS/TP to MQTT Bridge starting...");
    log_message("BACnet MS/TP to MQTT Bridge starting");

    // Load configuration
    let config = load_config()?;

    if !config.serial.enabled || !config.bacnet.enabled {
        warn!("BACnet module is disabled in configuration");
        return Ok(());
    }

    // Initialize serial port
    let serial_port = init_serial_port(&config.serial).await?;

    // Initialize MS/TP transport
    let mut transport = MstpTransport::new(&config.bacnet, serial_port);

    // Initialize MQTT client if enabled
    let (downlink_tx, mut downlink_rx) = mpsc::unbounded_channel::<String>();
    let mqtt_client = if config.mqtt.enabled {
        Some(init_mqtt_client(&config, downlink_tx).await?)
    } else {
        None
    };

    // Create trigger directories
    std::fs::create_dir_all("/tmp/rs485")?;

    info!("BACnet MS/TP bridge initialized");
    log_message("BACnet MS/TP bridge initialized");

    // Main loop
    loop {
        // Poll for token and handle incoming frames
        if let Err(e) = transport.poll_token().await {
            error!("Poll token error: {:?}", e);
        }

        // Check for read trigger
        if Path::new(TRIGGER_READ_PATH).exists() {
            if let Err(e) = handle_read_trigger(&config, &mut transport, &mqtt_client).await {
                error!("Read trigger error: {}", e);
                log_message(&format!("Read trigger error: {}", e));
            }
            let _ = std::fs::remove_file(TRIGGER_READ_PATH);
        }

        // Check for write trigger
        if Path::new(TRIGGER_WRITE_PATH).exists() {
            if let Err(e) = handle_write_trigger(&config, &mut transport, &mqtt_client).await {
                error!("Write trigger error: {}", e);
                log_message(&format!("Write trigger error: {}", e));
            }
            let _ = std::fs::remove_file(TRIGGER_WRITE_PATH);
        }

        // Check for MQTT downlink messages
        if let Some(downlink_msg) = downlink_rx.try_recv().ok() {
            if let Err(e) = handle_mqtt_downlink(&config, &mut transport, &mqtt_client, &downlink_msg).await {
                error!("MQTT downlink error: {}", e);
                log_message(&format!("MQTT downlink error: {}", e));
            }
        }

        // Poll mode operation
        if config.bacnet.work_mode == "poll" {
            if let Err(e) = perform_poll_read(&config, &mut transport, &mqtt_client).await {
                error!("Poll read error: {}", e);
            }
        }

        sleep(Duration::from_secs(config.bacnet.polling_interval)).await;
    }
}
