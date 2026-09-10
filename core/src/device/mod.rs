//! Device implementations for SteelSeries hardware

pub mod arctis_nova_7;
pub mod rival3_wireless;

use crate::Result;

/// Common interface for all SteelSeries devices
pub trait SteelSeriesDevice {
    /// Get the device name
    fn name(&self) -> &str;

    /// Get vendor ID
    fn vendor_id(&self) -> u16;

    /// Get product ID
    fn product_id(&self) -> u16;

    /// Save current settings to device memory
    fn save(&self) -> Result<()>;

    /// Reset device to factory defaults
    fn reset(&self) -> Result<()> {
        Err(crate::Error::Unsupported(
            "Reset not supported for this device".to_string(),
        ))
    }
}

/// Common interface for mice with configurable DPI
pub trait ConfigurableDpi {
    /// Set DPI values (supports multiple presets)
    fn set_dpi(&self, values: &[u16]) -> Result<()>;

    /// Get the valid DPI range for this device
    fn dpi_range(&self) -> (u16, u16, u16) {
        // (min, max, step)
        (100, 18000, 100)
    }
}

/// Common interface for mice with configurable polling rate
pub trait ConfigurablePollingRate {
    /// Set polling rate in Hz
    fn set_polling_rate(&self, hz: u16) -> Result<()>;
}

/// Common interface for wireless devices
pub trait WirelessDevice {
    /// Get battery status
    fn battery_status(&self) -> Result<crate::protocol::BatteryStatus>;
}

/// Common interface for devices with configurable button mapping
pub trait ConfigurableButtons {
    /// Set button mapping
    fn set_button_mapping(&self, mapping: &crate::buttons::ButtonMapping) -> Result<()>;

    /// Get the default button mapping for this device
    fn default_button_mapping(&self) -> crate::buttons::ButtonMapping {
        crate::buttons::ButtonMapping::default_mapping()
    }
}
