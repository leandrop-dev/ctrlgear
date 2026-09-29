//! Declarative device profiles: a [`DeviceProfile`] is a `const` data
//! description that [`ProfiledDevice`] interprets to implement every
//! capability trait from [`crate::device`] generically.

use crate::buttons::ButtonMapping;
use crate::device::{
    ChatMixReadable, ConfigurableBluetooth, ConfigurableButtons, ConfigurableDpi,
    ConfigurableEqualizer, ConfigurableMicMuteLed, ConfigurableMicVolume, ConfigurablePollingRate,
    ConfigurableRgb, ConfigurableSidetone, ConfigurableSleepTimer, ConfigurableVolumeLimiter,
    SteelSeriesDevice, WirelessDevice,
};
use crate::hid::SteelSeriesHid;
use crate::protocol::{self, BatteryStatus, ChatMixStatus, ReportType};
use crate::{Error, Result};

/// Data needed to query battery/status: which command to send and how to
/// parse the reply.
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

/// Data needed to read chatmix, kept as its own struct rather than folded
/// into `BatteryProfile`.
#[derive(Clone, Copy)]
pub struct ChatMixProfile {
    pub command: u8,
    pub extra_byte: Option<u8>,
    pub readback_length: usize,
    pub parse: fn(&[u8]) -> Result<ChatMixStatus>,
}

/// Data needed to set DPI: the command byte, the DPI-value -> device-value
/// mapping table, and the maximum number of presets the device accepts.
#[derive(Clone, Copy)]
pub struct DpiProfile {
    pub command: u8,
    pub mapping: &'static [(u16, u16)],
    pub max_presets: u8,
    /// The "selected preset" index `set_dpi` requests as active.
    pub first_preset: u8,
    /// Number of little-endian bytes `set_dpi` writes per DPI value.
    pub value_width: u8,
}

/// Data needed to set polling rate: the command byte and the device's own
/// Hz -> device-byte mapping.
#[derive(Clone, Copy)]
pub struct PollingRateProfile {
    pub command: u8,
    pub choices: &'static [(u16, u8)],
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

/// Data needed to set microphone-mute LED brightness: just the command byte,
/// since the 0-3 (off to max) level range is shared across devices.
#[derive(Clone, Copy)]
pub struct MicMuteLedProfile {
    pub command: u8,
}

/// Data needed to set microphone input volume: just the command byte, since
/// the 0-7 (off to max) level range is shared across devices.
#[derive(Clone, Copy)]
pub struct MicVolumeProfile {
    pub command: u8,
}

/// Data needed to drive a multi-band equalizer: the command byte, the number
/// of adjustable bands, and the device's built-in presets (each a
/// `bands`-length gain array in dB), since both the band count and the
/// preset set are device-specific.
#[derive(Clone, Copy)]
pub struct EqualizerProfile {
    pub command: u8,
    pub bands: usize,
    /// Built-in presets, each a `bands`-length gain array in dB.
    pub presets: &'static [&'static [f32]],
}

/// Data needed to drive per-zone RGB lighting — see
/// [`crate::device::aerox_gen2_rgb`] for the concrete profiles.
#[derive(Clone, Copy)]
pub struct RgbProfile {
    /// Command byte for per-zone RGB color (`0x21` on every known device).
    pub zone_command: u8,
    /// Zone index -> the exact bytes sent between `zone_command` and the
    /// `[r, g, b]` triplet.
    pub zones: &'static [(u8, &'static [u8])],
    /// Command byte for the reactive (on-click) color, when present.
    pub reactive_command: Option<u8>,
    /// Command byte for LED brightness, when present.
    pub brightness_command: Option<u8>,
    /// Valid device-byte range for LED brightness (`(0x00, 0x64)` on every
    /// device that has one); meaningless when `brightness_command` is `None`.
    pub brightness_range: (u8, u8),
    /// Command byte(s) for the rainbow effect, when present.
    pub rainbow_command: Option<&'static [u8]>,
    /// Named rainbow-effect choices -> bitmask byte; empty when
    /// `rainbow_command` is `None`.
    pub rainbow_choices: &'static [(&'static str, u8)],
    /// Command byte for default (startup) lighting.
    pub default_lighting_command: u8,
    /// Named default-lighting choices -> the bytes sent after the command
    /// byte (2 bytes for the Aerox/Rival 3 Gen 2 family's 4-choice
    /// combinatorial scheme, 1 byte for the Prime Mini's simpler off/rainbow
    /// scheme).
    pub default_lighting_choices: &'static [(&'static str, &'static [u8])],
}

/// Data needed to set the auto-sleep (inactive time) timer: just the command
/// byte, since the 0-255 minutes range is shared across devices.
#[derive(Clone, Copy)]
pub struct SleepTimerProfile {
    pub command: u8,
}

/// Data needed to toggle the output volume limiter: just the command byte,
/// since the on/off range is shared across devices.
#[derive(Clone, Copy)]
pub struct VolumeLimiterProfile {
    pub command: u8,
}

/// Data needed to drive Bluetooth settings: the power-on-toggle command, the
/// call-volume-mode command, and the distinct save command the protocol
/// requires after setting Bluetooth-when-powered-on.
#[derive(Clone, Copy)]
pub struct BluetoothProfile {
    pub power_on_command: u8,
    pub call_volume_command: u8,
    pub save_command: [u8; 2],
}

/// Data needed to read the device's firmware version: which command to send
/// (plus an optional extra byte) and how many bytes to read back, since the
/// parsing itself is a function pointer — mirrors [`BatteryProfile`]'s shape.
#[derive(Clone, Copy)]
pub struct FirmwareVersionProfile {
    /// Command byte that requests the firmware version.
    pub command: u8,
    /// Extra byte sent immediately after `command`, if the device expects
    /// one. `None` when the command is sent standalone.
    pub extra_byte: Option<u8>,
    /// Number of bytes to read back.
    pub readback_length: usize,
    /// Parses the raw readback into a firmware version string.
    pub parse: fn(&[u8]) -> Result<String>,
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
    pub mic_mute_led: Option<MicMuteLedProfile>,
    pub mic_volume: Option<MicVolumeProfile>,
    pub equalizer: Option<EqualizerProfile>,
    pub sleep_timer: Option<SleepTimerProfile>,
    pub volume_limiter: Option<VolumeLimiterProfile>,
    pub bluetooth: Option<BluetoothProfile>,
    pub firmware_version: Option<FirmwareVersionProfile>,
    pub rgb: Option<RgbProfile>,
    /// Bytes sent (standalone, no readback) to persist settings to device
    /// memory. Defaults to `[0x09]` via [`DeviceProfile::NONE`].
    pub save_command: &'static [u8],
}

impl DeviceProfile {
    /// A base profile with every identity field zeroed and every capability
    /// `None`. Declare a device's profile with `..DeviceProfile::NONE` to
    /// pick up `None` for every capability field you don't explicitly set.
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
        mic_mute_led: None,
        mic_volume: None,
        equalizer: None,
        sleep_timer: None,
        volume_limiter: None,
        bluetooth: None,
        firmware_version: None,
        rgb: None,
        save_command: &[protocol::commands::SAVE],
    };
}

/// Derives (min, max, step) from a DPI mapping table itself.
fn dpi_mapping_range(mapping: &[(u16, u16)]) -> (u16, u16, u16) {
    let min = mapping.first().map(|&(dpi, _)| dpi).unwrap_or(0);
    let max = mapping.last().map(|&(dpi, _)| dpi).unwrap_or(0);
    let step = mapping
        .get(1)
        .map(|&(dpi, _)| dpi.saturating_sub(min))
        .unwrap_or(0);
    (min, max, step)
}

/// Computes the range [`ConfigurableDpi::dpi_range`] reports for a profile.
fn dpi_range_for(profile: &DeviceProfile) -> (u16, u16, u16) {
    match &profile.dpi {
        Some(dpi) => dpi_mapping_range(dpi.mapping),
        None => (0, 0, 0),
    }
}

/// Converts a DPI value to its raw device value using `mapping`.
fn dpi_to_value(mapping: &[(u16, u16)], dpi: u16) -> Result<u16> {
    mapping
        .iter()
        .find(|(d, _)| *d == dpi)
        .map(|(_, value)| *value)
        .ok_or_else(|| {
            let (min, max, step) = dpi_mapping_range(mapping);
            Error::InvalidParameter(format!(
                "Invalid DPI value: {}. Must be {}-{} in steps of {}",
                dpi, min, max, step
            ))
        })
}

/// Encodes a single DPI value into the little-endian byte(s) `set_dpi`
/// writes for it, per `value_width` (see [`DpiProfile::value_width`]).
fn encode_dpi_value(mapping: &[(u16, u16)], value_width: u8, dpi: u16) -> Result<Vec<u8>> {
    let raw = dpi_to_value(mapping, dpi)?;
    let mut bytes = vec![(raw & 0xFF) as u8];
    if value_width >= 2 {
        bytes.push((raw >> 8) as u8);
    }
    Ok(bytes)
}

/// Fetches `$self.profile.$field`, or short-circuits the enclosing method
/// with `Err(Error::Unsupported(..))` naming both the device and the
/// missing capability.
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
    /// The product ID this instance was actually opened as.
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
            .write_command(ReportType::Output, self.profile.save_command, None)?;
        Ok(())
    }

    fn firmware_version(&self) -> Result<String> {
        let firmware_version = require_capability!(self, firmware_version, "firmware version");
        self.query_report(
            firmware_version.command,
            firmware_version.extra_byte,
            firmware_version.readback_length,
            firmware_version.parse,
        )
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

        let mut command = vec![dpi.command, values.len() as u8, dpi.first_preset];

        for &value in values {
            command.extend(encode_dpi_value(dpi.mapping, dpi.value_width, value)?);
        }

        self.hid
            .write_command(ReportType::Output, &command, None)?;
        Ok(())
    }

    fn dpi_range(&self) -> (u16, u16, u16) {
        dpi_range_for(self.profile)
    }
}

/// Looks up a Hz value in a profile's `choices` table.
fn polling_rate_to_byte(choices: &[(u16, u8)], hz: u16) -> Result<u8> {
    choices
        .iter()
        .find(|(h, _)| *h == hz)
        .map(|(_, byte)| *byte)
        .ok_or_else(|| {
            Error::InvalidParameter(format!(
                "Invalid polling rate: {}. Valid options: {}",
                hz,
                choices.iter().map(|(h, _)| h.to_string()).collect::<Vec<_>>().join(", ")
            ))
        })
}

impl ConfigurablePollingRate for ProfiledDevice {
    fn set_polling_rate(&self, hz: u16) -> Result<()> {
        let polling_rate = require_capability!(self, polling_rate, "configurable polling rate");
        let byte = polling_rate_to_byte(polling_rate.choices, hz)?;
        let command = [polling_rate.command, byte];
        self.hid
            .write_command(ReportType::Output, &command, None)?;
        Ok(())
    }
}

/// Validates a sidetone level is in the 0-3 (Off/Low/Medium/High) range.
/// `pub` so callers with a device's product ID but no open HID handle yet
/// (e.g. the Tauri IPC layer) can validate before opening the device.
pub fn validate_sidetone_level(level: u8) -> Result<()> {
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

/// Validates a mic mute LED brightness level is in the 0-3 (off to max)
/// range. `pub` — see [`validate_sidetone_level`].
pub fn validate_mic_mute_led_level(level: u8) -> Result<()> {
    if level > 3 {
        return Err(Error::InvalidParameter(
            "Mic mute LED brightness must be 0-3".to_string(),
        ));
    }
    Ok(())
}

impl ConfigurableMicMuteLed for ProfiledDevice {
    fn set_mic_mute_led_brightness(&self, level: u8) -> Result<()> {
        validate_mic_mute_led_level(level)?;
        let mic_mute_led = require_capability!(self, mic_mute_led, "configurable mic mute LED");
        self.hid
            .write_command(ReportType::Output, &[mic_mute_led.command, level], None)?;
        Ok(())
    }
}

/// Validates a mic volume level is in the 0-7 (off to max) range. `pub` —
/// see [`validate_sidetone_level`].
pub fn validate_mic_volume_level(level: u8) -> Result<()> {
    if level > 7 {
        return Err(Error::InvalidParameter(
            "Mic volume must be 0-7".to_string(),
        ));
    }
    Ok(())
}

impl ConfigurableMicVolume for ProfiledDevice {
    fn set_mic_volume(&self, level: u8) -> Result<()> {
        validate_mic_volume_level(level)?;
        let mic_volume = require_capability!(self, mic_volume, "configurable mic volume");
        self.hid
            .write_command(ReportType::Output, &[mic_volume.command, level], None)?;
        Ok(())
    }
}

/// Validates equalizer band gains: the slice must have exactly
/// `expected_len` entries, each within `-10.0..=10.0` dB.
fn validate_equalizer_bands(bands: &[f32], expected_len: usize) -> Result<()> {
    if bands.len() != expected_len {
        return Err(Error::InvalidParameter(format!(
            "Expected {} equalizer band values, got {}",
            expected_len,
            bands.len()
        )));
    }

    if bands.iter().any(|&gain| !(-10.0..=10.0).contains(&gain)) {
        return Err(Error::InvalidParameter(
            "Equalizer band gain must be -10.0-10.0 dB".to_string(),
        ));
    }

    Ok(())
}

/// Encodes a gain in dB to the device's byte representation: `0x14` is the
/// 0 dB baseline, with each unit step representing 0.5 dB.
fn encode_equalizer_band(gain: f32) -> u8 {
    (0x14_i16 + (gain * 2.0).round() as i16).clamp(0, 255) as u8
}

/// Validates an equalizer preset index is within `equalizer.presets`. `pub`
/// so callers with a device's product ID but no open HID handle yet (e.g.
/// the Tauri IPC layer) can validate before opening the device.
pub fn validate_equalizer_preset(equalizer: &EqualizerProfile, preset: u8) -> Result<()> {
    if (preset as usize) < equalizer.presets.len() {
        Ok(())
    } else {
        Err(Error::InvalidParameter(format!(
            "Invalid equalizer preset: {}. Must be 0-{}",
            preset,
            equalizer.presets.len().saturating_sub(1)
        )))
    }
}

impl ConfigurableEqualizer for ProfiledDevice {
    fn equalizer_bands(&self) -> usize {
        self.profile.equalizer.map(|e| e.bands).unwrap_or(0)
    }

    fn set_equalizer(&self, bands: &[f32]) -> Result<()> {
        let equalizer = require_capability!(self, equalizer, "configurable equalizer");
        validate_equalizer_bands(bands, equalizer.bands)?;

        let mut command = vec![equalizer.command];
        command.extend(bands.iter().map(|&gain| encode_equalizer_band(gain)));
        command.push(0x00);

        self.hid.write_command(ReportType::Output, &command, None)?;
        Ok(())
    }

    fn set_equalizer_preset(&self, preset: u8) -> Result<()> {
        let equalizer = require_capability!(self, equalizer, "configurable equalizer");
        validate_equalizer_preset(equalizer, preset)?;
        self.set_equalizer(equalizer.presets[preset as usize])
    }
}

impl ConfigurableSleepTimer for ProfiledDevice {
    fn set_sleep_timer(&self, minutes: u8) -> Result<()> {
        let sleep_timer = require_capability!(self, sleep_timer, "configurable sleep timer");
        self.hid.write_command(ReportType::Output, &[sleep_timer.command, minutes], None)?;
        Ok(())
    }
}

impl ConfigurableVolumeLimiter for ProfiledDevice {
    fn set_volume_limiter(&self, enabled: bool) -> Result<()> {
        let volume_limiter = require_capability!(self, volume_limiter, "configurable volume limiter");
        self.hid
            .write_command(ReportType::Output, &[volume_limiter.command, enabled as u8], None)?;
        Ok(())
    }
}

/// Validates a Bluetooth call-volume mode is in the 0-2 range. `pub` — see
/// [`validate_sidetone_level`].
pub fn validate_bluetooth_call_volume_mode(mode: u8) -> Result<()> {
    if mode > 2 {
        return Err(Error::InvalidParameter(
            "Bluetooth call volume mode must be 0-2".to_string(),
        ));
    }
    Ok(())
}

impl ConfigurableBluetooth for ProfiledDevice {
    fn set_bluetooth_when_powered_on(&self, enabled: bool) -> Result<()> {
        let bluetooth = require_capability!(self, bluetooth, "configurable bluetooth settings");
        self.hid
            .write_command(ReportType::Output, &[bluetooth.power_on_command, enabled as u8], None)?;
        self.hid
            .write_command(ReportType::Output, &bluetooth.save_command, None)?;
        Ok(())
    }

    fn set_bluetooth_call_volume(&self, mode: u8) -> Result<()> {
        validate_bluetooth_call_volume_mode(mode)?;
        let bluetooth = require_capability!(self, bluetooth, "configurable bluetooth settings");
        self.hid
            .write_command(ReportType::Output, &[bluetooth.call_volume_command, mode], None)?;
        Ok(())
    }
}

/// Looks up `zone`'s byte-prefix in `zones`.
fn zone_prefix(zones: &[(u8, &'static [u8])], zone: u8) -> Result<&'static [u8]> {
    zones
        .iter()
        .find(|(z, _)| *z == zone)
        .map(|(_, prefix)| *prefix)
        .ok_or_else(|| {
            Error::InvalidParameter(format!(
                "Invalid RGB zone: {}. Must be one of: {}",
                zone,
                zones
                    .iter()
                    .map(|(z, _)| z.to_string())
                    .collect::<Vec<_>>()
                    .join(", ")
            ))
        })
}

/// Builds the `[zone_command] [zone prefix...] [r, g, b]` command for
/// setting one zone's color.
fn build_zone_color_command(rgb: &RgbProfile, zone: u8, r: u8, g: u8, b: u8) -> Result<Vec<u8>> {
    let prefix = zone_prefix(rgb.zones, zone)?;
    let mut command = vec![rgb.zone_command];
    command.extend_from_slice(prefix);
    command.extend_from_slice(&[r, g, b]);
    Ok(command)
}

/// Builds the reactive (on-click) color command: `[cmd, 0x01, 0x00, r, g,
/// b]` when enabled, `[cmd, 0x00, 0x00, 0x00, 0x00, 0x00]` when disabled.
fn build_reactive_color_command(reactive_command: u8, color: Option<(u8, u8, u8)>) -> Vec<u8> {
    let mut command = vec![reactive_command];
    match color {
        Some((r, g, b)) => command.extend_from_slice(&[0x01, 0x00, r, g, b]),
        None => command.extend_from_slice(&[0x00, 0x00, 0x00, 0x00, 0x00]),
    }
    command
}

/// Clamps an LED brightness value into a profile's device-byte
/// `brightness_range`.
fn clamp_led_brightness(range: (u8, u8), level: u8) -> u8 {
    level.clamp(range.0, range.1)
}

/// Looks up a named rainbow-effect choice's bitmask byte.
fn rainbow_choice_byte(choices: &[(&'static str, u8)], choice: &str) -> Result<u8> {
    choices
        .iter()
        .find(|(name, _)| *name == choice)
        .map(|(_, byte)| *byte)
        .ok_or_else(|| {
            Error::InvalidParameter(format!(
                "Invalid rainbow effect: {}. Must be one of: {}",
                choice,
                choices.iter().map(|(n, _)| *n).collect::<Vec<_>>().join(", ")
            ))
        })
}

/// Builds the rainbow-effect command: `[rainbow_command..., choice_byte]`.
fn build_rainbow_effect_command(
    rainbow_command: &[u8],
    choices: &[(&'static str, u8)],
    choice: &str,
) -> Result<Vec<u8>> {
    let byte = rainbow_choice_byte(choices, choice)?;
    let mut command = rainbow_command.to_vec();
    command.push(byte);
    Ok(command)
}

/// Looks up a named default-lighting choice's bytes.
fn default_lighting_bytes(
    choices: &[(&'static str, &'static [u8])],
    choice: &str,
) -> Result<&'static [u8]> {
    choices
        .iter()
        .find(|(name, _)| *name == choice)
        .map(|(_, bytes)| *bytes)
        .ok_or_else(|| {
            Error::InvalidParameter(format!(
                "Invalid default lighting: {}. Must be one of: {}",
                choice,
                choices.iter().map(|(n, _)| *n).collect::<Vec<_>>().join(", ")
            ))
        })
}

/// Builds the default-lighting command: `[default_lighting_command,
/// choice bytes...]`.
fn build_default_lighting_command(
    command_byte: u8,
    choices: &[(&'static str, &'static [u8])],
    choice: &str,
) -> Result<Vec<u8>> {
    let bytes = default_lighting_bytes(choices, choice)?;
    let mut command = vec![command_byte];
    command.extend_from_slice(bytes);
    Ok(command)
}

impl ConfigurableRgb for ProfiledDevice {
    fn set_zone_color(&self, zone: u8, r: u8, g: u8, b: u8) -> Result<()> {
        let rgb = require_capability!(self, rgb, "configurable RGB lighting");
        let command = build_zone_color_command(rgb, zone, r, g, b)?;
        self.hid.write_command(ReportType::Output, &command, None)?;
        Ok(())
    }

    fn set_reactive_color(&self, color: Option<(u8, u8, u8)>) -> Result<()> {
        let rgb = require_capability!(self, rgb, "configurable RGB lighting");
        let reactive_command = rgb.reactive_command.ok_or_else(|| {
            Error::Unsupported(format!("{} has no reactive RGB color", self.profile.name))
        })?;
        let command = build_reactive_color_command(reactive_command, color);
        self.hid.write_command(ReportType::Output, &command, None)?;
        Ok(())
    }

    fn set_led_brightness(&self, level: u8) -> Result<()> {
        let rgb = require_capability!(self, rgb, "configurable RGB lighting");
        let brightness_command = rgb.brightness_command.ok_or_else(|| {
            Error::Unsupported(format!(
                "{} has no configurable LED brightness",
                self.profile.name
            ))
        })?;
        let level = clamp_led_brightness(rgb.brightness_range, level);
        self.hid
            .write_command(ReportType::Output, &[brightness_command, level], None)?;
        Ok(())
    }

    fn set_rainbow_effect(&self, choice: &str) -> Result<()> {
        let rgb = require_capability!(self, rgb, "configurable RGB lighting");
        let rainbow_command = rgb.rainbow_command.ok_or_else(|| {
            Error::Unsupported(format!("{} has no rainbow effect", self.profile.name))
        })?;
        let command = build_rainbow_effect_command(rainbow_command, rgb.rainbow_choices, choice)?;
        self.hid.write_command(ReportType::Output, &command, None)?;
        Ok(())
    }

    fn set_default_lighting(&self, choice: &str) -> Result<()> {
        let rgb = require_capability!(self, rgb, "configurable RGB lighting");
        let command = build_default_lighting_command(
            rgb.default_lighting_command,
            rgb.default_lighting_choices,
            choice,
        )?;
        self.hid.write_command(ReportType::Output, &command, None)?;
        Ok(())
    }
}

impl ProfiledDevice {
    /// Sends `[command]` (plus `extra_byte`, if any), reads back
    /// `readback_length` bytes, and hands the response to `parse`.
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
    use crate::device::rival3_wireless::{parse_firmware_version, RIVAL3_WIRELESS_PROFILE};

    #[test]
    fn test_dpi_to_byte_mapping() {
        let mapping = RIVAL3_WIRELESS_PROFILE.dpi.unwrap().mapping;
        assert_eq!(dpi_to_value(mapping, 100).unwrap(), 0x00);
        assert_eq!(dpi_to_value(mapping, 400).unwrap(), 0x04);
        assert_eq!(dpi_to_value(mapping, 800).unwrap(), 0x09);
        assert_eq!(dpi_to_value(mapping, 1600).unwrap(), 0x13);
        assert_eq!(dpi_to_value(mapping, 3200).unwrap(), 0x27);
        assert_eq!(dpi_to_value(mapping, 18000).unwrap(), 0xE0);
    }

    #[test]
    fn test_dpi_to_byte_invalid() {
        let mapping = RIVAL3_WIRELESS_PROFILE.dpi.unwrap().mapping;
        assert!(dpi_to_value(mapping, 99).is_err());
        assert!(dpi_to_value(mapping, 150).is_err());
        assert!(dpi_to_value(mapping, 18100).is_err());
    }

    /// Regression guard for the Prime Mini's 2-byte, formula-based DPI
    /// encoding.
    #[test]
    fn encode_dpi_value_writes_two_bytes_for_prime_mini_value_width() {
        use crate::device::aerox_gen2_rgb::PRIME_MINI_PROFILE;
        let dpi = PRIME_MINI_PROFILE.dpi.unwrap();
        assert_eq!(encode_dpi_value(dpi.mapping, dpi.value_width, 50).unwrap(), vec![0x01, 0x00]);
        assert_eq!(encode_dpi_value(dpi.mapping, dpi.value_width, 18000).unwrap(), vec![0x68, 0x01]);
    }

    /// Regression guard for the Aerox family's 1-byte-per-value DPI
    /// encoding.
    #[test]
    fn encode_dpi_value_writes_one_byte_for_aerox_value_width() {
        use crate::device::aerox_gen2_rgb::AEROX3_PROFILE;
        let dpi = AEROX3_PROFILE.dpi.unwrap();
        assert_eq!(encode_dpi_value(dpi.mapping, dpi.value_width, 200).unwrap(), vec![0x04]);
        assert_eq!(encode_dpi_value(dpi.mapping, dpi.value_width, 8500).unwrap(), vec![0xC5]);
    }

    /// Regression guard: Rival 3 Wireless and the Aerox/Rival 3 Gen
    /// 2/Prime Mini family use different (shifted-by-one) polling-rate byte
    /// mappings.
    #[test]
    fn polling_rate_to_byte_uses_profiles_own_choices() {
        let rival3_choices = RIVAL3_WIRELESS_PROFILE.polling_rate.unwrap().choices;
        assert_eq!(polling_rate_to_byte(rival3_choices, 125).unwrap(), 0x03);
        assert_eq!(polling_rate_to_byte(rival3_choices, 1000).unwrap(), 0x00);

        use crate::device::aerox_gen2_rgb::AEROX3_PROFILE;
        let aerox_choices = AEROX3_PROFILE.polling_rate.unwrap().choices;
        assert_eq!(polling_rate_to_byte(aerox_choices, 125).unwrap(), 0x04);
        assert_eq!(polling_rate_to_byte(aerox_choices, 1000).unwrap(), 0x01);
    }

    #[test]
    fn polling_rate_to_byte_rejects_unsupported_hz() {
        let choices = RIVAL3_WIRELESS_PROFILE.polling_rate.unwrap().choices;
        assert!(polling_rate_to_byte(choices, 999).is_err());
    }

    #[test]
    fn dpi_mapping_range_matches_known_bounds() {
        let mapping = RIVAL3_WIRELESS_PROFILE.dpi.unwrap().mapping;
        assert_eq!(dpi_mapping_range(mapping), (100, 18000, 100));
    }

    /// Guards the "in steps of N" claim shown as DPI help text in the GUI.
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
    /// trait default of `(100, 18000, 100)`.
    #[test]
    fn dpi_range_derives_from_profile_mapping_not_trait_default() {
        const TEST_MAPPING: &[(u16, u16)] = &[(50, 0x00), (100, 0x01), (150, 0x02)];
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
                first_preset: 1,
                value_width: 1,
            }),
            polling_rate: None,
            buttons: None,
            sidetone: None,
            mic_mute_led: None,
            mic_volume: None,
            equalizer: None,
            sleep_timer: None,
            volume_limiter: None,
            bluetooth: None,
            firmware_version: None,
            rgb: None,
            save_command: &[protocol::commands::SAVE],
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
            mic_mute_led: None,
            mic_volume: None,
            equalizer: None,
            sleep_timer: None,
            volume_limiter: None,
            bluetooth: None,
            firmware_version: None,
            rgb: None,
            save_command: &[protocol::commands::SAVE],
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

    #[test]
    fn set_mic_mute_led_rejects_level_above_3() {
        assert!(validate_mic_mute_led_level(4).is_err());
        assert!(matches!(
            validate_mic_mute_led_level(4),
            Err(Error::InvalidParameter(_))
        ));
    }

    #[test]
    fn set_mic_mute_led_accepts_level_0_to_3() {
        for level in 0..=3 {
            assert!(validate_mic_mute_led_level(level).is_ok());
        }
    }

    #[test]
    fn set_mic_volume_rejects_level_above_7() {
        assert!(validate_mic_volume_level(8).is_err());
        assert!(matches!(
            validate_mic_volume_level(8),
            Err(Error::InvalidParameter(_))
        ));
    }

    #[test]
    fn set_mic_volume_accepts_level_0_to_7() {
        for level in 0..=7 {
            assert!(validate_mic_volume_level(level).is_ok());
        }
    }

    #[test]
    fn validate_equalizer_bands_rejects_wrong_length() {
        let bands = [0.0; 9];
        assert!(validate_equalizer_bands(&bands, 10).is_err());
        assert!(matches!(
            validate_equalizer_bands(&bands, 10),
            Err(Error::InvalidParameter(_))
        ));
    }

    #[test]
    fn validate_equalizer_bands_rejects_out_of_range_gain() {
        let mut bands = [0.0; 10];
        bands[3] = 10.1;
        assert!(validate_equalizer_bands(&bands, 10).is_err());
        assert!(matches!(
            validate_equalizer_bands(&bands, 10),
            Err(Error::InvalidParameter(_))
        ));

        let mut bands = [0.0; 10];
        bands[7] = -10.1;
        assert!(validate_equalizer_bands(&bands, 10).is_err());
    }

    #[test]
    fn validate_equalizer_bands_accepts_valid_input() {
        let bands = [-10.0, -5.0, 0.0, 5.0, 10.0, 1.0, -1.0, 2.5, -2.5, 0.0];
        assert!(validate_equalizer_bands(&bands, 10).is_ok());
    }

    #[test]
    fn encode_equalizer_band_zero_gain_is_baseline() {
        assert_eq!(encode_equalizer_band(0.0), 0x14);
    }

    #[test]
    fn validate_equalizer_preset_accepts_every_index_in_range() {
        use crate::device::arctis_nova_7::ARCTIS_NOVA_7_PROFILE;
        let equalizer = ARCTIS_NOVA_7_PROFILE.equalizer.unwrap();
        for preset in 0..equalizer.presets.len() as u8 {
            assert!(validate_equalizer_preset(&equalizer, preset).is_ok());
        }
    }

    #[test]
    fn validate_equalizer_preset_rejects_index_past_last_preset() {
        use crate::device::arctis_nova_7::ARCTIS_NOVA_7_PROFILE;
        let equalizer = ARCTIS_NOVA_7_PROFILE.equalizer.unwrap();
        let err = validate_equalizer_preset(&equalizer, equalizer.presets.len() as u8).unwrap_err();
        assert!(matches!(err, Error::InvalidParameter(_)));
    }

    #[test]
    fn set_bluetooth_call_volume_rejects_mode_above_2() {
        assert!(validate_bluetooth_call_volume_mode(3).is_err());
        assert!(matches!(
            validate_bluetooth_call_volume_mode(3),
            Err(Error::InvalidParameter(_))
        ));
    }

    #[test]
    fn set_bluetooth_call_volume_accepts_mode_0_to_2() {
        for mode in 0..=2 {
            assert!(validate_bluetooth_call_volume_mode(mode).is_ok());
        }
    }

    #[test]
    fn encode_equalizer_band_extremes_stay_in_range() {
        assert_eq!(encode_equalizer_band(-10.0), 0);
        assert_eq!(encode_equalizer_band(10.0), 0x28);
    }

    #[test]
    fn parse_firmware_version_formats_major_minor() {
        let response = [1, 12];
        assert_eq!(parse_firmware_version(&response).unwrap(), "1.12");
    }

    use crate::device::aerox_gen2_rgb::{AEROX3_PROFILE, PRIME_MINI_PROFILE};

    #[test]
    fn build_zone_color_command_matches_aerox_zone_table() {
        let rgb = AEROX3_PROFILE.rgb.unwrap();
        assert_eq!(
            build_zone_color_command(&rgb, 1, 0x10, 0x20, 0x30).unwrap(),
            vec![0x21, 0x01, 0x10, 0x20, 0x30]
        );
        assert_eq!(
            build_zone_color_command(&rgb, 2, 0x10, 0x20, 0x30).unwrap(),
            vec![0x21, 0x02, 0x00, 0x00, 0x00, 0x10, 0x20, 0x30]
        );
        assert_eq!(
            build_zone_color_command(&rgb, 3, 0x10, 0x20, 0x30).unwrap(),
            vec![0x21, 0x04, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x10, 0x20, 0x30]
        );
    }

    #[test]
    fn build_zone_color_command_matches_prime_mini_single_zone() {
        let rgb = PRIME_MINI_PROFILE.rgb.unwrap();
        assert_eq!(
            build_zone_color_command(&rgb, 1, 0xff, 0x00, 0x00).unwrap(),
            vec![0x21, 0x00, 0xff, 0x00, 0x00]
        );
    }

    #[test]
    fn build_zone_color_command_rejects_unknown_zone() {
        let rgb = AEROX3_PROFILE.rgb.unwrap();
        assert!(build_zone_color_command(&rgb, 4, 0, 0, 0).is_err());
    }

    #[test]
    fn build_reactive_color_command_matches_rivalcfg_reactive_rgbcolor_handler() {
        assert_eq!(
            build_reactive_color_command(0x26, Some((0xff, 0x00, 0x00))),
            vec![0x26, 0x01, 0x00, 0xff, 0x00, 0x00]
        );
        assert_eq!(
            build_reactive_color_command(0x26, None),
            vec![0x26, 0x00, 0x00, 0x00, 0x00, 0x00]
        );
    }

    #[test]
    fn clamp_led_brightness_clamps_to_0x00_0x64() {
        assert_eq!(clamp_led_brightness((0x00, 0x64), 0), 0x00);
        assert_eq!(clamp_led_brightness((0x00, 0x64), 100), 0x64);
        assert_eq!(clamp_led_brightness((0x00, 0x64), 255), 0x64);
        assert_eq!(clamp_led_brightness((0x00, 0x64), 50), 50);
    }

    #[test]
    fn build_rainbow_effect_command_matches_aerox_bitmask_choices() {
        let rgb = AEROX3_PROFILE.rgb.unwrap();
        let rainbow_command = rgb.rainbow_command.unwrap();
        assert_eq!(
            build_rainbow_effect_command(rainbow_command, rgb.rainbow_choices, "all").unwrap(),
            vec![0x22, 0b111]
        );
        assert_eq!(
            build_rainbow_effect_command(rainbow_command, rgb.rainbow_choices, "bottom-top").unwrap(),
            vec![0x22, 0b101]
        );
        assert!(build_rainbow_effect_command(rainbow_command, rgb.rainbow_choices, "nope").is_err());
    }

    #[test]
    fn build_default_lighting_command_matches_aerox_four_choice_scheme() {
        let rgb = AEROX3_PROFILE.rgb.unwrap();
        assert_eq!(
            build_default_lighting_command(rgb.default_lighting_command, rgb.default_lighting_choices, "off").unwrap(),
            vec![0x27, 0x00, 0x00]
        );
        assert_eq!(
            build_default_lighting_command(rgb.default_lighting_command, rgb.default_lighting_choices, "reactive-rainbow").unwrap(),
            vec![0x27, 0x01, 0x01]
        );
    }

    #[test]
    fn build_default_lighting_command_matches_prime_mini_two_choice_scheme() {
        let rgb = PRIME_MINI_PROFILE.rgb.unwrap();
        assert_eq!(
            build_default_lighting_command(rgb.default_lighting_command, rgb.default_lighting_choices, "off").unwrap(),
            vec![0x27, 0x00]
        );
        assert_eq!(
            build_default_lighting_command(rgb.default_lighting_command, rgb.default_lighting_choices, "rainbow").unwrap(),
            vec![0x27, 0x01]
        );
    }

    /// The Prime Mini's `RgbProfile` genuinely has no reactive color,
    /// brightness or rainbow effect.
    #[test]
    fn prime_mini_rgb_profile_has_no_reactive_brightness_or_rainbow() {
        let rgb = PRIME_MINI_PROFILE.rgb.unwrap();
        assert!(rgb.reactive_command.is_none());
        assert!(rgb.brightness_command.is_none());
        assert!(rgb.rainbow_command.is_none());
    }
}
