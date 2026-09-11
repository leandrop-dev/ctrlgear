//! Discovery and instantiation of connected SteelSeries devices.
//!
//! This is the single place that knows which product IDs map to which
//! concrete device implementation. Adding support for a new device means
//! adding a variant here and to [`Device`] — nowhere else.

use crate::device::arctis_nova_7::{
    ArctisNova7, PRODUCT_ID as ARCTIS_NOVA_7_PRODUCT_ID, WIRED_PRODUCT_ID as ARCTIS_NOVA_7_WIRED_PRODUCT_ID,
};
use crate::device::rival3_wireless::{Rival3Wireless, PRODUCT_ID as RIVAL3_WIRELESS_PRODUCT_ID};
use crate::device::{ConfigurableButtons, ConfigurableDpi, ConfigurablePollingRate, SteelSeriesDevice, WirelessDevice};
use crate::hid::SteelSeriesHid;
use crate::protocol::{BatteryStatus, STEELSERIES_VENDOR_ID};
use crate::{Error, Result};
use hidapi::HidApi;

/// A concrete, opened SteelSeries device.
///
/// A thin enum rather than a trait object: matching on two variants is
/// simpler than a `dyn` trait bound spanning `SteelSeriesDevice` +
/// `WirelessDevice` + `ConfigurableDpi`, and not every device implements
/// every trait (a headset has no DPI).
pub enum Device {
    Rival3Wireless(Rival3Wireless),
    ArctisNova7(ArctisNova7),
}

impl Device {
    pub fn name(&self) -> &str {
        match self {
            Device::Rival3Wireless(d) => d.name(),
            Device::ArctisNova7(d) => d.name(),
        }
    }

    pub fn product_id(&self) -> u16 {
        match self {
            Device::Rival3Wireless(d) => d.product_id(),
            Device::ArctisNova7(d) => d.product_id(),
        }
    }

    pub fn battery_status(&self) -> Result<BatteryStatus> {
        match self {
            Device::Rival3Wireless(d) => d.battery_status(),
            Device::ArctisNova7(d) => d.battery_status(),
        }
    }

    /// Whether this device has a configurable DPI (mice only).
    pub fn supports_dpi(&self) -> bool {
        matches!(self, Device::Rival3Wireless(_))
    }

    /// Whether this device has a configurable polling rate (mice only).
    pub fn supports_polling_rate(&self) -> bool {
        matches!(self, Device::Rival3Wireless(_))
    }

    /// Whether this device has configurable button mapping (mice only).
    pub fn supports_button_mapping(&self) -> bool {
        matches!(self, Device::Rival3Wireless(_))
    }

    /// Valid DPI range as (min, max, step), for devices with configurable DPI.
    pub fn dpi_range(&self) -> Option<(u16, u16, u16)> {
        match self {
            Device::Rival3Wireless(d) => Some(d.dpi_range()),
            Device::ArctisNova7(_) => None,
        }
    }

    pub fn set_dpi(&self, values: &[u16]) -> Result<()> {
        match self {
            Device::Rival3Wireless(d) => d.set_dpi(values),
            Device::ArctisNova7(_) => Err(Error::Unsupported(
                "Arctis Nova 7 has no configurable DPI".to_string(),
            )),
        }
    }

    pub fn set_polling_rate(&self, hz: u16) -> Result<()> {
        match self {
            Device::Rival3Wireless(d) => d.set_polling_rate(hz),
            Device::ArctisNova7(_) => Err(Error::Unsupported(
                "Arctis Nova 7 has no configurable polling rate".to_string(),
            )),
        }
    }

    pub fn set_button_mapping(&self, mapping: &crate::buttons::ButtonMapping) -> Result<()> {
        match self {
            Device::Rival3Wireless(d) => d.set_button_mapping(mapping),
            Device::ArctisNova7(_) => Err(Error::Unsupported(
                "Arctis Nova 7 has no configurable button mapping".to_string(),
            )),
        }
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
    matches!(
        product_id,
        RIVAL3_WIRELESS_PRODUCT_ID | ARCTIS_NOVA_7_PRODUCT_ID
    )
}

/// Open a supported device by product ID.
pub fn open(api: &HidApi, product_id: u16) -> Result<Device> {
    match product_id {
        RIVAL3_WIRELESS_PRODUCT_ID => {
            let hid = SteelSeriesHid::open(api, STEELSERIES_VENDOR_ID, product_id)?;
            Ok(Device::Rival3Wireless(Rival3Wireless::new(hid)))
        }
        ARCTIS_NOVA_7_PRODUCT_ID => {
            let hid = SteelSeriesHid::open(api, STEELSERIES_VENDOR_ID, product_id)?;
            Ok(Device::ArctisNova7(ArctisNova7::new(hid)))
        }
        _ => Err(Error::Unsupported(format!(
            "No configuration support for device {:04x}:{:04x}",
            STEELSERIES_VENDOR_ID, product_id
        ))),
    }
}

/// A connected SteelSeries device, with battery status already queried for
/// devices that support it. Meant for populating a device list/grid UI.
pub struct DeviceSummary {
    pub product_id: u16,
    pub name: String,
    pub supported: bool,
    pub supports_dpi: bool,
    pub supports_polling_rate: bool,
    pub supports_button_mapping: bool,
    pub dpi_range: Option<(u16, u16, u16)>,
    pub connection_type: ConnectionType,
    pub battery: Option<BatteryStatus>,
    /// Set when the device is supported but a query (e.g. battery) failed,
    /// or left as the raw HID product string for unsupported devices.
    pub note: Option<String>,
}

/// Enumerate every connected SteelSeries device on its configuration
/// interface (usage_page 0xffc0), opening and querying battery status for
/// the ones with a concrete [`Device`] implementation.
pub fn discover(api: &HidApi) -> Vec<DeviceSummary> {
    let mut seen = std::collections::HashSet::new();

    api.device_list()
        .filter(|d| d.vendor_id() == STEELSERIES_VENDOR_ID && d.usage_page() == 0xffc0)
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
                    dpi_range: None,
                    connection_type,
                    battery: None,
                    note: None,
                };
            }

            match open(api, product_id) {
                Ok(device) => {
                    let name = device.name().to_string();
                    let supports_dpi = device.supports_dpi();
                    let supports_polling_rate = device.supports_polling_rate();
                    let supports_button_mapping = device.supports_button_mapping();
                    let dpi_range = device.dpi_range();
                    match device.battery_status() {
                        Ok(status) => DeviceSummary {
                            product_id,
                            name,
                            supported: true,
                            supports_dpi,
                            supports_polling_rate,
                            supports_button_mapping,
                            dpi_range,
                            connection_type,
                            battery: Some(status),
                            note: None,
                        },
                        Err(e) => DeviceSummary {
                            product_id,
                            name,
                            supported: true,
                            supports_dpi,
                            supports_polling_rate,
                            supports_button_mapping,
                            dpi_range,
                            connection_type,
                            battery: None,
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
                    dpi_range: None,
                    connection_type,
                    battery: None,
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
}
