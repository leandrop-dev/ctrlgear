use steelctl_core::device::rival3_wireless::{Rival3Wireless, PRODUCT_ID};
use steelctl_core::device::{ConfigurableDpi, SteelSeriesDevice, WirelessDevice};
use steelctl_core::hid::SteelSeriesHid;
use steelctl_core::protocol::STEELSERIES_VENDOR_ID;
use steelctl_core::Error;

slint::include_modules!();

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let window = MainWindow::new()?;

    let weak = window.as_weak();
    window.on_refresh(move || {
        let window = weak.unwrap();
        match refresh_info() {
            Ok((name, battery)) => {
                window.set_device_name(name.into());
                window.set_battery_status(battery.into());
            }
            Err(e) => {
                window.set_device_name("No device connected".into());
                window.set_battery_status(e.to_string().into());
            }
        }
    });

    let weak = window.as_weak();
    window.on_apply_dpi(move |values| {
        let window = weak.unwrap();
        window.set_battery_status(match apply_dpi(&values) {
            Ok(()) => format!("DPI set to {}", values).into(),
            Err(e) => e.to_string().into(),
        });
    });

    window.run()?;
    Ok(())
}

fn open_device() -> steelctl_core::Result<Rival3Wireless> {
    let api = hidapi::HidApi::new()?;
    let hid = SteelSeriesHid::open(&api, STEELSERIES_VENDOR_ID, PRODUCT_ID)?;
    Ok(Rival3Wireless::new(hid))
}

fn refresh_info() -> steelctl_core::Result<(String, String)> {
    let device = open_device()?;
    let battery = device.battery_status()?;
    let status = if battery.is_charging {
        format!("Battery: {}% (charging)", battery.level)
    } else {
        format!("Battery: {}%", battery.level)
    };
    Ok((device.name().to_string(), status))
}

fn apply_dpi(values: &str) -> steelctl_core::Result<()> {
    let dpi_values: Vec<u16> = values
        .split(',')
        .map(|s| {
            s.trim()
                .parse::<u16>()
                .map_err(|_| Error::InvalidParameter(format!("invalid DPI value: {}", s.trim())))
        })
        .collect::<steelctl_core::Result<_>>()?;

    let device = open_device()?;
    device.set_dpi(&dpi_values)
}
