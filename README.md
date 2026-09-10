# SteelCtl

A Rust utility for managing SteelSeries gaming peripherals.

## Features

- **DPI Configuration**: Set multiple DPI presets with device-specific ranges
- **Polling Rate**: Configure USB polling rate (125/250/500/1000 Hz)
- **Button Mapping**: Remap buttons with support for mouse buttons, DPI switch, scroll emulation, and disable
- **Battery Status**: Check battery level and charging status (wireless devices)
- **Settings Persistence**: Save settings to device memory
- **Device Discovery**: List all connected SteelSeries devices and their HID interfaces

## Installation

### Prerequisites

On Linux, you need a udev rule granting access to SteelSeries' HID devices. This is a one-time setup per machine — the rule matches on vendor ID (`1038`, shared across the whole SteelSeries product line), so it does not need to be redone when you plug in a different or newer SteelSeries device.

`udev/70-steelseries.rules` in this repo is the canonical copy of the rule (the `70-` prefix matters — see the comment in the file):

```
SUBSYSTEM=="hidraw", ATTRS{idVendor}=="1038", TAG+="uaccess"
SUBSYSTEM=="usb", ATTRS{idVendor}=="1038", TAG+="uaccess"
```

`TAG+="uaccess"` hands access to whoever is logged in at the physical seat (via `systemd-logind`), rather than making the device world-read/writable with `MODE="0666"` — no group membership or extra setup required.

You can install it manually, or let `make install` / the AUR package below do it for you:

```bash
sudo cp udev/70-steelseries.rules /etc/udev/rules.d/
sudo udevadm control --reload-rules
sudo udevadm trigger
```

### Build from Source

```bash
cargo build --release
```

The binary will be available at `target/release/steelctl`.

### Install (binary + udev rule)

```bash
sudo make install
```

Installs the binary to `/usr/local/bin` and the udev rule to `/etc/udev/rules.d/`, then reloads udev — one command instead of the manual steps above. `sudo make uninstall` reverses it. Override `PREFIX` or `UDEV_RULES_DIR` if you need a different layout.

### Arch Linux / CachyOS (PKGBUILD)

```bash
makepkg -si
```

Builds and installs `steelctl` as a proper pacman package (binary in `/usr/bin`, udev rule in `/usr/lib/udev/rules.d/`, license and docs under `/usr/share`), so `pacman -R steelctl` cleanly removes everything, including the udev rule.

## Usage

```bash
# List all connected SteelSeries devices
steelctl list-devices

# Set DPI presets
steelctl dpi "400,800,1600,3200"

# Set polling rate
steelctl polling-rate 1000

# Configure button mapping
steelctl buttons "button3=scrollup,button6=dpi"

# Check battery status (wireless devices)
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
├── udev/
│   └── 70-steelseries.rules       # Vendor-wide HID access rule (must sort before 73-seat-late.rules)
├── Cargo.toml
├── Makefile                       # sudo make install / uninstall
├── PKGBUILD                       # Arch Linux / CachyOS package
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
- Device-specific implementations with protocol commands
- Handles sensor-specific mappings (e.g., DPI tables)

### 4. CLI Layer (`main.rs`)
- Command-line interface using `clap`
- Parses user input and calls device methods

## Supported Devices

| Device | Product ID | DPI | Polling Rate | Buttons | Battery | RGB |
|--------|------------|-----|--------------|---------|---------|-----|
| **Rival 3 Wireless** | 0x1830 | ✅ 100-18000 | ✅ | ✅ 6 buttons | ✅ | ❌ |

**Legend:**
- ✅ Supported
- ❌ Not available on hardware
- ⚠️ Hardware supports, not yet implemented

To check which SteelSeries devices are connected to your system:
```bash
steelctl list-devices
```

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

WTFPL - Do What The Fuck You Want To Public License

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

1. Check if your device is connected:
   ```bash
   steelctl list-devices
   ```

2. For wireless mice, ensure they're in 2.4 GHz mode (not Bluetooth)

3. Verify the device is detected by the system:
   ```bash
   lsusb | grep 1038
   ```

4. Check which HID interfaces are available:
   ```bash
   ls -l /dev/hidraw*
   ```
