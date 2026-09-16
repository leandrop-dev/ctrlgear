//! Declarative device profiles.
//!
//! Instead of one hand-written struct plus trait impls per device (the old
//! `rival3_wireless.rs` / `arctis_nova_7.rs` pattern), a [`DeviceProfile`] is
//! a `const` data description of a device's HID identity and the commands
//! its capabilities need. [`ProfiledDevice`] wraps a [`SteelSeriesHid`] and
//! a `&'static DeviceProfile` and implements every capability trait from
//! [`crate::device`] generically, by interpreting that data. Adding a new
//! device means adding a new `const DeviceProfile` (see `rival3_wireless.rs`
//! / `arctis_nova_7.rs` for examples), not a new module full of trait impls.

use crate::buttons::ButtonMapping;
use crate::device::{
    ChatMixReadable, ConfigurableButtons, ConfigurableDpi, ConfigurablePollingRate,
    ConfigurableSidetone, SteelSeriesDevice, WirelessDevice,
};
use crate::hid::SteelSeriesHid;
use crate::protocol::{self, BatteryStatus, ChatMixStatus, PollingRate, ReportType};
use crate::{Error, Result};

/// Data needed to query battery/status: which command to send and how to
/// parse the reply. The two known reply shapes so far (Rival 3 Wireless's
/// plain 3-byte report vs. Arctis Nova 7's offset-2/3 status report with an
/// offline sentinel) differ enough in both layout and error handling that
/// the parsing itself is a function pointer rather than a further-decomposed
/// data shape — see each profile's `parse_*` function for the concrete rule.
#[derive(Clone, Copy)]
pub struct BatteryProfile {
    /// Command byte that requests a battery/status report.
    pub command: u8,
    /// Extra byte sent immediately after `command`, if the device expects
    /// one (e.g. Rival 3 Wireless's `0x01` sub-command byte). `None` when
    /// the command is sent standalone.
    pub extra_byte: Option<u8>,
    /// Number of bytes to read back.
    pub readback_length: usize,
    /// Parses the raw readback into a [`BatteryStatus`].
    pub parse: fn(&[u8]) -> Result<BatteryStatus>,
}

/// Data needed to read chatmix: kept as its own struct rather than folded
/// into `BatteryProfile`, even though on the Arctis Nova 7 both share the
/// identical 0xb0 command/response — their offline-handling semantics
/// differ (chatmix must stay readable offline, battery must not), and a
/// future device could have one capability without the other.
#[derive(Clone, Copy)]
pub struct ChatMixProfile {
    pub command: u8,
    pub extra_byte: Option<u8>,
    pub readback_length: usize,
    pub parse: fn(&[u8]) -> Result<ChatMixStatus>,
}

/// Data needed to set DPI: the command byte, the DPI-value -> device-byte
/// mapping table, and the maximum number of presets the device accepts.
#[derive(Clone, Copy)]
pub struct DpiProfile {
    pub command: u8,
    pub mapping: &'static [(u16, u8)],
    pub max_presets: u8,
}

/// Data needed to set polling rate: just the command byte, since the
/// Hz <-> device-byte mapping ([`PollingRate`]) is shared across devices.
#[derive(Clone, Copy)]
pub struct PollingRateProfile {
    pub command: u8,
}

/// Data needed to set button mapping: just the command byte, since
/// [`ButtonMapping::encode`] is shared across devices.
#[derive(Clone, Copy)]
pub struct ButtonsProfile {
    pub command: u8,
}

/// Data needed to set sidetone level: just the command byte, since the
/// 0-3 (Off/Low/Medium/High) level range is shared across devices.
#[derive(Clone, Copy)]
pub struct SidetoneProfile {
    pub command: u8,
}

/// Declarative description of one SteelSeries device: its HID identity plus
/// one `Option<XyzProfile>` per capability it supports. `None` means the
/// device doesn't have that capability — the corresponding [`ProfiledDevice`]
/// trait method returns [`Error::Unsupported`].
pub struct DeviceProfile {
    pub name: &'static str,
    pub vendor_id: u16,
    pub product_ids: &'static [u16],
    /// HID usage page of the device's configuration interface.
    pub usage_page: u16,
    /// HID interface number of the configuration interface, when
    /// `usage_page` alone doesn't disambiguate it.
    pub interface: Option<u8>,
    pub battery: Option<BatteryProfile>,
    pub chatmix: Option<ChatMixProfile>,
    pub dpi: Option<DpiProfile>,
    pub polling_rate: Option<PollingRateProfile>,
    pub buttons: Option<ButtonsProfile>,
    pub sidetone: Option<SidetoneProfile>,
}

impl DeviceProfile {
    /// A base profile with every identity field zeroed and every capability
    /// `None`. Declare a device's profile with `..DeviceProfile::NONE` to
    /// pick up `None` for every capability field you don't explicitly set,
    /// instead of writing `dpi: None, polling_rate: None, buttons: None, ...`
    /// by hand for devices that only support one or two capabilities — the
    /// common case across the catalog (see `plan.md`'s audit tables).
    pub const NONE: DeviceProfile = DeviceProfile {
        name: "",
        vendor_id: 0,
        product_ids: &[],
        usage_page: 0,
        interface: None,
        battery: None,
        chatmix: None,
        dpi: None,
        polling_rate: None,
        buttons: None,
        sidetone: None,
    };
}

/// Derives (min, max, step) from a DPI mapping table itself, so the reported
/// valid range always reflects the actual supported values rather than a
/// separately hand-maintained duplicate that could silently drift if the
/// mapping table changes.
fn dpi_mapping_range(mapping: &[(u16, u8)]) -> (u16, u16, u16) {
    let min = mapping.first().map(|&(dpi, _)| dpi).unwrap_or(0);
    let max = mapping.last().map(|&(dpi, _)| dpi).unwrap_or(0);
    let step = mapping
        .get(1)
        .map(|&(dpi, _)| dpi.saturating_sub(min))
        .unwrap_or(0);
    (min, max, step)
}

/// Computes the range [`ConfigurableDpi::dpi_range`] reports for a profile,
/// factored out as a plain function of `&DeviceProfile` (rather than a
/// `ProfiledDevice` method) so it's testable without a live HID handle.
fn dpi_range_for(profile: &DeviceProfile) -> (u16, u16, u16) {
    match &profile.dpi {
        Some(dpi) => dpi_mapping_range(dpi.mapping),
        None => (0, 0, 0),
    }
}

/// Converts a DPI value to its device byte using `mapping`.
fn dpi_to_byte(mapping: &[(u16, u8)], dpi: u16) -> Result<u8> {
    mapping
        .iter()
        .find(|(d, _)| *d == dpi)
        .map(|(_, byte)| *byte)
        .ok_or_else(|| {
            let (min, max, step) = dpi_mapping_range(mapping);
            Error::InvalidParameter(format!(
                "Invalid DPI value: {}. Must be {}-{} in steps of {}",
                dpi, min, max, step
            ))
        })
}

/// Fetches `$self.profile.$field`, or short-circuits the enclosing method
/// with `Err(Error::Unsupported(..))` naming both the device and the
/// missing capability. Every `ProfiledDevice` capability-trait method below
/// follows this same "check the profile field, build+send the command, or
/// refuse" shape, so the check itself lives here once instead of being
/// hand-copied into each `impl`.
macro_rules! require_capability {
    ($self:expr, $field:ident, $label:expr) => {
        match &$self.profile.$field {
            Some(p) => p,
            None => {
                return Err(Error::Unsupported(format!(
                    "{} has no {}",
                    $self.profile.name, $label
                )))
            }
        }
    };
}

/// A generic SteelSeries device driven entirely by a [`DeviceProfile`].
/// Implements every capability trait in [`crate::device`]; each method
/// checks its corresponding `Option<XyzProfile>` field on the profile and
/// returns [`Error::Unsupported`] when the device doesn't have that
/// capability.
pub struct ProfiledDevice {
    hid: SteelSeriesHid,
    profile: &'static DeviceProfile,
    /// The product ID this instance was actually opened as. Kept separately
    /// from `profile.product_ids` since a profile may cover more than one
    /// product ID (e.g. a wired and a wireless variant of the same device).
    product_id: u16,
}

impl ProfiledDevice {
    /// Wrap an already-opened [`SteelSeriesHid`] with the profile describing
    /// its commands. `product_id` is the specific ID the device was opened
    /// as (must be one of `profile.product_ids`).
    pub fn new(hid: SteelSeriesHid, profile: &'static DeviceProfile, product_id: u16) -> Self {
        ProfiledDevice {
            hid,
            profile,
            product_id,
        }
    }

    /// The profile driving this device.
    pub fn profile(&self) -> &'static DeviceProfile {
        self.profile
    }
}

impl SteelSeriesDevice for ProfiledDevice {
    fn name(&self) -> &str {
        self.profile.name
    }

    fn vendor_id(&self) -> u16 {
        self.profile.vendor_id
    }

    fn product_id(&self) -> u16 {
        self.product_id
    }

    fn save(&self) -> Result<()> {
        self.hid
            .write_command(ReportType::Output, &[protocol::commands::SAVE], None)?;
        Ok(())
    }
}

impl ConfigurableDpi for ProfiledDevice {
    fn set_dpi(&self, values: &[u16]) -> Result<()> {
        let dpi = require_capability!(self, dpi, "configurable DPI");

        if values.is_empty() {
            return Err(Error::InvalidParameter(
                "At least one DPI value required".to_string(),
            ));
        }

        if values.len() > dpi.max_presets as usize {
            return Err(Error::InvalidParameter(format!(
                "Maximum {} DPI presets allowed",
                dpi.max_presets
            )));
        }

        // Build command: [CMD] [count] [selected_preset] [dpi1_low] [dpi1_high] [dpi2_low] ...
        let mut command = vec![dpi.command, values.len() as u8, 1];

        for &value in values {
            let byte = dpi_to_byte(dpi.mapping, value)?;
            command.push(byte);
            command.push(0x00); // High byte (always 0 for this range)
        }

        self.hid
            .write_command(ReportType::Output, &command, None)?;
        Ok(())
    }

    fn dpi_range(&self) -> (u16, u16, u16) {
        dpi_range_for(self.profile)
    }
}

impl ConfigurablePollingRate for ProfiledDevice {
    fn set_polling_rate(&self, hz: u16) -> Result<()> {
        let polling_rate = require_capability!(self, polling_rate, "configurable polling rate");
        let rate = PollingRate::from_hz(hz)?;
        let command = [polling_rate.command, rate.to_byte()];
        self.hid
            .write_command(ReportType::Output, &command, None)?;
        Ok(())
    }
}

/// Validates a sidetone level is in the 0-3 (Off/Low/Medium/High) range,
/// factored out as a plain function so it's testable without a live HID
/// handle — mirrors `dpi_to_byte`'s standalone-function shape.
fn validate_sidetone_level(level: u8) -> Result<()> {
    if level > 3 {
        return Err(Error::InvalidParameter(
            "Sidetone level must be 0-3".to_string(),
        ));
    }
    Ok(())
}

impl ConfigurableSidetone for ProfiledDevice {
    fn set_sidetone(&self, level: u8) -> Result<()> {
        validate_sidetone_level(level)?;
        let sidetone = require_capability!(self, sidetone, "configurable sidetone");
        self.hid
            .write_command(ReportType::Output, &[sidetone.command, level], None)?;
        Ok(())
    }
}

impl ProfiledDevice {
    /// Sends `[command]` (plus `extra_byte`, if any), reads back
    /// `readback_length` bytes, and hands the response to `parse`. Shared by
    /// every capability whose HID shape is "one command byte, one optional
    /// extra byte, one readback, one parse function" — currently
    /// `battery_status` and `chatmix_status`, which happen to target the
    /// identical physical report on the Arctis Nova 7 but are still issued
    /// as separate round-trips (see `ChatMixProfile`'s doc comment for why).
    fn query_report<T>(
        &self,
        command: u8,
        extra_byte: Option<u8>,
        readback_length: usize,
        parse: fn(&[u8]) -> Result<T>,
    ) -> Result<T> {
        let mut cmd = vec![command];
        if let Some(extra) = extra_byte {
            cmd.push(extra);
        }

        let response = self
            .hid
            .write_command(ReportType::Output, &cmd, Some(readback_length))?;

        parse(&response)
    }
}

impl WirelessDevice for ProfiledDevice {
    fn battery_status(&self) -> Result<BatteryStatus> {
        let battery = require_capability!(self, battery, "battery status");
        self.query_report(battery.command, battery.extra_byte, battery.readback_length, battery.parse)
    }
}

impl ChatMixReadable for ProfiledDevice {
    fn chatmix_status(&self) -> Result<ChatMixStatus> {
        let chatmix = require_capability!(self, chatmix, "chatmix status");
        self.query_report(chatmix.command, chatmix.extra_byte, chatmix.readback_length, chatmix.parse)
    }
}

impl ConfigurableButtons for ProfiledDevice {
    fn set_button_mapping(&self, mapping: &ButtonMapping) -> Result<()> {
        let buttons = require_capability!(self, buttons, "configurable button mapping");
        let mut command = vec![buttons.command];
        command.extend_from_slice(&mapping.encode());

        self.hid
            .write_command(ReportType::Output, &command, None)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device::rival3_wireless::RIVAL3_WIRELESS_PROFILE;

    #[test]
    fn test_dpi_to_byte_mapping() {
        let mapping = RIVAL3_WIRELESS_PROFILE.dpi.unwrap().mapping;
        assert_eq!(dpi_to_byte(mapping, 100).unwrap(), 0x00);
        assert_eq!(dpi_to_byte(mapping, 400).unwrap(), 0x04);
        assert_eq!(dpi_to_byte(mapping, 800).unwrap(), 0x09);
        assert_eq!(dpi_to_byte(mapping, 1600).unwrap(), 0x13);
        assert_eq!(dpi_to_byte(mapping, 3200).unwrap(), 0x27);
        assert_eq!(dpi_to_byte(mapping, 18000).unwrap(), 0xE0);
    }

    #[test]
    fn test_dpi_to_byte_invalid() {
        let mapping = RIVAL3_WIRELESS_PROFILE.dpi.unwrap().mapping;
        assert!(dpi_to_byte(mapping, 99).is_err());
        assert!(dpi_to_byte(mapping, 150).is_err());
        assert!(dpi_to_byte(mapping, 18100).is_err());
    }

    #[test]
    fn dpi_mapping_range_matches_known_bounds() {
        let mapping = RIVAL3_WIRELESS_PROFILE.dpi.unwrap().mapping;
        assert_eq!(dpi_mapping_range(mapping), (100, 18000, 100));
    }

    /// Guards the "in steps of N" claim shown as DPI help text in the GUI
    /// (`gui/src/main.rs::dpi_help_text`): if a DPI mapping table ever
    /// gained a gap, `dpi_mapping_range()`'s derived step would no longer
    /// describe every entry, and this test would catch it.
    #[test]
    fn dpi_mapping_is_a_uniform_arithmetic_sequence_matching_its_step() {
        let mapping = RIVAL3_WIRELESS_PROFILE.dpi.unwrap().mapping;
        let (_, _, step) = dpi_mapping_range(mapping);
        for pair in mapping.windows(2) {
            assert_eq!(
                pair[1].0 - pair[0].0,
                step,
                "gap between {:?} and {:?} doesn't match step {}",
                pair[0],
                pair[1],
                step
            );
        }
    }

    /// Regression guard: `ProfiledDevice::dpi_range()` (here, its
    /// `dpi_range_for` equivalent, which doesn't need a live HID handle)
    /// must always derive its range from the profile's own `DpiProfile`
    /// mapping, never silently fall through to `ConfigurableDpi::dpi_range`'s
    /// trait default of `(100, 18000, 100)`. That default happens to equal
    /// the real Rival 3 Wireless range, so a profile whose mapping produces
    /// a *different* range is needed to actually catch an implementation
    /// that forgot to consult `DpiProfile::mapping`. This exact bug class
    /// (a hardcoded DPI-range tuple ignoring the real mapping data) has
    /// shipped before.
    #[test]
    fn dpi_range_derives_from_profile_mapping_not_trait_default() {
        const TEST_MAPPING: &[(u16, u8)] = &[(50, 0x00), (100, 0x01), (150, 0x02)];
        const TEST_PROFILE: DeviceProfile = DeviceProfile {
            name: "Test Device",
            vendor_id: 0x1038,
            product_ids: &[0xffff],
            usage_page: 0xffc0,
            interface: None,
            battery: None,
            chatmix: None,
            dpi: Some(DpiProfile {
                command: 0x99,
                mapping: TEST_MAPPING,
                max_presets: 1,
            }),
            polling_rate: None,
            buttons: None,
            sidetone: None,
        };

        const TRAIT_DEFAULT_RANGE: (u16, u16, u16) = (100, 18000, 100);

        assert_eq!(dpi_range_for(&TEST_PROFILE), (50, 150, 50));
        assert_ne!(dpi_range_for(&TEST_PROFILE), TRAIT_DEFAULT_RANGE);
    }

    /// A profile with no DPI capability must report a range that can't be
    /// mistaken for a real one, rather than the trait's default.
    #[test]
    fn dpi_range_for_profile_without_dpi_is_zeroed() {
        const TEST_PROFILE: DeviceProfile = DeviceProfile {
            name: "Headset-only Device",
            vendor_id: 0x1038,
            product_ids: &[0xfffe],
            usage_page: 0xffc0,
            interface: None,
            battery: None,
            chatmix: None,
            dpi: None,
            polling_rate: None,
            buttons: None,
            sidetone: None,
        };

        assert_eq!(dpi_range_for(&TEST_PROFILE), (0, 0, 0));
    }

    #[test]
    fn set_sidetone_rejects_level_above_3() {
        assert!(validate_sidetone_level(4).is_err());
        assert!(matches!(
            validate_sidetone_level(4),
            Err(Error::InvalidParameter(_))
        ));
    }

    #[test]
    fn set_sidetone_accepts_level_0_to_3() {
        for level in 0..=3 {
            assert!(validate_sidetone_level(level).is_ok());
        }
    }
}
