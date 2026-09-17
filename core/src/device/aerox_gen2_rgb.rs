//! SteelSeries Aerox 3, Aerox 5, Rival 3 Gen 2, and Prime Mini mouse profile
//! data — the shared "Gen 2 RGB" wired mouse family (Prime Mini included:
//! it shares this family's overall command style, not the unrelated
//! Prime/Prime+ scheme).
//!
//! Spec-only: none of these 4 devices are hardware-verified on this
//! development machine; every command byte, zone table, and choice table
//! below is transcribed from rivalcfg, not guessed. Re-verifying the task
//! plan's claims against the actual source surfaced several corrections —
//! see [`AEROX5_PROFILE`], [`RIVAL3_GEN2_PROFILE`], and [`BUTTONS`]'s doc
//! comments, and the polling-rate divergence noted on
//! [`crate::device::profile::PollingRateProfile`].

use crate::device::profile::{
    ButtonsProfile, DeviceProfile, DpiProfile, PollingRateProfile, RgbProfile,
};
use crate::device::dpi_tables::{PRIME_MINI_DPI_MAPPING, TRUEMOVE_AIR_MAPPING, TRUEMOVE_CORE_MAPPING};
use crate::protocol::{commands, STEELSERIES_VENDOR_ID};

/// Product ID for the SteelSeries Aerox 3.
pub const AEROX3_PRODUCT_ID: u16 = 0x1836;
/// Product ID for the SteelSeries Aerox 5.
pub const AEROX5_PRODUCT_ID: u16 = 0x1850;
/// Product ID for the SteelSeries Rival 3 Gen 2.
pub const RIVAL3_GEN2_PRODUCT_ID: u16 = 0x1870;
/// Product ID for the SteelSeries Prime Mini.
pub const PRIME_MINI_PRODUCT_ID: u16 = 0x184D;

/// `buttons_mapping` command byte, shared by all 4 devices.
///
/// **Known limitation**: doesn't fit these devices' real per-device
/// button-slot shape — see [`crate::buttons::ButtonMapping`].
const BUTTONS: ButtonsProfile = ButtonsProfile {
    command: commands::BUTTON_MAPPING_AEROX_GEN2,
};

/// `polling_rate` choices shared by all 4 devices; differs from Rival 3
/// Wireless's mapping — see
/// [`crate::device::profile::PollingRateProfile`]'s doc comment.
const POLLING_RATE_CHOICES: &[(u16, u8)] = &[(125, 0x04), (250, 0x03), (500, 0x02), (1000, 0x01)];

/// Zone byte-prefix table shared by the Aerox 3, Aerox 5 and Rival 3 Gen 2
/// (byte-identical z1/z2/z3 commands): zone 1 (top), zone 2 (middle), zone
/// 3 (bottom).
const AEROX_ZONES: &[(u8, &[u8])] = &[
    (1, &[0x01]),
    (2, &[0x02, 0x00, 0x00, 0x00]),
    (3, &[0x04, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]),
];

/// `rainbow_effect` bitmask choices shared by the Aerox 3/Aerox 5/Rival 3
/// Gen 2 (identical across all 3 sources); absent entirely on the Prime
/// Mini.
const RAINBOW_CHOICES: &[(&str, u8)] = &[
    ("all", 0b111),
    ("bottom", 0b100),
    ("middle", 0b010),
    ("top", 0b001),
    ("bottom-middle", 0b110),
    ("middle-top", 0b011),
    ("bottom-top", 0b101),
];

/// `default_lighting` 4-choice combinatorial scheme shared by the Aerox
/// 3/Aerox 5/Rival 3 Gen 2 (identical across all 3 sources); the Prime Mini
/// uses a simpler 2-choice scheme instead (see [`PRIME_MINI_DEFAULT_LIGHTING`]).
const AEROX_DEFAULT_LIGHTING: &[(&str, &[u8])] = &[
    ("off", &[0x00, 0x00]),
    ("reactive", &[0x00, 0x01]),
    ("rainbow", &[0x01, 0x00]),
    ("reactive-rainbow", &[0x01, 0x01]),
];

/// RGB profile shared by the Aerox 3, Aerox 5 and Rival 3 Gen 2
/// (byte-identical RGB commands).
const AEROX_RGB: RgbProfile = RgbProfile {
    zone_command: commands::RGB_ZONE_COLOR,
    zones: AEROX_ZONES,
    reactive_command: Some(commands::RGB_REACTIVE_COLOR),
    brightness_command: Some(commands::RGB_BRIGHTNESS),
    brightness_range: (0x00, 0x64),
    rainbow_command: Some(&[commands::RGB_RAINBOW_EFFECT]),
    rainbow_choices: RAINBOW_CHOICES,
    default_lighting_command: commands::RGB_DEFAULT_LIGHTING,
    default_lighting_choices: AEROX_DEFAULT_LIGHTING,
};

/// `[0x11, 0x00]` save command shared by all 4 devices; distinct from
/// Rival 3 Wireless/Arctis Nova 7's single-byte `[0x09]`.
const SAVE_COMMAND: &[u8] = &[0x11, 0x00];

/// SteelSeries Aerox 3 device profile.
pub const AEROX3_PROFILE: DeviceProfile = DeviceProfile {
    name: "SteelSeries Aerox 3",
    vendor_id: STEELSERIES_VENDOR_ID,
    product_ids: &[AEROX3_PRODUCT_ID],
    usage_page: 0xffc0,
    dpi: Some(DpiProfile {
        command: commands::SENSITIVITY_AEROX_GEN2,
        mapping: TRUEMOVE_CORE_MAPPING,
        max_presets: 5,
        first_preset: 1,
        value_width: 1,
    }),
    polling_rate: Some(PollingRateProfile {
        command: commands::POLLING_RATE_AEROX_GEN2,
        choices: POLLING_RATE_CHOICES,
    }),
    buttons: Some(BUTTONS),
    rgb: Some(AEROX_RGB),
    save_command: SAVE_COMMAND,
    ..DeviceProfile::NONE
};

/// SteelSeries Aerox 5 device profile.
///
/// Uses the `truemove_air` sensor (100-18000 DPI), not `truemove_core` as
/// the task plan stated — see
/// [`crate::device::dpi_tables::TRUEMOVE_AIR_MAPPING`].
pub const AEROX5_PROFILE: DeviceProfile = DeviceProfile {
    name: "SteelSeries Aerox 5",
    vendor_id: STEELSERIES_VENDOR_ID,
    product_ids: &[AEROX5_PRODUCT_ID],
    usage_page: 0xffc0,
    dpi: Some(DpiProfile {
        command: commands::SENSITIVITY_AEROX_GEN2,
        mapping: TRUEMOVE_AIR_MAPPING,
        max_presets: 5,
        first_preset: 0,
        value_width: 1,
    }),
    polling_rate: Some(PollingRateProfile {
        command: commands::POLLING_RATE_AEROX_GEN2,
        choices: POLLING_RATE_CHOICES,
    }),
    buttons: Some(BUTTONS),
    rgb: Some(AEROX_RGB),
    save_command: SAVE_COMMAND,
    ..DeviceProfile::NONE
};

/// SteelSeries Rival 3 Gen 2 device profile.
///
/// **`dpi: None`** — its DPI command uses a dual-axis (X/Y) value shape
/// that [`crate::device::profile::DpiProfile`] doesn't support.
pub const RIVAL3_GEN2_PROFILE: DeviceProfile = DeviceProfile {
    name: "SteelSeries Rival 3 Gen 2",
    vendor_id: STEELSERIES_VENDOR_ID,
    product_ids: &[RIVAL3_GEN2_PRODUCT_ID],
    usage_page: 0xffc0,
    polling_rate: Some(PollingRateProfile {
        command: commands::POLLING_RATE_AEROX_GEN2,
        choices: POLLING_RATE_CHOICES,
    }),
    buttons: Some(BUTTONS),
    rgb: Some(AEROX_RGB),
    save_command: SAVE_COMMAND,
    ..DeviceProfile::NONE
};

/// Prime Mini `default_lighting`: a simple off/rainbow 2-choice scheme
/// (single bytes, `off: 0x00`, `rainbow: 0x01`).
const PRIME_MINI_DEFAULT_LIGHTING: &[(&str, &[u8])] = &[("off", &[0x00]), ("rainbow", &[0x01])];

/// Prime Mini RGB profile: a single zone (`color` command `[0x21, 0x00]`),
/// no reactive color/brightness/rainbow effect at all.
const PRIME_MINI_RGB: RgbProfile = RgbProfile {
    zone_command: commands::RGB_ZONE_COLOR,
    zones: &[(1, &[0x00])],
    reactive_command: None,
    brightness_command: None,
    brightness_range: (0x00, 0x00),
    rainbow_command: None,
    rainbow_choices: &[],
    default_lighting_command: commands::RGB_DEFAULT_LIGHTING,
    default_lighting_choices: PRIME_MINI_DEFAULT_LIGHTING,
};

/// SteelSeries Prime Mini device profile.
///
/// DPI uses a formula-based sensor rather than the Aerox family's
/// lookup-table sensor; the formula is expanded to a full lookup table at
/// [`crate::device::dpi_tables::PRIME_MINI_DPI_MAPPING`] so this profile can
/// reuse the same lookup-table code path as every other `DpiProfile` — only
/// `value_width: 2` differs.
pub const PRIME_MINI_PROFILE: DeviceProfile = DeviceProfile {
    name: "SteelSeries Prime Mini",
    vendor_id: STEELSERIES_VENDOR_ID,
    product_ids: &[PRIME_MINI_PRODUCT_ID],
    usage_page: 0xffc0,
    dpi: Some(DpiProfile {
        command: commands::SENSITIVITY_AEROX_GEN2,
        mapping: PRIME_MINI_DPI_MAPPING,
        max_presets: 5,
        first_preset: 0,
        value_width: 2,
    }),
    polling_rate: Some(PollingRateProfile {
        command: commands::POLLING_RATE_AEROX_GEN2,
        choices: POLLING_RATE_CHOICES,
    }),
    buttons: Some(BUTTONS),
    rgb: Some(PRIME_MINI_RGB),
    save_command: SAVE_COMMAND,
    ..DeviceProfile::NONE
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aerox3_and_rival3_gen2_share_rgb_polling_and_save_bytes() {
        assert_eq!(AEROX3_PROFILE.rgb.unwrap().zone_command, RIVAL3_GEN2_PROFILE.rgb.unwrap().zone_command);
        assert_eq!(AEROX3_PROFILE.polling_rate.unwrap().choices, RIVAL3_GEN2_PROFILE.polling_rate.unwrap().choices);
        assert_eq!(AEROX3_PROFILE.save_command, RIVAL3_GEN2_PROFILE.save_command);
    }

    #[test]
    fn rival3_gen2_has_no_dpi_capability() {
        assert!(RIVAL3_GEN2_PROFILE.dpi.is_none());
    }

    #[test]
    fn aerox5_uses_truemove_air_mapping_not_truemove_core() {
        let aerox5_mapping = AEROX5_PROFILE.dpi.unwrap().mapping;
        let aerox3_mapping = AEROX3_PROFILE.dpi.unwrap().mapping;
        assert_ne!(aerox5_mapping.len(), aerox3_mapping.len());
        assert_eq!(aerox5_mapping.first(), Some(&(100, 0x00)));
        assert_eq!(aerox5_mapping.last(), Some(&(18000, 0xD6)));
    }

    #[test]
    fn prime_mini_dpi_uses_2_byte_value_width_others_use_1() {
        assert_eq!(PRIME_MINI_PROFILE.dpi.unwrap().value_width, 2);
        assert_eq!(AEROX3_PROFILE.dpi.unwrap().value_width, 1);
        assert_eq!(AEROX5_PROFILE.dpi.unwrap().value_width, 1);
    }

    #[test]
    fn every_product_id_is_distinct_from_the_others_in_this_family() {
        let ids = [
            AEROX3_PRODUCT_ID,
            AEROX5_PRODUCT_ID,
            RIVAL3_GEN2_PRODUCT_ID,
            PRIME_MINI_PRODUCT_ID,
        ];
        for (i, a) in ids.iter().enumerate() {
            for b in &ids[i + 1..] {
                assert_ne!(a, b);
            }
        }
    }
}
