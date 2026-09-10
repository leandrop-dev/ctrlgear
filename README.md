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

This builds the whole workspace. The CLI binary will be available at `target/release/steelctl-cli`; the GUI binary (see [GUI](#gui) below) at `target/release/steelctl`.

### Install (binary + udev rule)

```bash
sudo make install
```

Installs the CLI binary to `/usr/local/bin` and the udev rule to `/etc/udev/rules.d/`, then reloads udev — one command instead of the manual steps above. `sudo make uninstall` reverses it. Override `PREFIX` or `UDEV_RULES_DIR` if you need a different layout.

### Arch Linux / CachyOS (PKGBUILD)

```bash
makepkg -si
```

Builds and installs `steelctl-cli` as a proper pacman package (binary in `/usr/bin`, udev rule in `/usr/lib/udev/rules.d/`, license and docs under `/usr/share`), so `pacman -R steelctl-cli` cleanly removes everything, including the udev rule.

## Usage

```bash
# List all connected SteelSeries devices
steelctl-cli list-devices

# Set DPI presets
steelctl-cli dpi "400,800,1600,3200"

# Set polling rate
steelctl-cli polling-rate 1000

# Configure button mapping
steelctl-cli buttons "button3=scrollup,button6=dpi"

# Check battery status (wireless devices)
steelctl-cli battery

# Save settings to device memory
steelctl-cli save

# Show device information
steelctl-cli info
```

### Button Mapping Examples

```bash
# Reset to default mapping
steelctl-cli buttons "default"

# Make side buttons scroll
steelctl-cli buttons "button4=scrollup,button5=scrolldown"

# Swap left and right buttons
steelctl-cli buttons "button1=button2,button2=button1"

# Disable a button
steelctl-cli buttons "button6=disabled"
```

## GUI

An early-stage desktop GUI lives in `gui/`, built with [Slint](https://slint.dev). It shares all device/protocol logic with the CLI via the `steelctl-core` library crate — no duplicated HID handling.

```bash
cargo run --release -p steelctl-gui
```

Currently a minimal scaffold (device info, battery status, DPI apply) proving the workspace wiring; it is not yet packaged by `Makefile` or `PKGBUILD`.

See [BUTTON_MAPPING.md](BUTTON_MAPPING.md) for detailed button mapping documentation.

## Project Structure

```
steelctl/
├── core/                           # steelctl-core: device/protocol library, no UI deps
│   └── src/
│       ├── lib.rs                   # Library root
│       ├── error.rs                 # Error types
│       ├── protocol.rs              # HID protocol constants
│       ├── hid.rs                   # HID communication wrapper
│       ├── buttons.rs               # Button mapping logic
│       └── device/
│           ├── mod.rs                # Device trait definitions
│           └── rival3_wireless.rs    # Rival 3 Wireless implementation
├── cli/                             # steelctl-cli: command-line interface (clap)
│   └── src/main.rs
├── gui/                             # steelctl: desktop GUI (Slint)
│   ├── build.rs
│   ├── ui/main.slint
│   └── src/main.rs
├── udev/
│   └── 70-steelseries.rules       # Vendor-wide HID access rule (must sort before 73-seat-late.rules)
├── Cargo.toml                     # workspace manifest
├── Makefile                       # sudo make install / uninstall
├── PKGBUILD                       # Arch Linux / CachyOS package
├── README.md
└── BUTTON_MAPPING.md              # Detailed button mapping guide
```

## Architecture

The project is a Cargo workspace with three crates:

### 1. `steelctl-core` (`core/`)
All device/protocol logic, with no CLI or UI dependencies — shared by both `steelctl-cli` and `steelctl` (GUI).

- **Protocol layer** (`protocol.rs`): HID report types and command constants; maps high-level concepts (polling rate, battery status) to byte values
- **HID communication layer** (`hid.rs`): wraps the `hidapi` library, handles low-level USB HID communication, implements command delays to prevent device overload
- **Device layer** (`device/`): trait-based abstraction for different device types, device-specific implementations with protocol commands, sensor-specific mappings (e.g., DPI tables)

### 2. `steelctl-cli` (`cli/`)
Command-line interface using `clap`; parses user input and calls into `steelctl-core`.

### 3. `steelctl` GUI (`gui/`)
Desktop interface using [Slint](https://slint.dev); calls the same `steelctl-core` device traits as the CLI.

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
steelctl-cli list-devices
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

1. Create a new module in `core/src/device/`
2. Implement the appropriate traits (`SteelSeriesDevice`, `ConfigurableDpi`, etc.)
3. Add the device's protocol commands and DPI mapping table
4. Update the CLI (and GUI, if applicable) to support device auto-detection

## Troubleshooting

### Permission Denied on Linux

If you get permission errors when accessing the device:
1. Make sure you've added the udev rules (see Installation)
2. Unplug and replug your device
3. Verify the rules are loaded: `udevadm info -a -n /dev/hidraw0 | grep idVendor`

### Device Not Found

1. Check if your device is connected:
   ```bash
   steelctl-cli list-devices
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
