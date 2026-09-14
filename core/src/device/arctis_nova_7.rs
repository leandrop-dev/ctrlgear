//! SteelSeries Arctis Nova 7 Wireless headset profile data.
//!
//! Battery-only: this headset has no DPI/polling-rate/button concepts, so
//! its [`DeviceProfile`] leaves those fields `None` —
//! [`crate::device::profile::ProfiledDevice`] returns [`Error::Unsupported`]
//! for them generically.
//!
//! Protocol verified against the connected hardware (product ID `0x22a1`,
//! reported on interface 3 / usage page `0xffc0`) and cross-checked against
//! the community-maintained implementation in
//! [HeadsetControl](https://github.com/Sapd/HeadsetControl)
//! (`lib/devices/steelseries_arctis_nova_7.hpp`, `protocols/steelseries_protocol.hpp`).
//!
//! Only the "Gen 2" percentage-battery firmware (product ID `0x22a1`) is
//! implemented. Older Nova 7 units (e.g. `0x2202`) report battery as a
//! discrete 0-4 level requiring a different mapping to percent — out of
//! scope until that variant needs support.

use crate::device::profile::{BatteryProfile, DeviceProfile};
use crate::protocol::{BatteryStatus, STEELSERIES_VENDOR_ID};
use crate::{Error, Result};

/// Product ID for Arctis Nova 7 Wireless (percentage-battery firmware),
/// reported when connected via the 2.4GHz USB dongle.
pub const PRODUCT_ID: u16 = 0x22a1;

/// Product ID reported when the same headset is connected directly via a
/// USB cable (e.g. for charging) instead of the wireless dongle. Confirmed
/// on this machine's hardware (`lsusb`/`ctrlgear-cli list-devices`): a
/// second `1038:22a0` interface appears alongside `1038:22a1` while the
/// headset is plugged in both ways at once, also exposing a `0xffc0`
/// configuration interface — same protocol shape, different product ID.
/// Used today only to label connection type
/// (`registry::ConnectionType::UsbWired`); not yet wired into `is_supported`/
/// `open`, so it isn't treated as a configurable device in its own right.
pub const WIRED_PRODUCT_ID: u16 = 0x22a0;

/// Status request command byte; the device replies with a status report
/// whose byte 2 is battery percent (0-100) and byte 3 is charge/connection
/// state (`0x00` offline, `0x01`/`0x02` charging, otherwise on battery).
const STATUS_REQUEST: u8 = 0xb0;

/// Upper bound on the status report SteelSeries devices in this family send
/// back; the actual read is truncated to whatever the device returns.
const STATUS_RESPONSE_BUF_LEN: usize = 128;

/// Parses an Arctis Nova 7 status report into a [`BatteryStatus`]: byte 2 is
/// the battery percent, byte 3 the charge/connection state (`0x00` means
/// offline, which is surfaced as [`Error::DeviceNotFound`] rather than a
/// zero-battery reading).
fn parse_status(response: &[u8]) -> Result<BatteryStatus> {
    if response.len() < 4 {
        return Err(Error::InvalidParameter(
            "Invalid battery status response".to_string(),
        ));
    }

    const OFFLINE: u8 = 0x00;
    const CHARGING_A: u8 = 0x01;
    const CHARGING_B: u8 = 0x02;

    if response[3] == OFFLINE {
        return Err(Error::DeviceNotFound(
            "Headset not connected or turned off".to_string(),
        ));
    }

    Ok(BatteryStatus {
        level: response[2].min(100),
        is_charging: response[3] == CHARGING_A || response[3] == CHARGING_B,
    })
}

/// SteelSeries Arctis Nova 7 device profile. Battery-only: `dpi`,
/// `polling_rate` and `buttons` all come from `DeviceProfile::NONE`.
pub const ARCTIS_NOVA_7_PROFILE: DeviceProfile = DeviceProfile {
    name: "SteelSeries Arctis Nova 7",
    vendor_id: STEELSERIES_VENDOR_ID,
    product_ids: &[PRODUCT_ID],
    usage_page: 0xffc0,
    battery: Some(BatteryProfile {
        command: STATUS_REQUEST,
        extra_byte: None,
        readback_length: STATUS_RESPONSE_BUF_LEN,
        parse: parse_status,
    }),
    ..DeviceProfile::NONE
};
