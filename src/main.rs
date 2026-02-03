use clap::{Parser, Subcommand};
use steelctl::buttons::ButtonMapping;
use steelctl::device::rival3_wireless::{Rival3Wireless, PRODUCT_ID};
use steelctl::device::{
    ConfigurableButtons, ConfigurableDpi, ConfigurablePollingRate, SteelSeriesDevice,
    WirelessDevice,
};
use steelctl::hid::SteelSeriesHid;
use steelctl::protocol::STEELSERIES_VENDOR_ID;

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
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    // Initialize HID API
    let api = hidapi::HidApi::new()?;

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
    }

    Ok(())
}
