//! Discovery and instantiation of connected SteelSeries devices.
//!
//! This is the single place that knows which product IDs map to which
//! [`DeviceProfile`]. Adding support for a new device means adding a
//! `const DeviceProfile` (see `device::rival3_wireless` / `device::arctis_nova_7`
//! for examples) and listing it in [`PROFILES`] — nowhere else.

use crate::device::arctis_nova_7::{
    ARCTIS_NOVA_7_PROFILE, PRODUCT_ID as ARCTIS_NOVA_7_PRODUCT_ID,
    WIRED_PRODUCT_ID as ARCTIS_NOVA_7_WIRED_PRODUCT_ID,
};
use crate::device::profile::{DeviceProfile, ProfiledDevice};
use crate::device::rival3_wireless::{RIVAL3_WIRELESS_PROFILE, PRODUCT_ID as RIVAL3_WIRELESS_PRODUCT_ID};
use crate::device::{ChatMixReadable, ConfigurableButtons, ConfigurableDpi, ConfigurableMicMuteLed, ConfigurableMicVolume, ConfigurablePollingRate, ConfigurableSidetone, SteelSeriesDevice, WirelessDevice};
use crate::hid::SteelSeriesHid;
use crate::protocol::{BatteryStatus, ChatMixStatus, STEELSERIES_VENDOR_ID};
use crate::{Error, Result};
use hidapi::HidApi;

/// Every known device profile, matched by product ID in [`open`],
/// [`is_supported`] and [`discover`]. Adding a device family means adding
/// its `const DeviceProfile` here.
pub const PROFILES: &[DeviceProfile] = &[RIVAL3_WIRELESS_PROFILE, ARCTIS_NOVA_7_PROFILE];

/// Looks up the profile claiming `product_id`, if any.
fn profile_for(product_id: u16) -> Option<&'static DeviceProfile> {
    PROFILES.iter().find(|p| p.product_ids.contains(&product_id))
}

/// A concrete, opened SteelSeries device: a [`ProfiledDevice`] plus the
/// capability-flag/range accessors [`DeviceSummary`] needs, derived from the
/// device's [`DeviceProfile`] rather than matched on a device type.
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

    /// Valid DPI range as (min, max, step), for devices with configurable DPI.
    pub fn dpi_range(&self) -> Option<(u16, u16, u16)> {
        if self.inner.profile().dpi.is_some() {
            Some(self.inner.dpi_range())
        } else {
            None
        }
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
}

/// How a device is physically reached. A device that can be reached over
/// more than one transport at once (e.g. a wireless dongle plus a USB
/// charging cable) legitimately produces one [`DeviceSummary`] per product
/// ID — see [`discover`]'s per-product-ID dedup — so this exists to label
/// each row rather than to collapse them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionType {
    UsbWired,
    Dongle24Ghz,
    Bluetooth,
    Unknown,
}

/// Pure mapping from a HID enumeration's bus type + product ID to a
/// [`ConnectionType`]. `hidapi::BusType::Usb` alone can't distinguish a
/// direct USB cable from a 2.4GHz USB dongle receiver (both enumerate as
/// USB at the bus level) — only the product ID can, which is why known
/// wired-mode product IDs (e.g. [`ARCTIS_NOVA_7_WIRED_PRODUCT_ID`]) are
/// matched explicitly here rather than derived generically.
fn connection_type_for(bus: hidapi::BusType, product_id: u16) -> ConnectionType {
    match bus {
        hidapi::BusType::Bluetooth => ConnectionType::Bluetooth,
        hidapi::BusType::Usb => match product_id {
            ARCTIS_NOVA_7_WIRED_PRODUCT_ID => ConnectionType::UsbWired,
            RIVAL3_WIRELESS_PRODUCT_ID | ARCTIS_NOVA_7_PRODUCT_ID => ConnectionType::Dongle24Ghz,
            _ => ConnectionType::Unknown,
        },
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
#[derive(Clone)]
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
    pub dpi_range: Option<(u16, u16, u16)>,
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
                    dpi_range: None,
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
                    let dpi_range = device.dpi_range();
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
                            dpi_range,
                            connection_type,
                            battery: Some(status),
                            chatmix,
                            // Battery succeeded, so surface a chatmix failure here instead
                            // of dropping it silently — there's nothing more important to
                            // report in this arm. When battery also fails (the other arm),
                            // its error takes priority and the chatmix error is dropped.
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
                            dpi_range,
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
                    dpi_range: None,
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

    /// Guards against a silent product-ID collision across [`PROFILES`]: two
    /// profiles claiming the same ID would make [`profile_for`] (and so
    /// [`open`]) silently pick whichever comes first in the table.
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
    /// `dpi_range` must reflect each profile's actual capability fields —
    /// this is what gates every capability-specific section in the GUI, so a
    /// wrong flag here breaks the UI without a compile error.
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
}
