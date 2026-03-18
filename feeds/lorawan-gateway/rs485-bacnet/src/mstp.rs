// MS/TP Frame Implementation
//
// Implements BACnet MS/TP (Master-Slave/Token-Passing) data link layer
// Based on ANSI/ASHRAE 135-2012

// MS/TP Frame Type Constants
pub const FRAME_TYPE_TOKEN: u8 = 0x00;
pub const FRAME_TYPE_POLL_FOR_MASTER: u8 = 0x01;
pub const FRAME_TYPE_REPLY_TO_POLL: u8 = 0x02;
pub const FRAME_TYPE_TEST_REQUEST: u8 = 0x03;
pub const FRAME_TYPE_TEST_RESPONSE: u8 = 0x04;
pub const FRAME_TYPE_BACNET_DATA_EXPECTING_REPLY: u8 = 0x05;
pub const FRAME_TYPE_BACNET_DATA_NOT_EXPECTING_REPLY: u8 = 0x06;

// MS/TP Frame Structure
#[derive(Debug, Clone, PartialEq)]
pub struct MstpFrame {
    pub frame_type: u8,
    pub destination: u8,
    pub source: u8,
    pub length: u16,
    pub data: Vec<u8>,
}

impl MstpFrame {
    /// Create a new MS/TP frame
    pub fn new(frame_type: u8, destination: u8, source: u8, data: Vec<u8>) -> Self {
        let length = 2 + data.len() as u16; // Header (2) + data length
        Self {
            frame_type,
            destination,
            source,
            length,
            data,
        }
    }

    /// Create a token frame
    pub fn token(destination: u8, source: u8) -> Self {
        Self::new(FRAME_TYPE_TOKEN, destination, source, vec![])
    }

    /// Create a BACnet data frame expecting reply
    pub fn data_expect_reply(destination: u8, source: u8, data: Vec<u8>) -> Self {
        Self::new(FRAME_TYPE_BACNET_DATA_EXPECTING_REPLY, destination, source, data)
    }

    /// Create a BACnet data frame not expecting reply
    pub fn data_no_reply(destination: u8, source: u8, data: Vec<u8>) -> Self {
        Self::new(FRAME_TYPE_BACNET_DATA_NOT_EXPECTING_REPLY, destination, source, data)
    }

    /// Encode frame to bytes (without CRC)
    pub fn encode(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(8 + self.data.len());
        bytes.push(self.frame_type);
        bytes.push(self.destination);
        bytes.push(self.source);
        bytes.extend_from_slice(&self.length.to_be_bytes());
        bytes.extend_from_slice(&self.data);
        bytes
    }

    /// Encode frame with CRC
    pub fn encode_with_crc(&self) -> Vec<u8> {
        let mut bytes = self.encode();
        let crc = Self::calculate_crc(&bytes);
        bytes.extend_from_slice(&crc.to_le_bytes());
        bytes
    }

    /// Decode frame from bytes (without CRC)
    pub fn decode(bytes: &[u8]) -> Result<Self, MstpError> {
        if bytes.len() < 8 {
            return Err(MstpError::InvalidFrameLength);
        }

        let frame_type = bytes[0];
        let destination = bytes[1];
        let source = bytes[2];
        let length = u16::from_be_bytes([bytes[3], bytes[4]]);

        if bytes.len() < (5 + length) as usize {
            return Err(MstpError::InvalidDataLength);
        }

        let data_start = 5;
        let data_end = 5 + length as usize - 2; // Exclude CRC
        let data = bytes[data_start..data_end].to_vec();

        Ok(Self {
            frame_type,
            destination,
            source,
            length,
            data,
        })
    }

    /// Decode frame from bytes with CRC verification
    pub fn decode_with_crc(bytes: &[u8]) -> Result<Self, MstpError> {
        if bytes.len() < 10 {
            return Err(MstpError::InvalidFrameLength);
        }

        let frame_data = &bytes[..bytes.len()-2];
        let crc_received = u16::from_le_bytes([bytes[bytes.len()-2], bytes[bytes.len()-1]]);
        let crc_calculated = Self::calculate_crc(frame_data);

        if crc_received != crc_calculated {
            return Err(MstpError::InvalidCrc);
        }

        Self::decode(frame_data)
    }

    /// Calculate CRC-16 (ARC) for MS/TP
    pub fn calculate_crc(data: &[u8]) -> u16 {
        let mut crc: u16 = 0xFFFF;
        for &byte in data {
            crc ^= u16::from(byte);
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

    /// Get frame type name
    pub fn frame_type_name(&self) -> &'static str {
        match self.frame_type {
            FRAME_TYPE_TOKEN => "Token",
            FRAME_TYPE_POLL_FOR_MASTER => "PollForMaster",
            FRAME_TYPE_REPLY_TO_POLL => "ReplyToPoll",
            FRAME_TYPE_TEST_REQUEST => "TestRequest",
            FRAME_TYPE_TEST_RESPONSE => "TestResponse",
            FRAME_TYPE_BACNET_DATA_EXPECTING_REPLY => "BACnetDataExpectReply",
            FRAME_TYPE_BACNET_DATA_NOT_EXPECTING_REPLY => "BACnetDataNoReply",
            _ => "Unknown",
        }
    }

    /// Check if this is a token frame
    pub fn is_token(&self) -> bool {
        self.frame_type == FRAME_TYPE_TOKEN
    }

    /// Check if this is a BACnet data frame
    pub fn is_bacnet_data(&self) -> bool {
        self.frame_type == FRAME_TYPE_BACNET_DATA_EXPECTING_REPLY ||
        self.frame_type == FRAME_TYPE_BACNET_DATA_NOT_EXPECTING_REPLY
    }

    /// Check if this frame expects a reply
    pub fn expects_reply(&self) -> bool {
        self.frame_type == FRAME_TYPE_BACNET_DATA_EXPECTING_REPLY
    }
}

// MS/TP Error Types
#[derive(Debug, Clone, PartialEq)]
pub enum MstpError {
    InvalidFrameLength,
    InvalidDataLength,
    InvalidCrc,
    InvalidFrameType,
    Timeout,
    Collision,
}

impl std::fmt::Display for MstpError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            MstpError::InvalidFrameLength => write!(f, "Invalid frame length"),
            MstpError::InvalidDataLength => write!(f, "Invalid data length"),
            MstpError::InvalidCrc => write!(f, "CRC check failed"),
            MstpError::InvalidFrameType => write!(f, "Invalid frame type"),
            MstpError::Timeout => write!(f, "Timeout"),
            MstpError::Collision => write!(f, "Collision detected"),
        }
    }
}

impl std::error::Error for MstpError {}

// MS/TP Timing Parameters (in milliseconds)
#[derive(Debug, Clone, Copy)]
pub struct MstpTiming {
    pub treply_timeout: u64,
    pub ttoken_hold_time: u64,
    pub tframe_abort: u64,
    pub tturnaround: u64,
    pub tusage_timeout: u64,
}

impl Default for MstpTiming {
    fn default() -> Self {
        Self {
            treply_timeout: 200,     // 200ms
            ttoken_hold_time: 10,     // 10ms
            tframe_abort: 60,         // 60ms
            tturnaround: 40,          // 40ms
            tusage_timeout: 255,      // 255ms
        }
    }
}

// MS/TP Device State
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MstpState {
    Idle,
    UseToken,
    WaitForReply,
    Done,
}

// MS/TP Master State Machine
#[derive(Debug)]
pub struct MstpMaster {
    pub this_station: u8,
    pub max_master: u8,
    pub max_info_frames: u8,
    pub state: MstpState,
    pub timing: MstpTiming,
    pub token_count: u8,
}

impl MstpMaster {
    pub fn new(this_station: u8, max_master: u8, max_info_frames: u8) -> Self {
        Self {
            this_station,
            max_master,
            max_info_frames,
            state: MstpState::Idle,
            timing: MstpTiming::default(),
            token_count: 0,
        }
    }

    /// Process received frame
    pub fn receive_frame(&mut self, frame: &MstpFrame) -> Result<(), MstpError> {
        match frame.frame_type {
            FRAME_TYPE_TOKEN => {
                if frame.destination == self.this_station {
                    // Token received - we can use it
                    self.state = MstpState::UseToken;
                    self.token_count = 0;
                    log::debug!("Token received for station {}", self.this_station);
                } else if frame.destination == 0xFF {
                    // Poll for master frame
                    // TODO: Send reply to poll
                }
                // Pass token to next station
                Ok(())
            }
            FRAME_TYPE_BACNET_DATA_EXPECTING_REPLY => {
                if frame.destination == self.this_station {
                    // Data frame for us - process it
                    self.state = MstpState::WaitForReply;
                    // TODO: Process BACnet data
                    Ok(())
                } else {
                    // Not for us - ignore
                    Ok(())
                }
            }
            _ => Ok(()),
        }
    }

    /// Send token to next station
    pub fn send_token(&self) -> MstpFrame {
        let next_station = if self.this_station >= self.max_master {
            1
        } else {
            self.this_station + 1
        };
        MstpFrame::token(next_station, self.this_station)
    }

    /// Check if we have permission to send
    pub fn can_send(&self) -> bool {
        self.state == MstpState::UseToken && self.token_count < self.max_info_frames
    }

    /// Mark that we've sent a frame
    pub fn frame_sent(&mut self) {
        self.token_count += 1;
        if self.token_count >= self.max_info_frames {
            // Must pass token
            self.state = MstpState::Idle;
        }
    }

    /// Pass token to next station
    pub fn pass_token(&mut self) -> MstpFrame {
        self.state = MstpState::Idle;
        self.send_token()
    }
}

// Unit Tests
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_crc_calculation() {
        let data = vec![0x00, 0x01, 0x02, 0x00, 0x03];
        let crc = MstpFrame::calculate_crc(&data);
        assert_ne!(crc, 0);
    }

    #[test]
    fn test_token_frame() {
        let frame = MstpFrame::token(1, 0);
        assert_eq!(frame.frame_type, FRAME_TYPE_TOKEN);
        assert_eq!(frame.destination, 1);
        assert_eq!(frame.source, 0);
        assert!(frame.is_token());
        assert!(!frame.is_bacnet_data());
    }

    #[test]
    fn test_data_frame() {
        let data = vec![0x01, 0x02, 0x03];
        let frame = MstpFrame::data_expect_reply(5, 1, data.clone());
        assert_eq!(frame.frame_type, FRAME_TYPE_BACNET_DATA_EXPECTING_REPLY);
        assert_eq!(frame.destination, 5);
        assert_eq!(frame.source, 1);
        assert!(frame.is_bacnet_data());
        assert!(frame.expects_reply());
    }

    #[test]
    fn test_encode_decode() {
        let original = MstpFrame::token(10, 1);
        let encoded = original.encode();
        let decoded = MstpFrame::decode(&encoded).unwrap();
        assert_eq!(decoded.frame_type, original.frame_type);
        assert_eq!(decoded.destination, original.destination);
        assert_eq!(decoded.source, original.source);
    }
}
