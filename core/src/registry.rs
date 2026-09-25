//! Discovery and instantiation of connected SteelSeries devices.

use crate::device::aerox_gen2_rgb::{
    AEROX3_PROFILE, AEROX5_PROFILE, PRIME_MINI_PROFILE, RIVAL3_GEN2_PROFILE,
};
use crate::device::arctis_nova_7::{
    self, ARCTIS_NOVA_7_DISCRETE_BATTERY_PROFILE, ARCTIS_NOVA_7_PROFILE,
    WIRED_PRODUCT_ID as ARCTIS_NOVA_7_WIRED_PRODUCT_ID,
};
use crate::device::profile::{DeviceProfile, ProfiledDevice};
use crate::device::rival3_wireless::{RIVAL3_WIRELESS_PROFILE, PRODUCT_ID as RIVAL3_WIRELESS_PRODUCT_ID};
use crate::device::{ChatMixReadable, ConfigurableBluetooth, ConfigurableButtons, ConfigurableDpi, ConfigurableEqualizer, ConfigurableMicMuteLed, ConfigurableMicVolume, ConfigurablePollingRate, ConfigurableRgb, ConfigurableSidetone, ConfigurableSleepTimer, ConfigurableVolumeLimiter, SteelSeriesDevice, WirelessDevice};
use crate::hid::SteelSeriesHid;
use crate::protocol::{BatteryStatus, ChatMixStatus, STEELSERIES_VENDOR_ID};
use crate::{Error, Result};
use hidapi::HidApi;

/// Every known device profile, matched by product ID in [`open`],
/// [`is_supported`] and [`discover`]. Adding a device family means adding
/// its `const DeviceProfile` here.
pub const PROFILES: &[DeviceProfile] = &[
    RIVAL3_WIRELESS_PROFILE,
    ARCTIS_NOVA_7_PROFILE,
    ARCTIS_NOVA_7_DISCRETE_BATTERY_PROFILE,
    AEROX3_PROFILE,
    AEROX5_PROFILE,
    RIVAL3_GEN2_PROFILE,
    PRIME_MINI_PROFILE,
];

/// Looks up the profile claiming `product_id`, if any. Pure/static — safe to
/// call without an open HID handle (e.g. for input validation before opening
/// a device).
pub fn profile_for(product_id: u16) -> Option<&'static DeviceProfile> {
    PROFILES.iter().find(|p| p.product_ids.contains(&product_id))
}

/// A concrete, opened SteelSeries device: a [`ProfiledDevice`] plus the
/// capability-flag/range accessors [`DeviceSummary`] needs.
pub struct Device {
    inner: ProfiledDevice,
}

impl Device {
    pub fn name(&self) -> &str {
        self.inner.name()
    }

    pub fn vendor_id(&self) -> u16 {
        self.inner.vendor_id()
    }

    pub fn product_id(&self) -> u16 {
        self.inner.product_id()
    }

    pub fn save(&self) -> Result<()> {
        self.inner.save()
    }

    pub fn battery_status(&self) -> Result<BatteryStatus> {
        self.inner.battery_status()
    }

    pub fn chatmix_status(&self) -> Result<ChatMixStatus> {
        self.inner.chatmix_status()
    }

    pub fn firmware_version(&self) -> Result<String> {
        self.inner.firmware_version()
    }

    /// Whether this device has a configurable DPI (mice only).
    pub fn supports_dpi(&self) -> bool {
        self.inner.profile().dpi.is_some()
    }

    /// Whether this device has a configurable polling rate (mice only).
    pub fn supports_polling_rate(&self) -> bool {
        self.inner.profile().polling_rate.is_some()
    }

    /// Whether this device has configurable button mapping (mice only).
    pub fn supports_button_mapping(&self) -> bool {
        self.inner.profile().buttons.is_some()
    }

    /// Whether this device has a readable chatmix (headsets only).
    pub fn supports_chatmix(&self) -> bool {
        self.inner.profile().chatmix.is_some()
    }

    /// Whether this device has a readable battery status (wireless devices only).
    pub fn supports_battery(&self) -> bool {
        self.inner.profile().battery.is_some()
    }

    /// Whether this device has a configurable sidetone level (headsets only).
    pub fn supports_sidetone(&self) -> bool {
        self.inner.profile().sidetone.is_some()
    }

    /// Whether this device has a configurable mic-mute LED brightness (headsets only).
    pub fn supports_mic_mute_led(&self) -> bool {
        self.inner.profile().mic_mute_led.is_some()
    }

    /// Whether this device has a configurable mic volume (headsets only).
    pub fn supports_mic_volume(&self) -> bool {
        self.inner.profile().mic_volume.is_some()
    }

    /// Whether this device has a configurable equalizer (headsets only).
    pub fn supports_equalizer(&self) -> bool {
        self.inner.profile().equalizer.is_some()
    }

    /// Whether this device has a configurable sleep timer (wireless devices only).
    pub fn supports_sleep_timer(&self) -> bool {
        self.inner.profile().sleep_timer.is_some()
    }

    /// Whether this device has a configurable volume limiter (headsets only).
    pub fn supports_volume_limiter(&self) -> bool {
        self.inner.profile().volume_limiter.is_some()
    }

    /// Whether this device has configurable Bluetooth settings (headsets only).
    pub fn supports_bluetooth_settings(&self) -> bool {
        self.inner.profile().bluetooth.is_some()
    }

    /// Whether this device has a readable firmware version.
    pub fn supports_firmware_version(&self) -> bool {
        self.inner.profile().firmware_version.is_some()
    }

    /// Whether this device has configurable per-zone RGB lighting (mice only).
    pub fn supports_rgb(&self) -> bool {
        self.inner.profile().rgb.is_some()
    }

    /// Number of RGB zones, for devices with configurable RGB lighting.
    pub fn rgb_zone_count(&self) -> usize {
        self.inner.profile().rgb.map(|rgb| rgb.zones.len()).unwrap_or(0)
    }

    /// Whether this device's RGB lighting has a reactive (on-click) color.
    pub fn supports_rgb_reactive_color(&self) -> bool {
        self.inner.profile().rgb.is_some_and(|rgb| rgb.reactive_command.is_some())
    }

    /// Whether this device's RGB lighting has a configurable brightness.
    pub fn supports_rgb_brightness(&self) -> bool {
        self.inner.profile().rgb.is_some_and(|rgb| rgb.brightness_command.is_some())
    }

    /// Whether this device's RGB lighting has a rainbow effect.
    pub fn supports_rgb_rainbow(&self) -> bool {
        self.inner.profile().rgb.is_some_and(|rgb| rgb.rainbow_command.is_some())
    }

    /// Named rainbow-effect choices, for devices with a rainbow effect.
    pub fn rgb_rainbow_choices(&self) -> &'static [(&'static str, u8)] {
        self.inner.profile().rgb.map(|rgb| rgb.rainbow_choices).unwrap_or(&[])
    }

    /// Named default-lighting choices, for devices with configurable RGB lighting.
    pub fn rgb_default_lighting_choices(&self) -> &'static [(&'static str, &'static [u8])] {
        self.inner.profile().rgb.map(|rgb| rgb.default_lighting_choices).unwrap_or(&[])
    }

    /// Number of adjustable equalizer bands, for devices with a configurable equalizer.
    pub fn equalizer_bands(&self) -> usize {
        self.inner.equalizer_bands()
    }

    /// Valid DPI range as (min, max, step), for devices with configurable DPI.
    pub fn dpi_range(&self) -> Option<(u16, u16, u16)> {
        if self.inner.profile().dpi.is_some() {
            Some(self.inner.dpi_range())
        } else {
            None
        }
    }

    /// Maximum number of DPI presets accepted, for devices with configurable DPI.
    pub fn dpi_max_presets(&self) -> Option<u8> {
        self.inner.profile().dpi.map(|dpi| dpi.max_presets)
    }

    /// Valid polling-rate choices in Hz, for devices with a configurable
    /// polling rate; empty when unsupported.
    pub fn polling_rate_choices(&self) -> Vec<u16> {
        self.inner
            .profile()
            .polling_rate
            .map(|p| p.choices.iter().map(|(hz, _)| *hz).collect())
            .unwrap_or_default()
    }

    pub fn set_dpi(&self, values: &[u16]) -> Result<()> {
        self.inner.set_dpi(values)
    }

    pub fn set_polling_rate(&self, hz: u16) -> Result<()> {
        self.inner.set_polling_rate(hz)
    }

    pub fn set_button_mapping(&self, mapping: &crate::buttons::ButtonMapping) -> Result<()> {
        self.inner.set_button_mapping(mapping)
    }

    pub fn set_sidetone(&self, level: u8) -> Result<()> {
        self.inner.set_sidetone(level)
    }

    pub fn set_mic_mute_led_brightness(&self, level: u8) -> Result<()> {
        self.inner.set_mic_mute_led_brightness(level)
    }

    pub fn set_mic_volume(&self, level: u8) -> Result<()> {
        self.inner.set_mic_volume(level)
    }

    pub fn set_equalizer(&self, bands: &[f32]) -> Result<()> {
        self.inner.set_equalizer(bands)
    }

    pub fn set_equalizer_preset(&self, preset: u8) -> Result<()> {
        self.inner.set_equalizer_preset(preset)
    }

    pub fn set_sleep_timer(&self, minutes: u8) -> Result<()> {
        self.inner.set_sleep_timer(minutes)
    }

    pub fn set_volume_limiter(&self, enabled: bool) -> Result<()> {
        self.inner.set_volume_limiter(enabled)
    }

    pub fn set_bluetooth_when_powered_on(&self, enabled: bool) -> Result<()> {
        self.inner.set_bluetooth_when_powered_on(enabled)
    }

    pub fn set_bluetooth_call_volume(&self, mode: u8) -> Result<()> {
        self.inner.set_bluetooth_call_volume(mode)
    }

    pub fn set_rgb_zone_color(&self, zone: u8, r: u8, g: u8, b: u8) -> Result<()> {
        self.inner.set_zone_color(zone, r, g, b)
    }

    pub fn set_rgb_reactive_color(&self, color: Option<(u8, u8, u8)>) -> Result<()> {
        self.inner.set_reactive_color(color)
    }

    pub fn set_rgb_brightness(&self, level: u8) -> Result<()> {
        self.inner.set_led_brightness(level)
    }

    pub fn set_rgb_rainbow_effect(&self, choice: &str) -> Result<()> {
        self.inner.set_rainbow_effect(choice)
    }

    pub fn set_rgb_default_lighting(&self, choice: &str) -> Result<()> {
        self.inner.set_default_lighting(choice)
    }
}

/// How a device is physically reached.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub enum ConnectionType {
    UsbWired,
    Dongle24Ghz,
    Bluetooth,
    Unknown,
}

/// Pure mapping from a HID enumeration's bus type + product ID to a
/// [`ConnectionType`].
fn connection_type_for(bus: hidapi::BusType, product_id: u16) -> ConnectionType {
    match bus {
        hidapi::BusType::Bluetooth => ConnectionType::Bluetooth,
        hidapi::BusType::Usb => {
            if product_id == ARCTIS_NOVA_7_WIRED_PRODUCT_ID {
                ConnectionType::UsbWired
            } else if product_id == RIVAL3_WIRELESS_PRODUCT_ID
                || arctis_nova_7::PERCENTAGE_BATTERY_PRODUCT_IDS.contains(&product_id)
                || arctis_nova_7::DISCRETE_BATTERY_PRODUCT_IDS.contains(&product_id)
            {
                ConnectionType::Dongle24Ghz
            } else {
                ConnectionType::Unknown
            }
        }
        _ => ConnectionType::Unknown,
    }
}

/// Whether `product_id` has a concrete [`Device`] implementation.
pub fn is_supported(product_id: u16) -> bool {
    profile_for(product_id).is_some()
}

/// Open a supported device by product ID.
pub fn open(api: &HidApi, product_id: u16) -> Result<Device> {
    let profile = profile_for(product_id).ok_or_else(|| {
        Error::Unsupported(format!(
            "No configuration support for device {:04x}:{:04x}",
            STEELSERIES_VENDOR_ID, product_id
        ))
    })?;

    let hid = SteelSeriesHid::open(
        api,
        profile.vendor_id,
        product_id,
        profile.usage_page,
        profile.interface,
    )?;

    Ok(Device {
        inner: ProfiledDevice::new(hid, profile, product_id),
    })
}

/// A connected SteelSeries device, with battery status already queried for
/// devices that support it. Meant for populating a device list/grid UI.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct DeviceSummary {
    pub product_id: u16,
    pub name: String,
    pub supported: bool,
    pub supports_dpi: bool,
    pub supports_polling_rate: bool,
    pub supports_button_mapping: bool,
    pub supports_sidetone: bool,
    pub supports_mic_mute_led: bool,
    pub supports_mic_volume: bool,
    pub supports_equalizer: bool,
    pub supports_sleep_timer: bool,
    pub supports_volume_limiter: bool,
    pub supports_bluetooth_settings: bool,
    pub supports_firmware: bool,
    pub supports_rgb: bool,
    /// Exported to TS as `number` (`u32`) — specta forbids exporting `usize` directly.
    #[cfg_attr(feature = "specta", specta(type = u32))]
    pub rgb_zone_count: usize,
    pub supports_rgb_brightness: bool,
    pub supports_rgb_rainbow: bool,
    /// Named default-lighting choices (e.g. `["off", "reactive", "rainbow",
    /// "reactive-rainbow"]`), for populating the GUI's dropdown; empty when
    /// `supports_rgb` is `false`.
    pub rgb_default_lighting_choices: Vec<String>,
    pub dpi_range: Option<(u16, u16, u16)>,
    /// Maximum number of DPI presets accepted; `None` when `supports_dpi` is `false`.
    pub dpi_max_presets: Option<u8>,
    /// Valid polling-rate choices in Hz; empty when `supports_polling_rate` is `false`.
    pub polling_rate_choices: Vec<u16>,
    pub connection_type: ConnectionType,
    pub battery: Option<BatteryStatus>,
    pub chatmix: Option<ChatMixStatus>,
    /// Set when the device is supported but a query (e.g. battery) failed,
    /// or left as the raw HID product string for unsupported devices.
    pub note: Option<String>,
}

/// Enumerate every connected SteelSeries device on a configuration
/// interface (any usage page declared by [`PROFILES`]), opening and
/// querying battery status for the ones with a concrete [`Device`]
/// implementation.
pub fn discover(api: &HidApi) -> Vec<DeviceSummary> {
    let mut seen = std::collections::HashSet::new();
    let usage_pages: Vec<u16> = PROFILES.iter().map(|p| p.usage_page).collect();

    api.device_list()
        .filter(|d| d.vendor_id() == STEELSERIES_VENDOR_ID && usage_pages.contains(&d.usage_page()))
        .filter(|d| seen.insert(d.product_id()))
        .map(|d| {
            let product_id = d.product_id();
            let connection_type = connection_type_for(d.bus_type(), product_id);

            if !is_supported(product_id) {
                return DeviceSummary {
                    product_id,
                    name: d
                        .product_string()
                        .unwrap_or("Unknown SteelSeries device")
                        .to_string(),
                    supported: false,
                    supports_dpi: false,
                    supports_polling_rate: false,
                    supports_button_mapping: false,
                    supports_sidetone: false,
                    supports_mic_mute_led: false,
                    supports_mic_volume: false,
                    supports_equalizer: false,
                    supports_sleep_timer: false,
                    supports_volume_limiter: false,
                    supports_bluetooth_settings: false,
                    supports_firmware: false,
                    supports_rgb: false,
                    rgb_zone_count: 0,
                    supports_rgb_brightness: false,
                    supports_rgb_rainbow: false,
                    rgb_default_lighting_choices: Vec::new(),
                    dpi_range: None,
                    dpi_max_presets: None,
                    polling_rate_choices: Vec::new(),
                    connection_type,
                    battery: None,
                    chatmix: None,
                    note: None,
                };
            }

            match open(api, product_id) {
                Ok(device) => {
                    let name = device.name().to_string();
                    let supports_dpi = device.supports_dpi();
                    let supports_polling_rate = device.supports_polling_rate();
                    let supports_button_mapping = device.supports_button_mapping();
                    let supports_sidetone = device.supports_sidetone();
                    let supports_mic_mute_led = device.supports_mic_mute_led();
                    let supports_mic_volume = device.supports_mic_volume();
                    let supports_equalizer = device.supports_equalizer();
                    let supports_sleep_timer = device.supports_sleep_timer();
                    let supports_volume_limiter = device.supports_volume_limiter();
                    let supports_bluetooth_settings = device.supports_bluetooth_settings();
                    let supports_firmware = device.supports_firmware_version();
                    let supports_rgb = device.supports_rgb();
                    let rgb_zone_count = device.rgb_zone_count();
                    let supports_rgb_brightness = device.supports_rgb_brightness();
                    let supports_rgb_rainbow = device.supports_rgb_rainbow();
                    let rgb_default_lighting_choices: Vec<String> = device
                        .rgb_default_lighting_choices()
                        .iter()
                        .map(|(name, _)| name.to_string())
                        .collect();
                    let dpi_range = device.dpi_range();
                    let dpi_max_presets = device.dpi_max_presets();
                    let polling_rate_choices = device.polling_rate_choices();
                    let supports_chatmix = device.supports_chatmix();
                    let chatmix_result = supports_chatmix.then(|| device.chatmix_status());
                    let chatmix = chatmix_result.as_ref().and_then(|r| r.as_ref().ok()).copied();
                    match device.battery_status() {
                        Ok(status) => DeviceSummary {
                            product_id,
                            name,
                            supported: true,
                            supports_dpi,
                            supports_polling_rate,
                            supports_button_mapping,
                            supports_sidetone,
                            supports_mic_mute_led,
                            supports_mic_volume,
                            supports_equalizer,
                            supports_sleep_timer,
                            supports_volume_limiter,
                            supports_bluetooth_settings,
                            supports_firmware,
                            supports_rgb,
                            rgb_zone_count,
                            supports_rgb_brightness,
                            supports_rgb_rainbow,
                            rgb_default_lighting_choices,
                            dpi_range,
                            dpi_max_presets,
                            polling_rate_choices: polling_rate_choices.clone(),
                            connection_type,
                            battery: Some(status),
                            chatmix,
                            note: chatmix_result.and_then(|r| r.err()).map(|e| e.to_string()),
                        },
                        Err(e) => DeviceSummary {
                            product_id,
                            name,
                            supported: true,
                            supports_dpi,
                            supports_polling_rate,
                            supports_button_mapping,
                            supports_sidetone,
                            supports_mic_mute_led,
                            supports_mic_volume,
                            supports_equalizer,
                            supports_sleep_timer,
                            supports_volume_limiter,
                            supports_bluetooth_settings,
                            supports_firmware,
                            supports_rgb,
                            rgb_zone_count,
                            supports_rgb_brightness,
                            supports_rgb_rainbow,
                            rgb_default_lighting_choices,
                            dpi_range,
                            dpi_max_presets,
                            polling_rate_choices,
                            connection_type,
                            battery: None,
                            chatmix,
                            note: Some(e.to_string()),
                        },
                    }
                }
                Err(e) => DeviceSummary {
                    product_id,
                    name: d
                        .product_string()
                        .unwrap_or("Unknown SteelSeries device")
                        .to_string(),
                    supported: true,
                    supports_dpi: false,
                    supports_polling_rate: false,
                    supports_button_mapping: false,
                    supports_sidetone: false,
                    supports_mic_mute_led: false,
                    supports_mic_volume: false,
                    supports_equalizer: false,
                    supports_sleep_timer: false,
                    supports_volume_limiter: false,
                    supports_bluetooth_settings: false,
                    supports_firmware: false,
                    supports_rgb: false,
                    rgb_zone_count: 0,
                    supports_rgb_brightness: false,
                    supports_rgb_rainbow: false,
                    rgb_default_lighting_choices: Vec::new(),
                    dpi_range: None,
                    dpi_max_presets: None,
                    polling_rate_choices: Vec::new(),
                    connection_type,
                    battery: None,
                    chatmix: None,
                    note: Some(e.to_string()),
                },
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device::arctis_nova_7::PRODUCT_ID as ARCTIS_NOVA_7_PRODUCT_ID;

    #[test]
    fn connection_type_for_bluetooth_bus_is_always_bluetooth() {
        assert_eq!(
            connection_type_for(hidapi::BusType::Bluetooth, 0xffff),
            ConnectionType::Bluetooth
        );
    }

    #[test]
    fn connection_type_for_known_wired_product_id_is_usb_wired() {
        assert_eq!(
            connection_type_for(hidapi::BusType::Usb, ARCTIS_NOVA_7_WIRED_PRODUCT_ID),
            ConnectionType::UsbWired
        );
    }

    #[test]
    fn connection_type_for_known_wireless_product_ids_is_dongle() {
        assert_eq!(
            connection_type_for(hidapi::BusType::Usb, ARCTIS_NOVA_7_PRODUCT_ID),
            ConnectionType::Dongle24Ghz
        );
        assert_eq!(
            connection_type_for(hidapi::BusType::Usb, RIVAL3_WIRELESS_PRODUCT_ID),
            ConnectionType::Dongle24Ghz
        );
    }

    #[test]
    fn connection_type_for_unknown_usb_product_id_is_unknown() {
        assert_eq!(connection_type_for(hidapi::BusType::Usb, 0xffff), ConnectionType::Unknown);
    }

    #[test]
    fn profile_for_known_product_id_returns_its_profile() {
        assert_eq!(profile_for(RIVAL3_WIRELESS_PRODUCT_ID).unwrap().name, RIVAL3_WIRELESS_PROFILE.name);
    }

    #[test]
    fn profile_for_unknown_product_id_returns_none() {
        assert!(profile_for(0xffff).is_none());
    }

    /// Guards against a silent product-ID collision across [`PROFILES`].
    #[test]
    fn every_profile_product_id_is_unique() {
        let mut seen = std::collections::HashSet::new();
        for profile in PROFILES {
            for &id in profile.product_ids {
                assert!(
                    seen.insert(id),
                    "duplicate product_id 0x{:04x} claimed by more than one DeviceProfile",
                    id
                );
            }
        }
    }

    /// `supports_dpi`/`supports_polling_rate`/`supports_button_mapping`/
    /// `dpi_range` must reflect each profile's actual capability fields.
    #[test]
    fn rival3_wireless_profile_declares_mouse_capabilities() {
        assert!(RIVAL3_WIRELESS_PROFILE.dpi.is_some());
        assert!(RIVAL3_WIRELESS_PROFILE.polling_rate.is_some());
        assert!(RIVAL3_WIRELESS_PROFILE.buttons.is_some());
        assert!(RIVAL3_WIRELESS_PROFILE.battery.is_some());
    }

    #[test]
    fn arctis_nova_7_profile_declares_battery_and_chatmix_only() {
        assert!(ARCTIS_NOVA_7_PROFILE.battery.is_some());
        assert!(ARCTIS_NOVA_7_PROFILE.chatmix.is_some());
        assert!(ARCTIS_NOVA_7_PROFILE.dpi.is_none());
        assert!(ARCTIS_NOVA_7_PROFILE.polling_rate.is_none());
        assert!(ARCTIS_NOVA_7_PROFILE.buttons.is_none());
    }

    #[test]
    fn arctis_nova_7_profile_declares_sidetone_but_rival3_does_not() {
        assert!(ARCTIS_NOVA_7_PROFILE.sidetone.is_some());
        assert!(RIVAL3_WIRELESS_PROFILE.sidetone.is_none());
    }

    #[test]
    fn arctis_nova_7_profile_declares_mic_mute_led_and_mic_volume_but_rival3_does_not() {
        assert!(ARCTIS_NOVA_7_PROFILE.mic_mute_led.is_some());
        assert!(ARCTIS_NOVA_7_PROFILE.mic_volume.is_some());
        assert!(RIVAL3_WIRELESS_PROFILE.mic_mute_led.is_none());
        assert!(RIVAL3_WIRELESS_PROFILE.mic_volume.is_none());
    }

    #[test]
    fn arctis_nova_7_profile_declares_equalizer_but_rival3_does_not() {
        assert!(ARCTIS_NOVA_7_PROFILE.equalizer.is_some());
        assert!(RIVAL3_WIRELESS_PROFILE.equalizer.is_none());
    }

    #[test]
    fn arctis_nova_7_profile_declares_sleep_timer_volume_limiter_and_bluetooth_but_rival3_does_not() {
        assert!(ARCTIS_NOVA_7_PROFILE.sleep_timer.is_some());
        assert!(ARCTIS_NOVA_7_PROFILE.volume_limiter.is_some());
        assert!(ARCTIS_NOVA_7_PROFILE.bluetooth.is_some());
        assert!(RIVAL3_WIRELESS_PROFILE.sleep_timer.is_none());
        assert!(RIVAL3_WIRELESS_PROFILE.volume_limiter.is_none());
        assert!(RIVAL3_WIRELESS_PROFILE.bluetooth.is_none());
    }

    /// Firmware version is a Rival 3 Wireless capability only — the Arctis
    /// Nova 7 profile has no `firmware_version` field set.
    #[test]
    fn rival3_wireless_profile_declares_firmware_version_but_arctis_nova_7_does_not() {
        assert!(RIVAL3_WIRELESS_PROFILE.firmware_version.is_some());
        assert!(ARCTIS_NOVA_7_PROFILE.firmware_version.is_none());
    }
}
