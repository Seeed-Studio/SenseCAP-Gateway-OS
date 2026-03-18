// BACnet Application Layer
//
// Implements BACnet application services and object types
// Based on ANSI/ASHRAE 135-2012

use serde::{Serialize, Deserialize};
use crate::mstp::MstpFrame;

// BACnet Object Types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BacnetObjectType {
    AnalogInput = 0,
    AnalogOutput = 1,
    AnalogValue = 2,
    BinaryInput = 3,
    BinaryOutput = 4,
    BinaryValue = 5,
    Device = 8,
    CharacterstringValue = 39,
}

impl BacnetObjectType {
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            0 => Some(Self::AnalogInput),
            1 => Some(Self::AnalogOutput),
            2 => Some(Self::AnalogValue),
            3 => Some(Self::BinaryInput),
            4 => Some(Self::BinaryOutput),
            5 => Some(Self::BinaryValue),
            8 => Some(Self::Device),
            39 => Some(Self::CharacterstringValue),
            _ => None,
        }
    }

    pub fn to_u8(&self) -> u8 {
        *self as u8
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "analoginput" => Some(Self::AnalogInput),
            "analogoutput" => Some(Self::AnalogOutput),
            "analogvalue" => Some(Self::AnalogValue),
            "binaryinput" => Some(Self::BinaryInput),
            "binaryoutput" => Some(Self::BinaryOutput),
            "binaryvalue" => Some(Self::BinaryValue),
            "device" => Some(Self::Device),
            "characterstringvalue" => Some(Self::CharacterstringValue),
            _ => None,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::AnalogInput => "analogInput",
            Self::AnalogOutput => "analogOutput",
            Self::AnalogValue => "analogValue",
            Self::BinaryInput => "binaryInput",
            Self::BinaryOutput => "binaryOutput",
            Self::BinaryValue => "binaryValue",
            Self::Device => "device",
            Self::CharacterstringValue => "characterstringValue",
        }
    }
}

// BACnet Property Identifiers
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BacnetProperty {
    PresentValue = 28,
    StatusFlags = 111,
    Units = 117,
    Description = 29,
    ObjectName = 77,
    ObjectType = 75,
    ObjectIdentifier = 79,
}

impl BacnetProperty {
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "presentvalue" => Some(Self::PresentValue),
            "statusflags" => Some(Self::StatusFlags),
            "units" => Some(Self::Units),
            "description" => Some(Self::Description),
            "objectname" => Some(Self::ObjectName),
            "objecttype" => Some(Self::ObjectType),
            "objectidentifier" => Some(Self::ObjectIdentifier),
            _ => None,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::PresentValue => "presentValue",
            Self::StatusFlags => "statusFlags",
            Self::Units => "units",
            Self::Description => "description",
            Self::ObjectName => "objectName",
            Self::ObjectType => "objectType",
            Self::ObjectIdentifier => "objectIdentifier",
        }
    }

    pub fn to_u16(&self) -> u16 {
        *self as u16
    }
}

// BACnet Application Tag Types
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ApplicationTag {
    Null = 0,
    Boolean = 1,
    Unsigned = 2,
    Signed = 3,
    Real = 4,
    Double = 5,
    OctetString = 6,
    CharacterString = 7,
    BitString = 8,
    Enumerated = 9,
    Date = 10,
    Time = 11,
    ObjectIdentifier = 12,
}

// BACnet Service Choices
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BacnetService {
    WhoIs = 0,
    IAm = 1,
    ReadProperty = 8,
    ReadPropertyConditional = 9,
    ReadPropertyAck = 100,  // Complex ACK (not a service choice per se)
    WriteProperty = 12,
    WritePropertyMultiple = 16,
}

impl BacnetService {
    pub fn to_u8(&self) -> u8 {
        *self as u8
    }

    pub fn from_u8_with_tag(value: u8, is_confirmed: bool) -> Option<Self> {
        match (value, is_confirmed) {
            (0, _) => Some(Self::WhoIs),
            (1, _) => Some(Self::IAm),
            (8, true) => Some(Self::ReadProperty),
            (9, false) => Some(Self::ReadPropertyAck),
            (12, true) => Some(Self::WriteProperty),
            _ => None,
        }
    }
}

// BACnet Object Identifier
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ObjectIdentifier {
    pub object_type: BacnetObjectType,
    pub instance: u32,
}

impl ObjectIdentifier {
    pub fn new(object_type: BacnetObjectType, instance: u32) -> Self {
        Self {
            object_type,
            instance,
        }
    }

    pub fn encode(&self) -> u32 {
        // Encode as BACnet object identifier: (type << 22) | instance
        (self.object_type.to_u8() as u32) << 22 | (self.instance & 0x3FFFFF)
    }

    pub fn decode(value: u32) -> Option<Self> {
        let object_type = (value >> 22) as u8;
        let instance = value & 0x3FFFFF;
        BacnetObjectType::from_u8(object_type).map(|ot| Self {
            object_type: ot,
            instance,
        })
    }
}

// BACnet Value Types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum BacnetValue {
    Null,
    Boolean(bool),
    Unsigned(u32),
    Signed(i32),
    Real(f32),
    Double(f64),
    String(String),
    ObjectIdentifier(ObjectIdentifier),
}

// Read Property Request
#[derive(Debug, Clone)]
pub struct ReadPropertyRequest {
    pub destination: u8,
    pub object_identifier: ObjectIdentifier,
    pub property_identifier: BacnetProperty,
    pub array_index: Option<u32>,
}

// Write Property Request
#[derive(Debug, Clone)]
pub struct WritePropertyRequest {
    pub destination: u8,
    pub object_identifier: ObjectIdentifier,
    pub property_identifier: BacnetProperty,
    pub value: BacnetValue,
    pub priority: Option<u8>,
}

// BACnet APDU (Application Protocol Data Unit)
#[derive(Debug, Clone)]
pub struct Apdu {
    pub service: BacnetService,
    pub payload: Vec<u8>,
}

impl Apdu {
    /// Create a ReadProperty request APDU
    pub fn read_property_request(
        _dest_device: u8,
        object_id: ObjectIdentifier,
        property: BacnetProperty,
        array_index: Option<u32>,
    ) -> Self {
        let mut payload = Vec::new();

        // Object identifier (4 bytes)
        let encoded_id = object_id.encode();
        payload.extend_from_slice(&encoded_id.to_be_bytes());

        // Property identifier (2 bytes)
        payload.extend_from_slice(&property.to_u16().to_be_bytes());

        // Optional array index
        if let Some(index) = array_index {
            payload.extend_from_slice(&index.to_be_bytes());
        } else {
            payload.extend_from_slice(&0xFFFF_u16.to_be_bytes());
        }

        Self {
            service: BacnetService::ReadProperty,
            payload,
        }
    }

    /// Create a WriteProperty request APDU
    pub fn write_property_request(
        _dest_device: u8,
        object_id: ObjectIdentifier,
        property: BacnetProperty,
        value: BacnetValue,
        priority: Option<u8>,
    ) -> Self {
        let mut payload = Vec::new();

        // Object identifier
        let encoded_id = object_id.encode();
        payload.extend_from_slice(&encoded_id.to_be_bytes());

        // Property identifier
        payload.extend_from_slice(&property.to_u16().to_be_bytes());

        // Optional priority
        if let Some(p) = priority {
            payload.push(p);
        } else {
            payload.push(0xFF); // No priority
        }

        // Value (simplified - in reality needs proper encoding)
        match value {
            BacnetValue::Unsigned(v) => {
                payload.extend_from_slice(&v.to_be_bytes());
            }
            BacnetValue::Boolean(v) => {
                payload.push(if v { 1 } else { 0 });
            }
            BacnetValue::String(s) => {
                payload.extend_from_slice(s.as_bytes());
            }
            _ => {
                // Default encoding
            }
        }

        Self {
            service: BacnetService::WriteProperty,
            payload,
        }
    }

    /// Encode APDU to bytes
    pub fn encode(&self) -> Vec<u8> {
        let mut bytes = Vec::new();

        // PDU type (confirmed request = 0x00)
        bytes.push(0x00);

        // Service choice
        bytes.push(self.service.to_u8());

        // Invoke ID (simplified)
        bytes.extend_from_slice(&1u16.to_be_bytes());

        // Payload
        bytes.extend_from_slice(&self.payload);

        bytes
    }

    /// Decode APDU from bytes
    pub fn decode(bytes: &[u8]) -> Result<Self, &'static str> {
        if bytes.len() < 4 {
            return Err("APDU too short");
        }

        // Parse PDU type
        let pdu_type = bytes[0] & 0xF0;

        // Determine service based on PDU type and service choice
        let service_choice = bytes[1];
        let service = if pdu_type == 0x00 {
            // Confirmed request
            match service_choice {
                8 => BacnetService::ReadProperty,
                12 => BacnetService::WriteProperty,
                _ => return Err("Unknown confirmed service"),
            }
        } else if pdu_type == 0x10 {
            // Unconfirmed request
            match service_choice {
                0 => BacnetService::WhoIs,
                1 => BacnetService::IAm,
                _ => return Err("Unknown unconfirmed service"),
            }
        } else if pdu_type == 0x20 {
            // Simple ACK
            match service_choice {
                8 => BacnetService::ReadProperty,  // Actually an ACK
                12 => BacnetService::WriteProperty,  // Actually an ACK
                _ => return Err("Unknown simple ACK"),
            }
        } else if pdu_type == 0x30 {
            // Complex ACK
            match service_choice {
                8 => BacnetService::ReadPropertyAck,
                _ => return Err("Unknown complex ACK"),
            }
        } else {
            return Err("Unknown PDU type");
        };

        let payload = bytes[4..].to_vec();

        Ok(Self {
            service,
            payload,
        })
    }

    /// Parse ReadPropertyAck and extract property value
    pub fn parse_read_property_ack(&self) -> Option<BacnetValue> {
        if self.service != BacnetService::ReadPropertyAck {
            return None;
        }

        // Parse payload: [object_id(4) | property_id(2) | array_index(4) | value_tag(1) | length(1-4) | value...]
        let mut pos = 0;

        // Skip object identifier (4 bytes)
        pos += 4;

        // Skip property identifier (2 bytes)
        pos += 2;

        // Skip array index (4 bytes)
        pos += 4;

        // Check if there's more data for the property value
        if pos >= self.payload.len() {
            return None;
        }

        // Parse opening tag for property value
        let tag_byte = self.payload[pos];
        pos += 1;

        // Parse application tag (extract tag number from high bits)
        let tag_number = (tag_byte & 0xF8) >> 3;
        let is_context_tag = (tag_byte & 0x08) != 0;

        // Handle extended tag numbers
        let tag_number = if tag_number == 15 {
            // Extended tag
            if pos >= self.payload.len() {
                return None;
            }
            let ext_tag = self.payload[pos];
            pos += 1;
            ext_tag as u32
        } else {
            tag_number as u32
        };

        // Parse length
        let (length, len_size) = self.parse_length(&self.payload[pos..])?;
        pos += len_size;

        // Parse value based on tag
        if pos + length as usize > self.payload.len() {
            return None;
        }

        let value_data = &self.payload[pos..pos + length as usize];

        // Map application tags to BACnet types
        // Application Tag Numbers (from BACnet standard):
        // 0: Null, 1: Boolean, 2: Unsigned, 3: Signed, 4: Real, 5: Double,
        // 6: OctetString, 7: CharacterString, 8: BitString, 9: Enumerated,
        // 10: Date, 11: Time, 12: ObjectIdentifier

        let app_tag = if is_context_tag { ApplicationTag::Null } else {
            match tag_number {
                0 => ApplicationTag::Null,
                1 => ApplicationTag::Boolean,
                2 => ApplicationTag::Unsigned,
                3 => ApplicationTag::Signed,
                4 => ApplicationTag::Real,
                5 => ApplicationTag::Double,
                7 => ApplicationTag::CharacterString,
                9 => ApplicationTag::Enumerated,
                12 => ApplicationTag::ObjectIdentifier,
                _ => ApplicationTag::Null,
            }
        };

        self.parse_value_by_tag(app_tag, value_data)
    }

    /// Parse length field (can be 1-4 bytes)
    fn parse_length(&self, data: &[u8]) -> Option<(u32, usize)> {
        if data.is_empty() {
            return None;
        }

        let first_byte = data[0];

        if first_byte < 4 {
            // Length is in the byte itself
            Some((first_byte as u32, 1))
        } else if first_byte == 4 {
            // Four-byte length
            if data.len() < 5 {
                return None;
            }
            let length = u32::from_be_bytes([data[1], data[2], data[3], data[4]]);
            Some((length, 5))
        } else if first_byte == 5 {
            // Two-byte length
            if data.len() < 3 {
                return None;
            }
            let length = u16::from_be_bytes([data[1], data[2]]) as u32;
            Some((length, 3))
        } else {
            // Large length (not commonly used)
            None
        }
    }

    /// Parse value based on application tag
    fn parse_value_by_tag(&self, tag: ApplicationTag, data: &[u8]) -> Option<BacnetValue> {
        match tag {
            ApplicationTag::Null => Some(BacnetValue::Null),
            ApplicationTag::Boolean => {
                if !data.is_empty() {
                    Some(BacnetValue::Boolean(data[0] != 0))
                } else {
                    Some(BacnetValue::Boolean(false))
                }
            }
            ApplicationTag::Unsigned => {
                let len = data.len().min(4);
                let mut value: u32 = 0;
                for i in 0..len {
                    value |= (data[i] as u32) << (8 * (len - 1 - i));
                }
                Some(BacnetValue::Unsigned(value))
            }
            ApplicationTag::Signed => {
                let len = data.len().min(4);
                if len == 0 {
                    return Some(BacnetValue::Signed(0));
                }
                // Handle signed integers (two's complement)
                let is_negative = (data[0] & 0x80) != 0;
                let mut value: i32 = if is_negative { -1 } else { 0 };
                for i in 0..len {
                    let byte = data[i] as i32;
                    value = (value << 8) | byte;
                }
                Some(BacnetValue::Signed(value))
            }
            ApplicationTag::Real => {
                if data.len() >= 4 {
                    let bits = u32::from_be_bytes([data[0], data[1], data[2], data[3]]);
                    Some(BacnetValue::Real(f32::from_bits(bits)))
                } else {
                    None
                }
            }
            ApplicationTag::Double => {
                if data.len() >= 8 {
                    let bits = u64::from_be_bytes([
                        data[0], data[1], data[2], data[3],
                        data[4], data[5], data[6], data[7],
                    ]);
                    Some(BacnetValue::Double(f64::from_bits(bits)))
                } else {
                    None
                }
            }
            ApplicationTag::CharacterString => {
                // Parse character string: [encoding(1) | length(1) | characters...]
                if data.len() >= 2 {
                    let _encoding = data[0];
                    let str_len = data[1] as usize;
                    if data.len() >= 2 + str_len {
                        let string = String::from_utf8_lossy(&data[2..2 + str_len]).to_string();
                        Some(BacnetValue::String(string))
                    } else {
                        None
                    }
                } else {
                    None
                }
            }
            ApplicationTag::Enumerated => {
                let len = data.len().min(4);
                let mut value: u32 = 0;
                for i in 0..len {
                    value |= (data[i] as u32) << (8 * (len - 1 - i));
                }
                Some(BacnetValue::Unsigned(value))
            }
            ApplicationTag::ObjectIdentifier => {
                if data.len() >= 4 {
                    let encoded = u32::from_be_bytes([data[0], data[1], data[2], data[3]]);
                    ObjectIdentifier::decode(encoded).map(BacnetValue::ObjectIdentifier)
                } else {
                    None
                }
            }
            _ => None,
        }
    }
}

// BACnet Device Discovery
#[derive(Debug)]
pub struct BacnetDiscovery {
    pub devices: Vec<DiscoveredDevice>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveredDevice {
    pub device_id: u32,
    pub mac_address: u8,
    pub vendor_id: Option<u16>,
    pub object_name: Option<String>,
}

impl BacnetDiscovery {
    pub fn new() -> Self {
        Self {
            devices: Vec::new(),
        }
    }

    /// Send WhoIs request
    pub fn who_is(&self, target_device: Option<u32>) -> MstpFrame {
        // Create WhoIs APDU
        let mut payload = Vec::new();

        if let Some(device_id) = target_device {
            // Limited broadcast
            let low_id = device_id & 0xFFFF;
            let high_id = (device_id >> 16) & 0xFFFF;
            payload.extend_from_slice(&low_id.to_be_bytes());
            payload.extend_from_slice(&high_id.to_be_bytes());
        } else {
            // Broadcast
            payload.extend_from_slice(&0xFFFF_u16.to_be_bytes());
            payload.extend_from_slice(&0x0000_u16.to_be_bytes());
        }

        // Create BACnet data frame (broadcast to all)
        let destination = 0xFF; // Broadcast
        let source = 1; // TODO: Use actual station address

        MstpFrame::data_no_reply(destination, source, payload)
    }

    /// Process IAm response
    pub fn process_i_am(&mut self, frame: &MstpFrame) -> Option<DiscoveredDevice> {
        if frame.source == 0xFF || frame.source == 0 {
            return None;
        }

        // Parse IAm APDU
        if let Ok(apdu) = Apdu::decode(&frame.data) {
            if apdu.service == BacnetService::IAm {
                // TODO: Parse device info from APDU
                return Some(DiscoveredDevice {
                    device_id: 0, // Extract from APDU
                    mac_address: frame.source,
                    vendor_id: None,
                    object_name: None,
                });
            }
        }

        None
    }
}

// Unit Tests
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_object_type_conversion() {
        assert_eq!(BacnetObjectType::AnalogInput.as_str(), "analogInput");
        assert_eq!(BacnetObjectType::from_str("analoginput"), Some(BacnetObjectType::AnalogInput));
    }

    #[test]
    fn test_object_identifier() {
        let obj_id = ObjectIdentifier::new(BacnetObjectType::AnalogInput, 100);
        let encoded = obj_id.encode();
        let decoded = ObjectIdentifier::decode(encoded).unwrap();
        assert_eq!(decoded.object_type, BacnetObjectType::AnalogInput);
        assert_eq!(decoded.instance, 100);
    }

    #[test]
    fn test_property_conversion() {
        assert_eq!(BacnetProperty::PresentValue.as_str(), "presentValue");
        assert_eq!(BacnetProperty::from_str("presentvalue"), Some(BacnetProperty::PresentValue));
    }
}
