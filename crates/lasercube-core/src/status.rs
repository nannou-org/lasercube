use bitflags::bitflags;

bitflags! {
    /// Status flags for the LaserCube device (byte 5 of the full info response).
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct StatusFlags: u8 {
        /// Output is enabled
        const OUTPUT_ENABLED = 0b0000_0001;
        /// Interlock is enabled (firmware version >= 0.13)
        const INTERLOCK_ENABLED_V013 = 0b0000_0010;
        /// Temperature warning (firmware version >= 0.13)
        const TEMPERATURE_WARNING_V013 = 0b0000_0100;
        /// Over temperature condition (firmware version >= 0.13)
        const OVER_TEMPERATURE_V013 = 0b0000_1000;
        /// Packet errors mask (firmware version >= 0.13, upper 4 bits)
        const PACKET_ERRORS_MASK = 0b1111_0000;

        // Legacy flag definitions for firmware version <= 0.12

        /// Interlock is enabled (firmware version <= 0.12)
        const INTERLOCK_ENABLED_V012 = 0b0000_1000;
        /// Temperature warning (firmware version <= 0.12)
        const TEMPERATURE_WARNING_V012 = 0b0001_0000;
        /// Over temperature condition (firmware version <= 0.12)
        const OVER_TEMPERATURE_V012 = 0b0010_0000;
    }
}

impl StatusFlags {
    /// Get whether output is enabled.
    pub fn output_enabled(self) -> bool {
        self.contains(Self::OUTPUT_ENABLED)
    }

    /// Get whether interlock is enabled, handling firmware version differences.
    pub fn interlock_enabled(self, fw_major: u8, fw_minor: u8) -> bool {
        if fw_major > 0 || fw_minor >= 13 {
            self.contains(Self::INTERLOCK_ENABLED_V013)
        } else {
            self.contains(Self::INTERLOCK_ENABLED_V012)
        }
    }

    /// Get whether there's a temperature warning, handling firmware version
    /// differences.
    pub fn temperature_warning(self, fw_major: u8, fw_minor: u8) -> bool {
        if fw_major > 0 || fw_minor >= 13 {
            self.contains(Self::TEMPERATURE_WARNING_V013)
        } else {
            self.contains(Self::TEMPERATURE_WARNING_V012)
        }
    }

    /// Get whether there's an over-temperature condition, handling firmware
    /// version differences.
    pub fn over_temperature(self, fw_major: u8, fw_minor: u8) -> bool {
        if fw_major > 0 || fw_minor >= 13 {
            self.contains(Self::OVER_TEMPERATURE_V013)
        } else {
            self.contains(Self::OVER_TEMPERATURE_V012)
        }
    }

    /// Get the packet errors count (firmware version >= 0.13 only).
    pub fn packet_errors(self) -> u8 {
        if self.is_empty() {
            0
        } else {
            (self.bits() & Self::PACKET_ERRORS_MASK.bits()) >> 4
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_output_enabled() {
        let flags = StatusFlags::OUTPUT_ENABLED;
        assert!(flags.output_enabled());

        let flags = StatusFlags::empty();
        assert!(!flags.output_enabled());
    }

    #[test]
    fn test_interlock_enabled() {
        // Test for newer firmware
        let flags = StatusFlags::INTERLOCK_ENABLED_V013;
        assert!(flags.interlock_enabled(0, 13));
        assert!(flags.interlock_enabled(1, 0));
        assert!(!flags.interlock_enabled(0, 12));

        // Test for older firmware
        let flags = StatusFlags::INTERLOCK_ENABLED_V012;
        assert!(!flags.interlock_enabled(0, 13));
        assert!(flags.interlock_enabled(0, 12));
    }

    #[test]
    fn test_temperature_warning() {
        // Test for newer firmware
        let flags = StatusFlags::TEMPERATURE_WARNING_V013;
        assert!(flags.temperature_warning(0, 13));
        assert!(flags.temperature_warning(1, 0));
        assert!(!flags.temperature_warning(0, 12));

        // Test for older firmware
        let flags = StatusFlags::TEMPERATURE_WARNING_V012;
        assert!(!flags.temperature_warning(0, 13));
        assert!(flags.temperature_warning(0, 12));
    }

    #[test]
    fn test_over_temperature() {
        // Test for newer firmware
        let flags = StatusFlags::OVER_TEMPERATURE_V013;
        assert!(flags.over_temperature(0, 13));
        assert!(flags.over_temperature(1, 0));
        assert!(!flags.over_temperature(0, 12));

        // Test for older firmware
        let flags = StatusFlags::OVER_TEMPERATURE_V012;
        assert!(!flags.over_temperature(0, 13));
        assert!(flags.over_temperature(0, 12));
    }

    #[test]
    fn test_packet_errors() {
        let flags = StatusFlags::from_bits_truncate(0x50); // 0101_0000
        assert_eq!(flags.packet_errors(), 5);

        let flags = StatusFlags::empty();
        assert_eq!(flags.packet_errors(), 0);
    }
}
