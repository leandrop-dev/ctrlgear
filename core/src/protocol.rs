//! USB HID protocol constants and types for SteelSeries devices

use crate::Result;

/// HID report types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReportType {
    /// Standard HID output report
    Output = 0x02,
    /// HID feature report
    Feature = 0x03,
}

/// Command delay in milliseconds to prevent device overload
pub const DEFAULT_COMMAND_DELAY_MS: u64 = 50;

/// SteelSeries vendor ID
pub const STEELSERIES_VENDOR_ID: u16 = 0x1038;

/// Command codes for Rival 3 Wireless
pub mod commands {
    /// Save settings to device memory
    pub const SAVE: u8 = 0x09;

    /// Get firmware version
    pub const FIRMWARE_VERSION: u8 = 0x90;

    /// Set polling rate
    pub const POLLING_RATE: u8 = 0x17;

    /// Set button mapping
    pub const BUTTON_MAPPING: u8 = 0x19;

    /// Set sidetone (mic monitoring) level
    pub const SIDETONE: u8 = 0x39;

    /// Set microphone-mute LED brightness
    pub const MIC_MUTE_LED_BRIGHTNESS: u8 = 0xae;

    /// Set microphone input volume
    pub const MIC_VOLUME: u8 = 0x37;

    /// Set DPI/sensitivity
    pub const SENSITIVITY: u8 = 0x20;

    /// Get battery level (wireless devices)
    pub const BATTERY_LEVEL: u8 = 0xAA;

    /// Set 10-band equalizer gains
    pub const EQUALIZER: u8 = 0x33;

    /// Set auto-sleep (inactive time) timer, in minutes
    pub const SLEEP_TIMER: u8 = 0xa3;

    /// Toggle the output volume limiter
    pub const VOLUME_LIMITER: u8 = 0x3a;

    /// Toggle Bluetooth-when-powered-on
    pub const BLUETOOTH_POWER_ON: u8 = 0xb2;

    /// Set Bluetooth call-volume mode
    pub const BLUETOOTH_CALL_VOLUME: u8 = 0xb3;

    /// Save Bluetooth settings to device memory; distinct from the general single-byte [`SAVE`].
    pub const BLUETOOTH_SAVE: [u8; 2] = [0x06, 0x09];

    /// Set DPI/sensitivity presets (Aerox/Rival 3 Gen 2/Prime Mini family)
    pub const SENSITIVITY_AEROX_GEN2: u8 = 0x2D;

    /// Set polling rate (Aerox/Rival 3 Gen 2/Prime Mini family)
    pub const POLLING_RATE_AEROX_GEN2: u8 = 0x2B;

    /// Set button mapping (Aerox/Rival 3 Gen 2/Prime Mini family)
    pub const BUTTON_MAPPING_AEROX_GEN2: u8 = 0x2A;

    /// Set per-zone RGB color (Aerox/Rival 3 Gen 2/Prime Mini family)
    pub const RGB_ZONE_COLOR: u8 = 0x21;

    /// Set rainbow effect (Aerox 3/Aerox 5/Rival 3 Gen 2; absent on Prime Mini)
    pub const RGB_RAINBOW_EFFECT: u8 = 0x22;

    /// Set RGB LED brightness (Aerox 3/Aerox 5/Rival 3 Gen 2; absent on Prime Mini)
    pub const RGB_BRIGHTNESS: u8 = 0x23;

    /// Set reactive (on-click) RGB color (Aerox 3/Aerox 5/Rival 3 Gen 2; absent on Prime Mini)
    pub const RGB_REACTIVE_COLOR: u8 = 0x26;

    /// Set default (startup) lighting (Aerox/Rival 3 Gen 2/Prime Mini family)
    pub const RGB_DEFAULT_LIGHTING: u8 = 0x27;
}

/// Polling rate options
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PollingRate {
    /// 125 Hz
    Hz125 = 125,
    /// 250 Hz
    Hz250 = 250,
    /// 500 Hz
    Hz500 = 500,
    /// 1000 Hz
    Hz1000 = 1000,
}

impl PollingRate {
    /// Convert polling rate to device command byte
    pub fn to_byte(self) -> u8 {
        match self {
            PollingRate::Hz125 => 0x03,
            PollingRate::Hz250 => 0x02,
            PollingRate::Hz500 => 0x01,
            PollingRate::Hz1000 => 0x00,
        }
    }

    /// Parse polling rate from Hz value
    pub fn from_hz(hz: u16) -> Result<Self> {
        match hz {
            125 => Ok(PollingRate::Hz125),
            250 => Ok(PollingRate::Hz250),
            500 => Ok(PollingRate::Hz500),
            1000 => Ok(PollingRate::Hz1000),
            _ => Err(crate::Error::InvalidParameter(format!(
                "Invalid polling rate: {}. Valid options: 125, 250, 500, 1000",
                hz
            ))),
        }
    }
}

/// Battery status for wireless devices
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BatteryStatus {
    /// Battery level (0-100)
    pub level: u8,
    /// Whether the device is currently charging
    pub is_charging: bool,
}

impl BatteryStatus {
    /// Parse battery status from device response
    pub fn from_response(data: &[u8]) -> Option<Self> {
        if data.len() >= 3 {
            Some(BatteryStatus {
                level: data[0],
                is_charging: data[2] != 0,
            })
        } else {
            None
        }
    }
}

/// ChatMix (game/chat volume balance) dial position for wireless headsets.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChatMixStatus {
    /// Game volume (0-100)
    pub game_percent: u8,
    /// Chat volume (0-100)
    pub chat_percent: u8,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_polling_rate_conversion() {
        assert_eq!(PollingRate::Hz125.to_byte(), 0x03);
        assert_eq!(PollingRate::Hz250.to_byte(), 0x02);
        assert_eq!(PollingRate::Hz500.to_byte(), 0x01);
        assert_eq!(PollingRate::Hz1000.to_byte(), 0x00);
    }

    #[test]
    fn test_polling_rate_from_hz() {
        assert!(matches!(PollingRate::from_hz(125), Ok(PollingRate::Hz125)));
        assert!(matches!(PollingRate::from_hz(1000), Ok(PollingRate::Hz1000)));
        assert!(PollingRate::from_hz(999).is_err());
    }

    #[test]
    fn test_battery_status_parsing() {
        let data = [75, 0, 1];
        let status = BatteryStatus::from_response(&data).unwrap();
        assert_eq!(status.level, 75);
        assert!(status.is_charging);

        let data = [50, 0, 0];
        let status = BatteryStatus::from_response(&data).unwrap();
        assert_eq!(status.level, 50);
        assert!(!status.is_charging);
    }
}
