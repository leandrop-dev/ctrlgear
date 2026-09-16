//! SteelSeries Aerox 3, Aerox 5, Rival 3 Gen 2, and Prime Mini mouse profile
//! data — the shared "Gen 2 RGB" wired mouse family (Prime Mini included:
//! it shares this family's overall command style, not the unrelated
//! Prime/Prime+ scheme).
//!
//! **Hardware-verification status: spec-only.** None of these 4 devices are
//! physically available on this development machine; only the Rival 3
//! Wireless and Arctis Nova 7 are hardware-verified (see their own modules).
//! Every command byte, zone table, and choice table below is transcribed
//! directly from rivalcfg (WTFPL) via `gh api` —
//! `rivalcfg/devices/aerox3.py`, `aerox5.py`, `rival3_gen2.py`,
//! `prime_mini.py`, plus `rivalcfg/handlers/reactive_rgbcolor.py` and
//! `rivalcfg/handlers/multidpi_range.py`/`multidpi_range_choice.py` for the
//! exact wire shapes — not guessed, but never round-tripped against real
//! hardware.
//!
//! Re-verifying the task plan's claims against the actual rivalcfg source
//! (rather than trusting the plan's own summary) surfaced 3 corrections,
//! documented in full at each relevant `const`/comment below:
//!
//! 1. **Aerox 5 uses the `truemove_air` sensor (100-18000 DPI), not
//!    `truemove_core`** (200-8500 DPI) as the task plan stated — confirmed
//!    by `aerox5.py`'s `from .dpi import truemove_air` and its
//!    `input_range: [100, 18000, 100]`. See [`AEROX5_PROFILE`].
//! 2. **Rival 3 Gen 2's DPI command differs from Aerox 3/5's and uses a
//!    dual-axis (X/Y) value shape** the current [`crate::device::profile::DpiProfile`]
//!    doesn't support, so [`RIVAL3_GEN2_PROFILE`] leaves `dpi: None`. See
//!    its doc comment.
//! 3. **Every profile's `polling_rate` choices are shifted by one byte from
//!    Rival 3 Wireless's**, not "the same 4-value scheme" the task plan
//!    claimed — confirmed against `aerox3.py` vs. `rival3_wireless.py`
//!    (`{125:0x04,...}` vs. `{125:0x03,...}`). This required widening
//!    [`crate::device::profile::PollingRateProfile`] with its own `choices`
//!    table instead of the crate-wide [`crate::protocol::PollingRate`]
//!    mapping it used to rely on exclusively.
//!
//! A 4th finding, **buttons_mapping's real wire shape doesn't fit this
//! crate's existing 6-slot [`crate::buttons::ButtonMapping`]**, is
//! documented on [`BUTTONS`] below rather than fixed — it would require
//! redesigning shared infrastructure also used by the hardware-verified
//! Rival 3 Wireless, out of scope for this task.

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

/// `buttons_mapping` command byte, shared by all 4 devices (verified
/// identical across `aerox3.py`, `aerox5.py`, `rival3_gen2.py`,
/// `prime_mini.py`).
///
/// **Known limitation**: every one of these devices' real button tables has
/// 8 (Aerox 3/Rival 3 Gen 2/Prime Mini) or 11 (Aerox 5, which has 9 physical
/// buttons) fixed-offset slots — `ScrollUp`/`ScrollDown` are separate
/// always-present slots at their own offsets, not alternate actions
/// assignable to one of 6 button slots like on the already-shipped Rival 3
/// Wireless. This crate's [`crate::buttons::ButtonMapping`] only models 6
/// slots (30 bytes), shared by every device including Rival 3 Wireless, so
/// attaching it here unmodified (per the approved task plan: "Reuse
/// existing ButtonsProfile/ButtonMapping::encode") produces a
/// too-short/wrong-shape `buttons_mapping` command for all 4 of these
/// devices. Redesigning the shared button-mapping shape to support
/// per-device slot tables is out of this task's scope (it touches
/// hardware-verified Rival 3 Wireless too) — flagged here for a follow-up
/// task rather than silently shipped as correct.
const BUTTONS: ButtonsProfile = ButtonsProfile {
    command: commands::BUTTON_MAPPING_AEROX_GEN2,
};

/// `polling_rate` choices shared by all 4 devices — verified identical
/// across `aerox3.py`, `aerox5.py`, `rival3_gen2.py`, `prime_mini.py`, and
/// confirmed *different* from Rival 3 Wireless's `{125:0x03, 250:0x02,
/// 500:0x01, 1000:0x00}` (see [`crate::device::profile::PollingRateProfile`]'s
/// doc comment).
const POLLING_RATE_CHOICES: &[(u16, u8)] = &[(125, 0x04), (250, 0x03), (500, 0x02), (1000, 0x01)];

/// Zone byte-prefix table shared by the Aerox 3, Aerox 5 and Rival 3 Gen 2
/// (their z1/z2/z3 commands are byte-identical): zone 1 (top) ->
/// `[0x01]`, zone 2 (middle) -> `[0x02, 0x00, 0x00, 0x00]`, zone 3 (bottom)
/// -> `[0x04, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]`. The padding bytes are
/// positional (verified against `aerox3.py`/`rival3_gen2.py`), not
/// derivable from a formula.
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

/// RGB profile shared by the Aerox 3, Aerox 5 and Rival 3 Gen 2 (their RGB
/// commands are byte-identical, per `aerox3.py`/`aerox5.py`/`rival3_gen2.py`).
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

/// `[0x11, 0x00]` save command shared by all 4 devices (verified identical
/// across `aerox3.py`, `aerox5.py`, `rival3_gen2.py`, `prime_mini.py`),
/// distinct from Rival 3 Wireless/Arctis Nova 7's single-byte `[0x09]`.
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
/// Uses the `truemove_air` sensor (100-18000 DPI, same range/table as the
/// already-shipped Rival 3 Wireless's sensor — see
/// [`crate::device::dpi_tables::TRUEMOVE_AIR_MAPPING`]'s doc comment for why
/// this is a fresh, independently-verified table rather than a reuse of
/// Rival 3 Wireless's own `DPI_MAPPING`), *not* `truemove_core` as the
/// approved task plan's summary stated — confirmed directly against
/// `aerox5.py`'s `from .dpi import truemove_air` and `input_range: [100,
/// 18000, 100]`. `first_preset: 0`, also per `aerox5.py` (unlike Aerox 3's
/// `first_preset: 1`).
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
/// **`dpi: None`** — despite RGB/polling_rate/buttons/save being
/// byte-identical to Aerox 3/5 (per the approved task plan), `rival3_gen2.py`'s
/// actual `sensitivity` setting differs in 2 ways the plan's summary missed:
/// its command is `[0x34]` (not `[0x2D]`), and its `value_type` is
/// `multidpi_range_choice_xy` — an independent X/Y DPI pair per preset
/// (`xy_mapping: "xyxy"`, doubling the values written and interleaving
/// them), not the single-value-per-preset shape
/// [`crate::device::profile::DpiProfile`]/`set_dpi` implements. Shipping
/// either the wrong command byte or a wrong-shaped command for this setting
/// would be worse than leaving it unsupported, so DPI is left `None` here
/// pending a follow-up task that extends `DpiProfile` (or adds a sibling
/// capability) for dual-axis presets.
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
/// (single bytes, `off: 0x00`, `rainbow: 0x01`) — confirmed against
/// `prime_mini.py` directly, simpler than the Aerox family's 4-choice
/// combinatorial scheme (matches the task plan's expectation that this
/// mirrors the Rival 5's simpler scheme).
const PRIME_MINI_DEFAULT_LIGHTING: &[(&str, &[u8])] = &[("off", &[0x00]), ("rainbow", &[0x01])];

/// Prime Mini RGB profile: a single zone (`color` command `[0x21, 0x00]`),
/// no reactive color/brightness/rainbow effect at all — `prime_mini.py`'s
/// settings dict has no `reactive_color`, `led_brightness` or
/// `rainbow_effect` keys, confirmed directly against the source rather than
/// assumed from the rest of the family.
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
/// DPI uses a formula-based sensor (`value_type: "multidpi_range"`,
/// `byte_value = dpi / 50`, confirmed against `prime_mini.py`'s
/// `input_range: [50, 18000, 50]` / `output_range: [0x01, 0x0168, 1]`)
/// rather than the Aerox family's lookup-table sensor. Rather than adding a
/// separate formula-evaluation code path to
/// [`crate::device::profile::ProfiledDevice::set_dpi`], the formula is
/// expanded to a full lookup table at
/// [`crate::device::dpi_tables::PRIME_MINI_DPI_MAPPING`] (computed directly
/// from the verified formula, not guessed), so this profile uses the exact
/// same lookup-table code path as every other `DpiProfile` — only
/// `value_width: 2` differs, since the Prime Mini's raw value can exceed a
/// single byte (up to `0x0168`), unlike Aerox 3/5/Rival 3 Gen 2's
/// `value_width: 1`. `first_preset: 0`, per `prime_mini.py`.
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
        // See RIVAL3_GEN2_PROFILE's doc comment: its real DPI shape
        // (dual-axis, command 0x34) doesn't fit DpiProfile.
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
