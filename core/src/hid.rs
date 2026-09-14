//! HID communication wrapper

use crate::protocol::{ReportType, DEFAULT_COMMAND_DELAY_MS};
use crate::{Error, Result};
use hidapi::{HidApi, HidDevice};
use std::time::Duration;

/// Wrapper around HID device with SteelSeries-specific communication logic
pub struct SteelSeriesHid {
    device: HidDevice,
    command_delay: Duration,
}

impl SteelSeriesHid {
    /// Open a SteelSeries device on the correct interface
    ///
    /// SteelSeries devices have multiple HID interfaces. We need to open the
    /// one matching `usage_page` (the device's configuration interface) and,
    /// when `interface` is `Some`, also matching that HID interface number —
    /// some device families need both to disambiguate the configuration
    /// interface from others sharing the same usage page.
    pub fn open(
        api: &HidApi,
        vendor_id: u16,
        product_id: u16,
        usage_page: u16,
        interface: Option<u8>,
    ) -> Result<Self> {
        let device_path = api
            .device_list()
            .find(|dev| {
                dev.vendor_id() == vendor_id
                    && dev.product_id() == product_id
                    && dev.usage_page() == usage_page
                    && interface.is_none_or(|iface| dev.interface_number() == iface as i32)
            })
            .map(|dev| dev.path())
            .ok_or_else(|| {
                Error::DeviceNotFound(format!(
                    "SteelSeries device {:04x}:{:04x} not found or wrong interface",
                    vendor_id, product_id
                ))
            })?;

        let device = api.open_path(device_path).map_err(|e| {
            Error::DeviceNotFound(format!(
                "Failed to open device {:04x}:{:04x}: {}",
                vendor_id, product_id, e
            ))
        })?;

        Ok(SteelSeriesHid {
            device,
            command_delay: Duration::from_millis(DEFAULT_COMMAND_DELAY_MS),
        })
    }

    /// Set custom command delay (useful for testing or specific devices)
    pub fn set_command_delay(&mut self, delay: Duration) {
        self.command_delay = delay;
    }

    /// Write a HID report to the device
    ///
    /// # Arguments
    /// * `report_type` - Type of HID report (Output or Feature)
    /// * `command` - Command bytes to send
    /// * `readback_length` - Optional number of bytes to read back after command
    pub fn write_command(
        &self,
        report_type: ReportType,
        command: &[u8],
        readback_length: Option<usize>,
    ) -> Result<Vec<u8>> {
        // Prepare the buffer: [Report ID (0x00)] + command bytes
        let mut buf = Vec::with_capacity(1 + command.len());
        buf.push(0x00); // Report ID
        buf.extend_from_slice(command);

        // Send the command
        match report_type {
            ReportType::Output => {
                self.device.write(&buf)?;
            }
            ReportType::Feature => {
                self.device.send_feature_report(&buf)?;
            }
        }

        // Sleep to prevent device overload
        if self.command_delay.as_millis() > 0 {
            std::thread::sleep(self.command_delay);
        }

        // Read response if requested
        if let Some(len) = readback_length {
            let mut response = vec![0u8; len];
            let bytes_read = self.device.read_timeout(&mut response, 5000)?;

            if bytes_read == 0 {
                return Err(Error::Timeout);
            }

            response.truncate(bytes_read);
            Ok(response)
        } else {
            Ok(Vec::new())
        }
    }

    /// Get the underlying HID device (for advanced operations)
    pub fn device(&self) -> &HidDevice {
        &self.device
    }
}

/// Check if a device is currently plugged in
pub fn is_device_plugged(api: &HidApi, vendor_id: u16, product_id: u16) -> bool {
    api.device_list()
        .any(|dev| dev.vendor_id() == vendor_id && dev.product_id() == product_id)
}

/// Find the first available SteelSeries device from a list of product IDs
pub fn find_first_device(
    api: &HidApi,
    vendor_id: u16,
    product_ids: &[u16],
) -> Option<(u16, String)> {
    for device_info in api.device_list() {
        if device_info.vendor_id() == vendor_id && product_ids.contains(&device_info.product_id()) {
            let product_string = device_info
                .product_string()
                .unwrap_or("Unknown")
                .to_string();
            return Some((device_info.product_id(), product_string));
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hid_api_initialization() {
        // Just verify we can create HidApi
        let api = HidApi::new();
        assert!(api.is_ok());
    }
}
