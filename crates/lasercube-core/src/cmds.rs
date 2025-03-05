//! Command definitions for LaserCube protocol.

use crate::{LaserInfo, LaserInfoParseError, Point};
use std::convert::TryFrom;
use thiserror::Error;

/// Command types supported by the LaserCube protocol.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum CommandType {
    /// Get detailed device information.
    GetFullInfo = 0x77,
    /// Enable/disable buffer size responses on data packets.
    EnableBufferSizeResponseOnData = 0x78,
    /// Enable/disable laser output.
    SetOutput = 0x80,
    /// Get the number of free samples in the device's ring buffer.
    GetRingbufferEmptySampleCount = 0x8a,
    /// Send point data to render.
    SampleData = 0xa9,
}

/// Command structure for the LaserCube protocol.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    /// Get detailed device information.
    GetFullInfo,
    /// Enable/disable buffer size responses on data packets.
    EnableBufferSizeResponseOnData(bool),
    /// Enable/disable laser output.
    SetOutput(bool),
    /// Get the number of free samples in the device's ring buffer.
    GetRingbufferEmptySampleCount,
    /// Send point data to render.
    SampleData(SampleData),
}

/// Send point data to render.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SampleData {
    /// Message sequence number (0-255)
    pub message_num: u8,
    /// Frame sequence number (0-255)
    pub frame_num: u8,
    /// Point data to render
    pub points: Vec<Point>,
}

/// Responses from LaserCube device
#[derive(Debug, Clone, PartialEq)]
pub enum Response {
    /// Full device information
    FullInfo(LaserInfo),
    /// Buffer free space
    BufferFree(u16),
    /// Simple acknowledgment
    Ack,
}

/// Error types that can occur when parsing command responses
#[derive(Debug, Error)]
pub enum ResponseParseError {
    #[error("Empty response")]
    EmptyResponse,
    #[error("Unknown command type: {0}")]
    UnknownCommandType(u8),
    #[error("Response too short for {command_type:?} command: expected at least {expected} bytes, got {actual}")]
    ResponseTooShort {
        command_type: CommandType,
        expected: usize,
        actual: usize,
    },
    #[error("Failed to parse LaserInfo: {0}")]
    LaserInfoError(#[from] LaserInfoParseError),
}

impl TryFrom<&[u8]> for Response {
    type Error = ResponseParseError;

    fn try_from(bytes: &[u8]) -> Result<Self, Self::Error> {
        if bytes.is_empty() {
            return Err(ResponseParseError::EmptyResponse);
        }

        // First byte is the command type
        let cmd_type = match CommandType::try_from(bytes[0]) {
            Ok(cmd) => cmd,
            Err(_) => return Err(ResponseParseError::UnknownCommandType(bytes[0])),
        };

        match cmd_type {
            CommandType::GetFullInfo => {
                // Parse the LaserInfo using its TryFrom implementation
                let laser_info = LaserInfo::try_from(bytes)?;
                Ok(Response::FullInfo(laser_info))
            }

            CommandType::GetRingbufferEmptySampleCount => {
                let minimum_len = 4;
                if bytes.len() < minimum_len {
                    return Err(ResponseParseError::ResponseTooShort {
                        command_type: cmd_type,
                        expected: minimum_len,
                        actual: bytes.len(),
                    });
                }

                let buffer_free = u16::from_le_bytes([bytes[2], bytes[3]]);
                Ok(Response::BufferFree(buffer_free))
            }

            // Data packets can respond with buffer info when enabled
            CommandType::SampleData => {
                let minimum_len = 3;
                if bytes.len() < minimum_len {
                    return Err(ResponseParseError::ResponseTooShort {
                        command_type: cmd_type,
                        expected: minimum_len,
                        actual: bytes.len(),
                    });
                }

                // The response includes the free buffer space
                let buffer_free = u16::from_le_bytes([bytes[1], bytes[2]]);
                Ok(Response::BufferFree(buffer_free))
            }

            // Acknowledgment responses
            CommandType::EnableBufferSizeResponseOnData | CommandType::SetOutput => {
                Ok(Response::Ack)
            }
        }
    }
}

impl TryFrom<u8> for CommandType {
    type Error = ();
    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0x77 => Ok(CommandType::GetFullInfo),
            0x78 => Ok(CommandType::EnableBufferSizeResponseOnData),
            0x80 => Ok(CommandType::SetOutput),
            0x8a => Ok(CommandType::GetRingbufferEmptySampleCount),
            0xa9 => Ok(CommandType::SampleData),
            _ => Err(()),
        }
    }
}

impl Command {
    /// Get the command type associated with this command.
    pub fn command_type(&self) -> CommandType {
        match self {
            Command::GetFullInfo => CommandType::GetFullInfo,
            Command::EnableBufferSizeResponseOnData(_) => {
                CommandType::EnableBufferSizeResponseOnData
            }
            Command::SetOutput(_) => CommandType::SetOutput,
            Command::GetRingbufferEmptySampleCount => CommandType::GetRingbufferEmptySampleCount,
            Command::SampleData { .. } => CommandType::SampleData,
        }
    }

    /// Estimate the size in bytes this command will take when serialized.
    pub fn size(&self) -> usize {
        match self {
            Command::GetFullInfo => 1,
            Command::EnableBufferSizeResponseOnData(_) => 2,
            Command::SetOutput(_) => 2,
            Command::GetRingbufferEmptySampleCount => 1,
            Command::SampleData(SampleData { points, .. }) => {
                // 1 byte command
                // + 1 byte padding
                // + 1 byte message num
                // + 1 byte frame num
                4 + (points.len() * 10) // Each point is 10 bytes
            }
        }
    }

    /// Write this command into the provided byte buffer.
    ///
    /// Returns the number of bytes written.
    pub fn write_bytes(&self, buffer: &mut Vec<u8>) -> usize {
        let start_len = buffer.len();

        match self {
            Command::GetFullInfo => {
                buffer.push(CommandType::GetFullInfo as u8);
            }

            Command::EnableBufferSizeResponseOnData(enable) => {
                buffer.push(CommandType::EnableBufferSizeResponseOnData as u8);
                buffer.push(if *enable { 1 } else { 0 });
            }

            Command::SetOutput(enable) => {
                buffer.push(CommandType::SetOutput as u8);
                buffer.push(if *enable { 1 } else { 0 });
            }

            Command::GetRingbufferEmptySampleCount => {
                buffer.push(CommandType::GetRingbufferEmptySampleCount as u8);
            }

            Command::SampleData(data) => {
                // Header: command byte, 0x00, message_num, frame_num
                buffer.push(CommandType::SampleData as u8);
                buffer.push(0x00); // Always 0x00 according to protocol
                buffer.push(data.message_num);
                buffer.push(data.frame_num);

                // Append each point's serialized bytes
                for point in &data.points {
                    let point_bytes: [u8; Point::SIZE] = (*point).into();
                    buffer.extend_from_slice(&point_bytes);
                }
            }
        }

        buffer.len() - start_len
    }

    /// Convenience method to get command bytes as a new Vec<u8>
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut buffer = Vec::with_capacity(self.size());
        self.write_bytes(&mut buffer);
        buffer
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_buffer_free_response() {
        // Sample response for GetRingbufferEmptySampleCount with 1000 free samples
        let response = [0x8a, 0x00, 0xe8, 0x03]; // 0x03e8 = 1000 in little-endian

        let parsed = Response::try_from(&response[..]).unwrap();

        match parsed {
            Response::BufferFree(free) => assert_eq!(free, 1000),
            _ => panic!("Wrong response type parsed"),
        }
    }

    #[test]
    fn test_parse_ack_response() {
        // Sample response for SetOutput
        let response = [0x80];

        let parsed = Response::try_from(&response[..]).unwrap();

        match parsed {
            Response::Ack => {}
            _ => panic!("Wrong response type parsed"),
        }
    }

    #[test]
    fn test_parse_error_handling() {
        // Empty response
        let result = Response::try_from(&[][..]);
        assert!(matches!(result, Err(ResponseParseError::EmptyResponse)));

        // Unknown command type
        let result = Response::try_from(&[0xFF][..]);
        assert!(matches!(
            result,
            Err(ResponseParseError::UnknownCommandType(0xFF))
        ));

        // Response too short
        let result = Response::try_from(&[0x8a, 0x00][..]);
        assert!(matches!(
            result,
            Err(ResponseParseError::ResponseTooShort {
                command_type: CommandType::GetRingbufferEmptySampleCount,
                ..
            })
        ));
    }
}
