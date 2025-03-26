//! Core types and constants for the LaserCube network protocol.
//!
//! This crate provides the fundamental data structures and protocol definitions
//! for communicating with LaserCube devices, without any actual network implementation.

pub mod buffer;
pub mod cmds;
pub mod point;
pub mod status;

// Re-export commonly used types
pub use buffer::BufferState;
pub use cmds::{Command, CommandType, SampleData};
pub use point::Point;
pub use status::StatusFlags;
use std::{convert::TryFrom, ffi::CStr, net::Ipv4Addr};
use thiserror::Error;

/// Ports that the device listens on.
pub mod port {
    /// Port for "alive" messages (simple pings).
    pub const ALIVE: u16 = 45456;
    /// Port for commands (get info, enable/disable output, etc.).
    pub const CMD: u16 = 45457;
    /// Port for point data transmission.
    pub const DATA: u16 = 45458;
}

/// Maximum points per data message to stay under typical network MTU.
pub const MAX_POINTS_PER_MESSAGE: usize = 140;

/// Default broadcast address
pub const DEFAULT_BROADCAST_ADDR: &str = "255.255.255.255";

/// Connection type for the LaserCube.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ConnectionType {
    /// Unknown connection type.
    Unknown = 0,
    /// Connected via USB.
    Usb = 1,
    /// Connected via Ethernet.
    Ethernet = 2,
    /// Connected via Wifi.
    Wifi = 3,
}

/// Error types that can occur when parsing a LaserInfo response
#[derive(Debug, Error)]
pub enum LaserInfoParseError {
    #[error("Response too short: expected at least {expected} bytes, got {actual}")]
    ResponseTooShort { expected: usize, actual: usize },
    #[error("Missing null terminator in model name: {0}")]
    MissingNullTerminator(#[from] std::ffi::FromBytesUntilNulError),
}

/// Fixed-size header portion of the LaserInfo response
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LaserInfoHeader {
    /// Firmware major version
    pub fw_major: u8,
    /// Firmware minor version
    pub fw_minor: u8,
    /// Status flags containing output enabled, interlock status, etc.
    pub status: StatusFlags,
    /// Current DAC rate
    pub dac_rate: u32,
    /// Maximum DAC rate
    pub max_dac_rate: u32,
    /// Current free space in the RX buffer
    pub rx_buffer_free: u16,
    /// Total size of the RX buffer
    pub rx_buffer_size: u16,
    /// Battery percentage
    pub battery_percent: u8,
    /// Device temperature
    pub temperature: u8,
    /// Model number
    pub model_number: u8,
    /// The type of connection (usb, ethernet, wifi, unknown).
    pub conn_type: ConnectionType,
    /// Serial number
    pub serial_number: [u8; 6],
    /// IP address
    pub ip_addr: Ipv4Addr,
}

/// The fixed-size header along with the variable length model name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LaserInfo {
    /// Fixed-size header fields
    pub header: LaserInfoHeader,
    /// Model name (variable length, no greater than 26 bytes).
    pub model_name: String,
}

impl LaserInfoHeader {
    /// The size of the header encoded as bytes.
    pub const SIZE: usize = 38;

    /// Get whether interlock is enabled.
    pub fn interlock_enabled(&self) -> bool {
        self.status.interlock_enabled(self.fw_major, self.fw_minor)
    }

    /// Get whether a temperature warning is active.
    pub fn temperature_warning(&self) -> bool {
        self.status
            .temperature_warning(self.fw_major, self.fw_minor)
    }

    /// Get whether an over-temperature condition is active.
    pub fn over_temperature(&self) -> bool {
        self.status.over_temperature(self.fw_major, self.fw_minor)
    }
}

impl LaserInfo {
    /// The minimum size of the `LaserInfo` in bytes.
    pub const MIN_SIZE: usize = LaserInfoHeader::SIZE;
    /// The maximum size of the `LaserInfo` in bytes.
    pub const MAX_SIZE: usize = 64;
    /// The maximum size of the `LaserInfo`'s model name field in bytes.
    pub const MAX_MODEL_NAME_SIZE: usize = Self::MAX_SIZE - Self::MIN_SIZE;

    /// Get the firmware version as a string (e.g., "1.2")
    pub fn firmware_version(&self) -> String {
        format!("{}.{}", self.header.fw_major, self.header.fw_minor)
    }

    /// Get the serial number as a formatted string (XX:XX:XX:XX:XX:XX)
    pub fn serial_number_string(&self) -> String {
        let mut result = String::with_capacity(17);
        for (i, byte) in self.header.serial_number.iter().enumerate() {
            if i > 0 {
                result.push(':');
            }
            use std::fmt::Write;
            write!(result, "{:02x}", byte).unwrap();
        }
        result
    }
}

impl From<u8> for ConnectionType {
    fn from(value: u8) -> Self {
        match value {
            1 => ConnectionType::Usb,
            2 => ConnectionType::Ethernet,
            3 => ConnectionType::Wifi,
            _ => ConnectionType::Unknown,
        }
    }
}

impl From<[u8; 38]> for LaserInfoHeader {
    fn from(bytes: [u8; 38]) -> Self {
        #[rustfmt::skip]
        let [
            _cmd_echo,                      // 0: Command echo
            _,                              // 1: Padding
            _,                              // 2: Padding
            fw_major,                       // 3
            fw_minor,                       // 4
            status_flags,                   // 5
            _, _, _, _,                     // 6-9: 4 bytes padding
            dr0, dr1, dr2, dr3,             // 10-13: DAC rate
            mdr0, mdr1, mdr2, mdr3,         // 14-17: Max DAC rate
            _,                              // 18: Padding
            rxbf0, rxbf1,                   // 19-20: RX buffer free
            rxbs0, rxbs1,                   // 21-22: RX buffer size
            battery_percent,                // 23
            temperature,                    // 24
            conn_type,                      // 25
            sn0, sn1, sn2, sn3, sn4, sn5,   // 26-31: Serial number
            ip0, ip1, ip2, ip3,             // 32-35: IP address
            _,                              // 36: Padding
            model_number,                   // 37
        ] = bytes;

        Self {
            fw_major,
            fw_minor,
            status: StatusFlags::from_bits_retain(status_flags),
            dac_rate: u32::from_le_bytes([dr0, dr1, dr2, dr3]),
            max_dac_rate: u32::from_le_bytes([mdr0, mdr1, mdr2, mdr3]),
            rx_buffer_free: u16::from_le_bytes([rxbf0, rxbf1]),
            rx_buffer_size: u16::from_le_bytes([rxbs0, rxbs1]),
            battery_percent,
            temperature,
            conn_type: conn_type.into(),
            serial_number: [sn0, sn1, sn2, sn3, sn4, sn5],
            ip_addr: [ip0, ip1, ip2, ip3].into(),
            model_number,
        }
    }
}

impl TryFrom<&[u8]> for LaserInfo {
    type Error = LaserInfoParseError;

    fn try_from(bytes: &[u8]) -> Result<Self, Self::Error> {
        // Need at least 38 bytes for the header
        let header_bytes: &[u8; LaserInfoHeader::SIZE] = bytes
            .get(0..LaserInfoHeader::SIZE)
            .and_then(|slice| slice.try_into().ok())
            .ok_or_else(|| LaserInfoParseError::ResponseTooShort {
                expected: LaserInfoHeader::SIZE,
                actual: bytes.len(),
            })?;
        // Parse the fixed header portion
        let header = LaserInfoHeader::from(*header_bytes);
        // Model name is a null-terminated string starting after the fixed region.
        let model_name_start = LaserInfoHeader::SIZE;
        let model_name_cstr = CStr::from_bytes_until_nul(&bytes[model_name_start..])?;
        let model_name = String::from_utf8_lossy(model_name_cstr.to_bytes()).to_string();
        Ok(LaserInfo { header, model_name })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_laser_info_header() {
        // Create a test header array
        let mut header = [0u8; LaserInfoHeader::SIZE];

        // Set specific fields
        header[3] = 1; // fw_major
        header[4] = 2; // fw_minor
        header[5] = 0x01; // status_flags: output_enabled bit set

        // DAC rate (6000)
        header[10] = 0x70;
        header[11] = 0x17;
        header[12] = 0;
        header[13] = 0;

        // Max DAC rate (6000)
        header[14] = 0x70;
        header[15] = 0x17;
        header[16] = 0;
        header[17] = 0;

        // Buffer info
        header[19] = 0x88; // 5000 (low byte)
        header[20] = 0x13; // 5000 (high byte)
        header[21] = 0x70; // 6000 (low byte)
        header[22] = 0x17; // 6000 (high byte)

        // Status
        header[23] = 100; // battery 100%
        header[24] = 31; // temperature 31°C

        // Connection type
        header[25] = 2;

        // Serial number (offset 26-31)
        header[26] = 1;
        header[27] = 2;
        header[28] = 3;
        header[29] = 4;
        header[30] = 5;
        header[31] = 6;

        // IP address (offset 32-35)
        header[32] = 192;
        header[33] = 168;
        header[34] = 1;
        header[35] = 100;

        // Model number
        header[37] = 1;

        let info_header = LaserInfoHeader::from(header);

        assert_eq!(info_header.fw_major, 1);
        assert_eq!(info_header.fw_minor, 2);
        assert!(info_header.status.output_enabled());
        assert_eq!(info_header.status.output_enabled(), true);
        assert_eq!(info_header.dac_rate, 6000);
        assert_eq!(info_header.max_dac_rate, 6000);
        assert_eq!(info_header.rx_buffer_free, 5000);
        assert_eq!(info_header.rx_buffer_size, 6000);
        assert_eq!(info_header.battery_percent, 100);
        assert_eq!(info_header.temperature, 31);
        assert_eq!(info_header.model_number, 1);
        assert_eq!(info_header.conn_type, ConnectionType::Ethernet);
        assert_eq!(info_header.serial_number, [1, 2, 3, 4, 5, 6]); // First byte is 2 for Ethernet
        assert_eq!(info_header.ip_addr, Ipv4Addr::from([192, 168, 1, 100]));
    }

    #[test]
    fn test_parse_status_flags() {
        // Create a test header array with different status flags
        let mut header = [0u8; LaserInfoHeader::SIZE];

        // Set firmware version to 0.13 (new flag layout)
        header[3] = 0; // fw_major
        header[4] = 13; // fw_minor

        // Set status flags with multiple bits
        // 0x2F = 0b0010_1111
        // - Output enabled
        // - Interlock enabled (v0.13)
        // - Temperature warning (v0.13)
        // - Over temperature (v0.13)
        // - Packet errors = 2
        header[5] = 0x2F;

        let info_header = LaserInfoHeader::from(header);

        assert_eq!(info_header.fw_major, 0);
        assert_eq!(info_header.fw_minor, 13);
        assert!(info_header.status.output_enabled());
        assert!(info_header.interlock_enabled());
        assert!(info_header.temperature_warning());
        assert!(info_header.over_temperature());
        assert_eq!(info_header.status.packet_errors(), 2);

        // Now test with older firmware version (0.12) and different flag layout
        header[4] = 12; // fw_minor
                        // 0x39 = 0b0011_1001
                        // - Output enabled
                        // - Interlock enabled (v0.12) at bit 3
                        // - Temperature warning (v0.12) at bit 4
                        // - Over temperature (v0.12) at bit 5
        header[5] = 0x39;

        let info_header = LaserInfoHeader::from(header);

        assert_eq!(info_header.fw_major, 0);
        assert_eq!(info_header.fw_minor, 12);
        assert!(info_header.status.output_enabled());
        assert!(info_header.interlock_enabled());
        assert!(info_header.temperature_warning());
        assert!(info_header.over_temperature());
    }

    #[test]
    fn test_parse_laser_info_with_header() {
        // Create a test header array
        let mut message = [0u8; 80]; // 64 byte header plus model name and null terminator

        // Fill header with test values
        message[0] = 0x77; // Command byte
        message[3] = 1; // fw_major
        message[4] = 2; // fw_minor
        message[5] = 0x01; // status: output_enabled bit set

        // DAC rate (6000)
        message[10] = 0x70;
        message[11] = 0x17;
        message[12] = 0;
        message[13] = 0;

        // Max DAC rate (6000)
        message[14] = 0x70;
        message[15] = 0x17;
        message[16] = 0;
        message[17] = 0;

        // Buffer info
        message[19] = 0x88; // 5000 (low byte)
        message[20] = 0x13; // 5000 (high byte)
        message[21] = 0x70; // 6000 (low byte)
        message[22] = 0x17; // 6000 (high byte)

        // Status
        message[23] = 100; // battery 100%
        message[24] = 31; // temperature 31°C

        // Connection type (ethernet)
        message[25] = 2;

        // Serial number (offset 26-31)
        message[26] = 1;
        message[27] = 2;
        message[28] = 3;
        message[29] = 4;
        message[30] = 5;
        message[31] = 6;

        // IP address (offset 32-35)
        message[32] = 192;
        message[33] = 168;
        message[34] = 1;
        message[35] = 100;

        // Model number
        message[37] = 1;

        // Model name starting at offset 38
        let model_name = b"LaserCube Pro";
        let name_offset = 38;
        for (i, &byte) in model_name.iter().enumerate() {
            message[name_offset + i] = byte;
        }
        message[name_offset + model_name.len()] = 0; // Null terminator

        let laser_info = LaserInfo::try_from(&message[..]).unwrap();

        assert_eq!(laser_info.header.fw_major, 1);
        assert_eq!(laser_info.header.fw_minor, 2);
        assert_eq!(laser_info.header.status.output_enabled(), true);
        assert_eq!(laser_info.header.dac_rate, 6000);
        assert_eq!(laser_info.header.max_dac_rate, 6000);
        assert_eq!(laser_info.header.rx_buffer_free, 5000);
        assert_eq!(laser_info.header.rx_buffer_size, 6000);
        assert_eq!(laser_info.header.battery_percent, 100);
        assert_eq!(laser_info.header.temperature, 31);
        assert_eq!(laser_info.header.conn_type, ConnectionType::Ethernet);
        assert_eq!(laser_info.header.model_number, 1);
        assert_eq!(laser_info.header.serial_number, [1, 2, 3, 4, 5, 6]);
        assert_eq!(
            laser_info.header.ip_addr,
            Ipv4Addr::from([192, 168, 1, 100])
        );
        assert_eq!(laser_info.model_name, "LaserCube Pro");
    }
}
