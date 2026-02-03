# SteelCtl

A Rust utility for managing SteelSeries hardware, with initial support for the Rival 3 Wireless mouse.

## Features

- **DPI Configuration**: Set up to 5 DPI presets (100-18000 in steps of 100)
- **Polling Rate**: Configure USB polling rate (125/250/500/1000 Hz)
- **Button Mapping**: Remap all 6 buttons (mouse buttons, DPI switch, scroll, disable)
- **Battery Status**: Check battery level and charging status
- **Settings Persistence**: Save settings to device memory

## Installation

### Prerequisites

On Linux, you may need to set up udev rules for USB device access:

```bash
# Create a udev rule for SteelSeries devices
sudo tee /etc/udev/rules.d/99-steelseries.rules << EOF
SUBSYSTEM=="hidraw", ATTRS{idVendor}=="1038", MODE="0666"
SUBSYSTEM=="usb", ATTRS{idVendor}=="1038", MODE="0666"
EOF

# Reload udev rules
sudo udevadm control --reload-rules
sudo udevadm trigger
```

### Build from Source

```bash
cargo build --release
```

The binary will be available at `target/release/steelctl`.

## Usage

```bash
# Set DPI presets
steelctl dpi "400,800,1600,3200"

# Set polling rate
steelctl polling-rate 1000

# Configure button mapping
steelctl buttons "button3=scrollup,button6=dpi"

# Check battery status
steelctl battery

# Save settings to device memory
steelctl save

# Show device information
steelctl info
```

### Button Mapping Examples

```bash
# Reset to default mapping
steelctl buttons "default"

# Make side buttons scroll
steelctl buttons "button4=scrollup,button5=scrolldown"

# Swap left and right buttons
steelctl buttons "button1=button2,button2=button1"

# Disable a button
steelctl buttons "button6=disabled"
```

See [BUTTON_MAPPING.md](BUTTON_MAPPING.md) for detailed button mapping documentation.

## Project Structure

```
steelctl/
├── src/
│   ├── main.rs                    # CLI entry point
│   ├── lib.rs                     # Library root
│   ├── error.rs                   # Error types
│   ├── protocol.rs                # HID protocol constants
│   ├── hid.rs                     # HID communication wrapper
│   ├── buttons.rs                 # Button mapping logic
│   └── device/
│       ├── mod.rs                 # Device trait definitions
│       └── rival3_wireless.rs     # Rival 3 Wireless implementation
├── Cargo.toml
├── README.md
└── BUTTON_MAPPING.md              # Detailed button mapping guide
```

## Architecture

The project is organized into several layers:

### 1. Protocol Layer (`protocol.rs`)
- Defines HID report types and command constants
- Contains device-specific protocol definitions
- Maps high-level concepts (polling rate, battery status) to byte values

### 2. HID Communication Layer (`hid.rs`)
- Wraps the `hidapi` library
- Handles low-level USB HID communication
- Implements command delays to prevent device overload

### 3. Device Layer (`device/`)
- Trait-based abstraction for different device types
- Device-specific implementations (e.g., Rival 3 Wireless)
- Handles sensor-specific mappings (e.g., DPI tables)

### 4. CLI Layer (`main.rs`)
- Command-line interface using `clap`
- Parses user input and calls device methods

## Supported Devices

Currently supported:
- **SteelSeries Rival 3 Wireless** (Product ID: 0x1830)

## Development

### Running Tests

```bash
cargo test
```

### Building Documentation

```bash
cargo doc --open
```

## License

MIT OR Apache-2.0

## Contributing

Contributions are welcome! To add support for additional SteelSeries devices:

1. Create a new module in `src/device/`
2. Implement the appropriate traits (`SteelSeriesDevice`, `ConfigurableDpi`, etc.)
3. Add the device's protocol commands and DPI mapping table
4. Update the CLI to support device auto-detection

## Troubleshooting

### Permission Denied on Linux

If you get permission errors when accessing the device:
1. Make sure you've added the udev rules (see Installation)
2. Unplug and replug your device
3. Verify the rules are loaded: `udevadm info -a -n /dev/hidraw0 | grep idVendor`

### Device Not Found

Verify your device is connected and in 2.4 GHz wireless mode (not Bluetooth):
```bash
lsusb | grep 1038
```

You should see a line like:
```
Bus 001 Device 005: ID 1038:1830 SteelSeries ApS SteelSeries Rival 3 Wireless
```
