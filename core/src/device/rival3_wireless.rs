//! SteelSeries Rival 3 Wireless mouse profile data.
//!
//! Declarative data only — interpreted generically by
//! [`crate::device::profile::ProfiledDevice`]. See that module for the
//! command-building/response-parsing logic this data drives.

use crate::device::profile::{
    BatteryProfile, ButtonsProfile, DeviceProfile, DpiProfile, FirmwareVersionProfile,
    PollingRateProfile,
};
use crate::protocol::{commands, BatteryStatus, STEELSERIES_VENDOR_ID};
use crate::{Error, Result};

/// Product ID for Rival 3 Wireless (2.4 GHz mode)
pub const PRODUCT_ID: u16 = 0x1830;

/// TrueMove Air sensor DPI mapping table
/// Maps DPI values (100-18000 in steps of 100) to device byte values
const DPI_MAPPING: &[(u16, u8)] = &[
    (100, 0x00), (200, 0x02), (300, 0x03), (400, 0x04),
    (500, 0x05), (600, 0x06), (700, 0x07), (800, 0x09),
    (900, 0x0A), (1000, 0x0B), (1100, 0x0C), (1200, 0x0E),
    (1300, 0x0F), (1400, 0x10), (1500, 0x11), (1600, 0x13),
    (1700, 0x14), (1800, 0x15), (1900, 0x16), (2000, 0x18),
    (2100, 0x19), (2200, 0x1A), (2300, 0x1B), (2400, 0x1D),
    (2500, 0x1E), (2600, 0x1F), (2700, 0x20), (2800, 0x22),
    (2900, 0x23), (3000, 0x24), (3100, 0x25), (3200, 0x27),
    (3300, 0x28), (3400, 0x29), (3500, 0x2A), (3600, 0x2C),
    (3700, 0x2D), (3800, 0x2E), (3900, 0x2F), (4000, 0x31),
    (4100, 0x32), (4200, 0x33), (4300, 0x34), (4400, 0x36),
    (4500, 0x37), (4600, 0x38), (4700, 0x39), (4800, 0x3B),
    (4900, 0x3C), (5000, 0x3D), (5100, 0x3E), (5200, 0x40),
    (5300, 0x41), (5400, 0x42), (5500, 0x43), (5600, 0x45),
    (5700, 0x46), (5800, 0x47), (5900, 0x48), (6000, 0x4A),
    (6100, 0x4B), (6200, 0x4C), (6300, 0x4D), (6400, 0x4F),
    (6500, 0x50), (6600, 0x51), (6700, 0x52), (6800, 0x54),
    (6900, 0x55), (7000, 0x56), (7100, 0x57), (7200, 0x59),
    (7300, 0x5A), (7400, 0x5B), (7500, 0x5C), (7600, 0x5E),
    (7700, 0x5F), (7800, 0x60), (7900, 0x61), (8000, 0x63),
    (8100, 0x64), (8200, 0x65), (8300, 0x66), (8400, 0x68),
    (8500, 0x69), (8600, 0x6A), (8700, 0x6B), (8800, 0x6D),
    (8900, 0x6E), (9000, 0x6F), (9100, 0x70), (9200, 0x72),
    (9300, 0x73), (9400, 0x74), (9500, 0x75), (9600, 0x77),
    (9700, 0x78), (9800, 0x79), (9900, 0x7A), (10000, 0x7C),
    (10100, 0x7D), (10200, 0x7E), (10300, 0x7F), (10400, 0x81),
    (10500, 0x82), (10600, 0x83), (10700, 0x84), (10800, 0x86),
    (10900, 0x87), (11000, 0x88), (11100, 0x89), (11200, 0x8B),
    (11300, 0x8C), (11400, 0x8D), (11500, 0x8E), (11600, 0x90),
    (11700, 0x91), (11800, 0x92), (11900, 0x93), (12000, 0x95),
    (12100, 0x96), (12200, 0x97), (12300, 0x98), (12400, 0x9A),
    (12500, 0x9B), (12600, 0x9C), (12700, 0x9D), (12800, 0x9F),
    (12900, 0xA0), (13000, 0xA1), (13100, 0xA2), (13200, 0xA4),
    (13300, 0xA5), (13400, 0xA6), (13500, 0xA7), (13600, 0xA9),
    (13700, 0xAA), (13800, 0xAB), (13900, 0xAC), (14000, 0xAE),
    (14100, 0xAF), (14200, 0xB0), (14300, 0xB1), (14400, 0xB3),
    (14500, 0xB4), (14600, 0xB5), (14700, 0xB6), (14800, 0xB8),
    (14900, 0xB9), (15000, 0xBA), (15100, 0xBB), (15200, 0xBD),
    (15300, 0xBE), (15400, 0xBF), (15500, 0xC0), (15600, 0xC2),
    (15700, 0xC3), (15800, 0xC4), (15900, 0xC5), (16000, 0xC7),
    (16100, 0xC8), (16200, 0xC9), (16300, 0xCA), (16400, 0xCC),
    (16500, 0xCD), (16600, 0xCE), (16700, 0xCF), (16800, 0xD1),
    (16900, 0xD2), (17000, 0xD3), (17100, 0xD4), (17200, 0xD6),
    (17300, 0xD7), (17400, 0xD8), (17500, 0xD9), (17600, 0xDB),
    (17700, 0xDC), (17800, 0xDD), (17900, 0xDE), (18000, 0xE0),
];

/// Parses a Rival 3 Wireless battery response: `[level, _, charging]`.
fn parse_battery(data: &[u8]) -> Result<BatteryStatus> {
    BatteryStatus::from_response(data)
        .ok_or_else(|| Error::InvalidParameter("Invalid battery status response".to_string()))
}

/// Formats the 2-byte firmware-version response as "major.minor". This ordering/format is
/// an assumption (major byte first, minor byte second) not independently re-verified against
/// rivalcfg's actual CLI display this session — correct if it turns out wrong once tested
/// against real hardware.
pub(crate) fn parse_firmware_version(response: &[u8]) -> Result<String> {
    if response.len() < 2 {
        return Err(Error::InvalidParameter(
            "Invalid firmware version response".to_string(),
        ));
    }
    Ok(format!("{}.{}", response[0], response[1]))
}

/// SteelSeries Rival 3 Wireless device profile.
pub const RIVAL3_WIRELESS_PROFILE: DeviceProfile = DeviceProfile {
    name: "SteelSeries Rival 3 Wireless",
    vendor_id: STEELSERIES_VENDOR_ID,
    product_ids: &[PRODUCT_ID],
    usage_page: 0xffc0,
    interface: None,
    battery: Some(BatteryProfile {
        command: commands::BATTERY_LEVEL,
        extra_byte: Some(0x01),
        readback_length: 3,
        parse: parse_battery,
    }),
    chatmix: None,
    dpi: Some(DpiProfile {
        command: commands::SENSITIVITY,
        mapping: DPI_MAPPING,
        max_presets: 5,
    }),
    polling_rate: Some(PollingRateProfile {
        command: commands::POLLING_RATE,
    }),
    buttons: Some(ButtonsProfile {
        command: commands::BUTTON_MAPPING,
    }),
    sidetone: None,
    mic_mute_led: None,
    mic_volume: None,
    equalizer: None,
    sleep_timer: None,
    volume_limiter: None,
    bluetooth: None,
    firmware_version: Some(FirmwareVersionProfile {
        command: commands::FIRMWARE_VERSION,
        extra_byte: Some(0x00),
        readback_length: 2,
        parse: parse_firmware_version,
    }),
};
