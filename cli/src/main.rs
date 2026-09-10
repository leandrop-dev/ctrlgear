use clap::{Parser, Subcommand};
use steelctl_core::buttons::ButtonMapping;
use steelctl_core::device::rival3_wireless::{Rival3Wireless, PRODUCT_ID};
use steelctl_core::device::{
    ConfigurableButtons, ConfigurableDpi, ConfigurablePollingRate, SteelSeriesDevice,
    WirelessDevice,
};
use steelctl_core::hid::SteelSeriesHid;
use steelctl_core::protocol::STEELSERIES_VENDOR_ID;

#[derive(Parser)]
#[command(name = "steelctl")]
#[command(author, version, about = "Manage SteelSeries hardware", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Set DPI values (up to 5 presets)
    Dpi {
        /// DPI values separated by commas (e.g., "400,800,1600")
        values: String,
    },
    /// Set polling rate in Hz
    PollingRate {
        /// Polling rate: 125, 250, 500, or 1000
        hz: u16,
    },
    /// Set button mapping
    Buttons {
        /// Button mapping (e.g., "button3=scrollup,button6=dpi")
        /// Use "default" to reset to default mapping
        /// Available actions: button1-6, dpi, scrollup, scrolldown, disabled
        mapping: String,
    },
    /// Get battery status (wireless devices only)
    Battery,
    /// Save current settings to device memory
    Save,
    /// Show device information
    Info,
    /// List all SteelSeries HID devices and interfaces
    ListDevices,
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    // Initialize HID API
    let api = hidapi::HidApi::new()?;

    // Handle commands that don't require opening a device
    if matches!(cli.command, Commands::ListDevices) {
        return list_devices(&api);
    }

    // Open Rival 3 Wireless device
    let hid = SteelSeriesHid::open(&api, STEELSERIES_VENDOR_ID, PRODUCT_ID)?;
    let device = Rival3Wireless::new(hid);

    match cli.command {
        Commands::Dpi { values } => {
            // Parse comma-separated DPI values
            let dpi_values: Result<Vec<u16>, _> = values
                .split(',')
                .map(|s| s.trim().parse::<u16>())
                .collect();

            let dpi_values = dpi_values?;
            device.set_dpi(&dpi_values)?;
            println!("Set DPI to: {:?}", dpi_values);
        }
        Commands::PollingRate { hz } => {
            device.set_polling_rate(hz)?;
            println!("Set polling rate to: {} Hz", hz);
        }
        Commands::Buttons { mapping } => {
            let button_mapping = ButtonMapping::from_string(&mapping)?;
            device.set_button_mapping(&button_mapping)?;
            println!("Button mapping applied:");
            println!("  {}", button_mapping.describe());
        }
        Commands::Battery => {
            let status = device.battery_status()?;
            println!("Battery Level: {}%", status.level);
            println!("Charging: {}", if status.is_charging { "Yes" } else { "No" });
        }
        Commands::Save => {
            device.save()?;
            println!("Settings saved to device memory");
        }
        Commands::Info => {
            println!("Device: {}", device.name());
            println!("Vendor ID: 0x{:04x}", device.vendor_id());
            println!("Product ID: 0x{:04x}", device.product_id());
        }
        Commands::ListDevices => unreachable!(), // Handled before device opening
    }

    Ok(())
}

fn list_devices(api: &hidapi::HidApi) -> anyhow::Result<()> {
    println!("SteelSeries HID Devices:\n");

    let mut found_any = false;

    for device in api.device_list() {
        if device.vendor_id() == STEELSERIES_VENDOR_ID {
            found_any = true;
            println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
            println!("Product ID:      0x{:04x}", device.product_id());
            println!(
                "Product:         {}",
                device.product_string().unwrap_or("N/A")
            );
            println!(
                "Manufacturer:    {}",
                device.manufacturer_string().unwrap_or("N/A")
            );
            println!("Path:            {}", device.path().to_string_lossy());
            println!("Interface:       {}", device.interface_number());
            println!("Usage Page:      0x{:04x}", device.usage_page());
            println!("Usage:           0x{:04x}", device.usage());

            // Highlight the configuration interface
            if device.usage_page() == 0xffc0 {
                println!("                 ⮕ Configuration interface (used by steelctl)");
            }

            println!();
        }
    }

    if !found_any {
        println!("No SteelSeries devices found.");
        println!("\nTroubleshooting:");
        println!("  1. Make sure your device is connected");
        println!("  2. For wireless mice, ensure they're in 2.4 GHz mode (not Bluetooth)");
        println!("  3. Check USB cable/receiver connection");
    }

    Ok(())
}
