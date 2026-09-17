//! SteelSeries Arctis Nova 7 Wireless headset profile data.
//!
//! Battery, chatmix (game/chat volume balance), sidetone, mic-mute-LED
//! brightness, mic volume, a 10-band equalizer, sleep timer, volume limiter
//! and Bluetooth settings only: this headset has no DPI/polling-rate/button
//! concepts, so its [`DeviceProfile`] leaves those fields `None` —
//! [`crate::device::profile::ProfiledDevice`] returns
//! [`Error::Unsupported`] for them generically.
//!
//! Protocol verified against the connected hardware and cross-checked
//! against a community-maintained implementation.
//!
//! Two battery-report protocols are implemented across this family's 12
//! known product IDs, as [`ARCTIS_NOVA_7_PROFILE`] and
//! [`ARCTIS_NOVA_7_DISCRETE_BATTERY_PROFILE`] — see
//! [`PERCENTAGE_BATTERY_PRODUCT_IDS`] and [`DISCRETE_BATTERY_PRODUCT_IDS`]
//! for the full ID lists.

use crate::device::profile::{
    BatteryProfile, BluetoothProfile, ChatMixProfile, DeviceProfile, EqualizerProfile,
    MicMuteLedProfile, MicVolumeProfile, SidetoneProfile, SleepTimerProfile, VolumeLimiterProfile,
};
use crate::protocol::{commands, BatteryStatus, ChatMixStatus, STEELSERIES_VENDOR_ID};
use crate::{Error, Result};

/// Product ID for Arctis Nova 7 Wireless (percentage-battery firmware),
/// reported when connected via the 2.4GHz USB dongle.
pub const PRODUCT_ID: u16 = 0x22a1;

/// Product IDs using the Gen 2 percentage battery protocol (response[2] is 0-100 directly)
pub const PERCENTAGE_BATTERY_PRODUCT_IDS: &[u16] = &[0x22a1, 0x227e, 0x2258, 0x229e, 0x22ad, 0x22a9, 0x22a5];
/// Product IDs using the original discrete battery protocol (response[2] is 0-4, scale to percent)
pub const DISCRETE_BATTERY_PRODUCT_IDS: &[u16] = &[0x2202, 0x2206, 0x223a, 0x227a, 0x22a4];

/// Product ID reported when the same headset is connected directly via a
/// USB cable instead of the wireless dongle.
pub const WIRED_PRODUCT_ID: u16 = 0x22a0;

/// Status request command byte; the device replies with a status report
/// whose byte 2 is battery percent (0-100) and byte 3 is charge/connection
/// state (`0x00` offline, `0x01`/`0x02` charging, otherwise on battery).
const STATUS_REQUEST: u8 = 0xb0;

/// Upper bound on the status report SteelSeries devices in this family send
/// back; the actual read is truncated to whatever the device returns.
const STATUS_RESPONSE_BUF_LEN: usize = 128;

/// Parses an Arctis Nova 7 (Gen 2 percentage-battery firmware) status report
/// into a [`BatteryStatus`]: byte 2 is the battery percent, byte 3 the
/// charge/connection state (`0x00` means offline, which is surfaced as
/// [`Error::DeviceNotFound`] rather than a zero-battery reading).
fn parse_status_percentage(response: &[u8]) -> Result<BatteryStatus> {
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

/// Parses an Arctis Nova 7 (original discrete-battery firmware) status
/// report into a [`BatteryStatus`]. Identical to [`parse_status_percentage`]
/// except byte 2 is a discrete 0-4 level rather than a direct percentage, so
/// it is scaled to a 0-100 percent (`0` -> 0%, `4` -> 100%).
fn parse_status_discrete(response: &[u8]) -> Result<BatteryStatus> {
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
        level: (response[2].min(4) as u32 * 100 / 4) as u8,
        is_charging: response[3] == CHARGING_A || response[3] == CHARGING_B,
    })
}

/// Parses chatmix (game/chat volume balance) from the same status report
/// `parse_status` reads; unlike `parse_status`, does not gate on the
/// offline sentinel — chatmix stays valid even when the headset reports
/// offline.
fn parse_chatmix(response: &[u8]) -> Result<ChatMixStatus> {
    if response.len() < 6 {
        return Err(Error::InvalidParameter(
            "Invalid chatmix status response".to_string(),
        ));
    }

    Ok(ChatMixStatus {
        game_percent: response[4].min(100),
        chat_percent: response[5].min(100),
    })
}

/// The four built-in equalizer presets, each a 10-band gain array in dB.
const EQ_PRESET_FLAT: [f32; 10] = [0.0; 10];
const EQ_PRESET_BASS: [f32; 10] = [3.5, 5.5, 4.0, 1.0, -1.5, -1.5, -1.0, -1.0, -1.0, -1.0];
const EQ_PRESET_FOCUS: [f32; 10] = [-5.0, -3.5, -1.0, -3.5, -2.5, 4.0, 6.0, -3.5, 0.0, 0.0];
const EQ_PRESET_SMILEY: [f32; 10] = [3.0, 3.5, 1.5, -1.5, -4.0, -4.0, -2.5, 1.5, 3.0, 4.0];
const EQ_PRESETS: &[&[f32]] = &[&EQ_PRESET_FLAT, &EQ_PRESET_BASS, &EQ_PRESET_FOCUS, &EQ_PRESET_SMILEY];

/// SteelSeries Arctis Nova 7 device profile. Battery, chatmix and sidetone
/// only: `dpi`, `polling_rate` and `buttons` all come from `DeviceProfile::NONE`.
pub const ARCTIS_NOVA_7_PROFILE: DeviceProfile = DeviceProfile {
    name: "SteelSeries Arctis Nova 7",
    vendor_id: STEELSERIES_VENDOR_ID,
    product_ids: PERCENTAGE_BATTERY_PRODUCT_IDS,
    usage_page: 0xffc0,
    battery: Some(BatteryProfile {
        command: STATUS_REQUEST,
        extra_byte: None,
        readback_length: STATUS_RESPONSE_BUF_LEN,
        parse: parse_status_percentage,
    }),
    chatmix: Some(ChatMixProfile {
        command: STATUS_REQUEST,
        extra_byte: None,
        readback_length: STATUS_RESPONSE_BUF_LEN,
        parse: parse_chatmix,
    }),
    sidetone: Some(SidetoneProfile {
        command: commands::SIDETONE,
    }),
    mic_mute_led: Some(MicMuteLedProfile {
        command: commands::MIC_MUTE_LED_BRIGHTNESS,
    }),
    mic_volume: Some(MicVolumeProfile {
        command: commands::MIC_VOLUME,
    }),
    equalizer: Some(EqualizerProfile {
        command: commands::EQUALIZER,
        bands: 10,
        presets: EQ_PRESETS,
    }),
    sleep_timer: Some(SleepTimerProfile { command: commands::SLEEP_TIMER }),
    volume_limiter: Some(VolumeLimiterProfile { command: commands::VOLUME_LIMITER }),
    bluetooth: Some(BluetoothProfile {
        power_on_command: commands::BLUETOOTH_POWER_ON,
        call_volume_command: commands::BLUETOOTH_CALL_VOLUME,
        save_command: commands::BLUETOOTH_SAVE,
    }),
    ..DeviceProfile::NONE
};

/// SteelSeries Arctis Nova 7 family using the original discrete (0-4) battery protocol
/// instead of the Gen 2 percentage protocol. Shares every other capability with
/// [`ARCTIS_NOVA_7_PROFILE`] — only the product IDs and battery-parsing function differ.
pub const ARCTIS_NOVA_7_DISCRETE_BATTERY_PROFILE: DeviceProfile = DeviceProfile {
    product_ids: DISCRETE_BATTERY_PRODUCT_IDS,
    battery: Some(BatteryProfile {
        command: STATUS_REQUEST,
        extra_byte: None,
        readback_length: STATUS_RESPONSE_BUF_LEN,
        parse: parse_status_discrete,
    }),
    ..ARCTIS_NOVA_7_PROFILE
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_chatmix_reads_game_and_chat_percent_even_when_offline() {
        let response = [0xb0, 0x00, 75, 0x00, 60, 40];
        let status = parse_chatmix(&response).unwrap();
        assert_eq!(status.game_percent, 60);
        assert_eq!(status.chat_percent, 40);
    }

    #[test]
    fn parse_chatmix_errors_on_short_response() {
        let response = [0xb0, 0x00, 75, 0x00, 60];
        assert!(parse_chatmix(&response).is_err());
    }

    #[test]
    fn parse_chatmix_clamps_percent_to_100() {
        let response = [0xb0, 0x00, 75, 0x00, 200, 150];
        let status = parse_chatmix(&response).unwrap();
        assert_eq!(status.game_percent, 100);
        assert_eq!(status.chat_percent, 100);
    }

    #[test]
    fn parse_status_discrete_scales_zero_of_four_to_zero_percent() {
        let response = [0xb0, 0x00, 0, 0x01];
        let status = parse_status_discrete(&response).unwrap();
        assert_eq!(status.level, 0);
    }

    #[test]
    fn parse_status_discrete_scales_two_of_four_to_fifty_percent() {
        let response = [0xb0, 0x00, 2, 0x01];
        let status = parse_status_discrete(&response).unwrap();
        assert_eq!(status.level, 50);
    }

    #[test]
    fn parse_status_discrete_scales_four_of_four_to_hundred_percent() {
        let response = [0xb0, 0x00, 4, 0x01];
        let status = parse_status_discrete(&response).unwrap();
        assert_eq!(status.level, 100);
    }

    /// Narrower, file-local sanity check on just these two lists.
    #[test]
    fn percentage_and_discrete_battery_product_ids_do_not_overlap() {
        for id in PERCENTAGE_BATTERY_PRODUCT_IDS {
            assert!(
                !DISCRETE_BATTERY_PRODUCT_IDS.contains(id),
                "product_id 0x{:04x} appears in both PERCENTAGE_BATTERY_PRODUCT_IDS and DISCRETE_BATTERY_PRODUCT_IDS",
                id
            );
        }
    }
}
