use clap::{Parser, Subcommand};
use ctrlgear_core::buttons::ButtonMapping;
use ctrlgear_core::protocol::STEELSERIES_VENDOR_ID;
use ctrlgear_core::registry;

#[derive(Parser)]
#[command(name = "ctrlgear-cli")]
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
    /// Set sidetone (mic monitoring) level
    Sidetone {
        /// Sidetone level: 0 (Off), 1 (Low), 2 (Medium), or 3 (High)
        level: u8,
    },
    /// Set microphone-mute LED brightness
    MicMuteLed {
        /// Brightness level: 0 (Off) to 3 (max)
        level: u8,
    },
    /// Set microphone input volume
    MicVolume {
        /// Volume level: 0 (Off) to 7 (max)
        level: u8,
    },
    /// Set 10-band equalizer gains
    Equalizer {
        /// Gain values in dB, separated by commas (e.g., "0,0,0,0,0,0,0,0,0,0")
        bands: String,
    },
    /// Apply a built-in equalizer preset
    EqualizerPreset {
        /// Preset index: 0 (Flat), 1 (Bass), 2 (Focus), or 3 (Smiley)
        preset: u8,
    },
    /// Set auto-sleep (inactive time) timer
    SleepTimer {
        /// Minutes of inactivity before the device sleeps: 0-255
        minutes: u8,
    },
    /// Toggle the output volume limiter
    VolumeLimiter {
        /// true or false
        #[arg(action = clap::ArgAction::Set)]
        enabled: bool,
    },
    /// Toggle whether Bluetooth turns on when the headset powers on
    BluetoothPowerOn {
        /// true or false
        #[arg(action = clap::ArgAction::Set)]
        enabled: bool,
    },
    /// Set Bluetooth call-volume mode
    BluetoothCallVolume {
        /// Mode: 0 (do nothing), 1 (lower game volume by 12dB), or 2 (mute game during calls)
        mode: u8,
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

    // Open the first connected device whose profile supports the requested capability, rather
    // than assuming a single hardcoded product — SteelSeries devices span multiple product
    // families with disjoint capability sets (e.g. DPI is mouse-only, sidetone is headset-only).
    let device = match &cli.command {
        Commands::Dpi { .. } => open_matching(&api, |d| d.supports_dpi())?,
        Commands::PollingRate { .. } => open_matching(&api, |d| d.supports_polling_rate())?,
        Commands::Buttons { .. } => open_matching(&api, |d| d.supports_button_mapping())?,
        Commands::Sidetone { .. } => open_matching(&api, |d| d.supports_sidetone())?,
        Commands::MicMuteLed { .. } => open_matching(&api, |d| d.supports_mic_mute_led())?,
        Commands::MicVolume { .. } => open_matching(&api, |d| d.supports_mic_volume())?,
        Commands::Equalizer { .. } => open_matching(&api, |d| d.supports_equalizer())?,
        Commands::EqualizerPreset { .. } => open_matching(&api, |d| d.supports_equalizer())?,
        Commands::SleepTimer { .. } => open_matching(&api, |d| d.supports_sleep_timer())?,
        Commands::VolumeLimiter { .. } => open_matching(&api, |d| d.supports_volume_limiter())?,
        Commands::BluetoothPowerOn { .. } => open_matching(&api, |d| d.supports_bluetooth_settings())?,
        Commands::BluetoothCallVolume { .. } => open_matching(&api, |d| d.supports_bluetooth_settings())?,
        Commands::Battery => open_matching(&api, |d| d.supports_battery())?,
        Commands::Save | Commands::Info => open_matching(&api, |_| true)?,
        Commands::ListDevices => unreachable!(), // Handled before device opening
    };

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
        Commands::Sidetone { level } => {
            device.set_sidetone(level)?;
            println!("Set sidetone level to: {}", level);
        }
        Commands::MicMuteLed { level } => {
            device.set_mic_mute_led_brightness(level)?;
            println!("Set mic mute LED brightness to: {}", level);
        }
        Commands::MicVolume { level } => {
            device.set_mic_volume(level)?;
            println!("Set mic volume to: {}", level);
        }
        Commands::Equalizer { bands } => {
            // Parse comma-separated gain values
            let gains: Result<Vec<f32>, _> = bands
                .split(',')
                .map(|s| s.trim().parse::<f32>())
                .collect();

            let gains = gains?;
            device.set_equalizer(&gains)?;
            println!("Set equalizer bands to: {:?}", gains);
        }
        Commands::EqualizerPreset { preset } => {
            device.set_equalizer_preset(preset)?;
            println!("Applied equalizer preset: {}", preset);
        }
        Commands::SleepTimer { minutes } => {
            device.set_sleep_timer(minutes)?;
            println!("Set sleep timer to: {} minutes", minutes);
        }
        Commands::VolumeLimiter { enabled } => {
            device.set_volume_limiter(enabled)?;
            println!("Set volume limiter to: {}", enabled);
        }
        Commands::BluetoothPowerOn { enabled } => {
            device.set_bluetooth_when_powered_on(enabled)?;
            println!("Set Bluetooth-when-powered-on to: {}", enabled);
        }
        Commands::BluetoothCallVolume { mode } => {
            device.set_bluetooth_call_volume(mode)?;
            println!("Set Bluetooth call volume mode to: {}", mode);
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
            if let Ok(version) = device.firmware_version() {
                println!("Firmware Version: {}", version);
            }
        }
        Commands::ListDevices => unreachable!(), // Handled before device opening
    }

    Ok(())
}

/// Opens the first connected device (across every product ID declared by any
/// `registry::PROFILES` entry) whose capabilities satisfy `predicate`. SteelSeries devices span
/// multiple product families with disjoint capability sets (e.g. DPI is mouse-only, sidetone is
/// headset-only), so a single hardcoded product ID can't serve every subcommand.
fn open_matching(
    api: &hidapi::HidApi,
    predicate: impl Fn(&registry::Device) -> bool,
) -> anyhow::Result<registry::Device> {
    for profile in registry::PROFILES {
        for &product_id in profile.product_ids {
            if let Ok(device) = registry::open(api, product_id) {
                if predicate(&device) {
                    return Ok(device);
                }
            }
        }
    }
    anyhow::bail!("No connected device supports this command")
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
                println!("                 ⮕ Configuration interface (used by ctrlgear-cli)");
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
