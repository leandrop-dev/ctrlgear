![Control Gear](Control-Gear-banner.jpg "Control Gear")

> A Rust utility for managing SteelSeries/Razer/Logitech/HyperX/Keychron gaming peripherals — CLI and desktop GUI. Unofficial; not affiliated with or endorsed by SteelSeries/Razer/Logitech/HyperX/Keychron.

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

This builds the whole workspace. The CLI binary will be available at `target/release/ctrlgear-cli`; the GUI binary (see [GUI](#gui) below) at `target/release/ctrlgear`.

### Install (CLI + GUI + udev rule)

```bash
sudo make install
```

Installs both the `ctrlgear-cli` binary and the `ctrlgear` GUI (with its desktop entry and icon) to `/usr/local/bin`/`/usr/local/share`, plus the udev rule to `/etc/udev/rules.d/`, then reloads udev — one command instead of the manual steps above. `sudo make uninstall` reverses it. Want just one? `sudo make install-cli` or `sudo make install-gui` (with matching `uninstall-cli`/`uninstall-gui`). Override `PREFIX`, `DATADIR`, or `UDEV_RULES_DIR` if you need a different layout.

### Arch Linux / CachyOS (PKGBUILD)

```bash
makepkg -si
```

Builds and installs `ctrlgear-cli` as a proper pacman package (binary in `/usr/bin`, udev rule in `/usr/lib/udev/rules.d/`, license and docs under `/usr/share`), so `pacman -R ctrlgear-cli` cleanly removes everything, including the udev rule.

## Usage

```bash
# List all connected SteelSeries devices
ctrlgear-cli list-devices

# Set DPI presets
ctrlgear-cli dpi "400,800,1600,3200"

# Set polling rate
ctrlgear-cli polling-rate 1000

# Configure button mapping
ctrlgear-cli buttons "button3=scrollup,button6=dpi"

# Check battery status (wireless devices)
ctrlgear-cli battery

# Save settings to device memory
ctrlgear-cli save

# Show device information
ctrlgear-cli info
```

### Button Mapping Examples

```bash
# Reset to default mapping
ctrlgear-cli buttons "default"

# Make side buttons scroll
ctrlgear-cli buttons "button4=scrollup,button5=scrolldown"

# Swap left and right buttons
ctrlgear-cli buttons "button1=button2,button2=button1"

# Disable a button
ctrlgear-cli buttons "button6=disabled"
```

## GUI

A desktop GUI lives in `gui/`, built with [Slint](https://slint.dev). It shares all device/protocol logic with the CLI via the `ctrlgear-core` library crate — no duplicated HID handling.

```bash
cargo run --release -p ctrlgear-gui
```

Sidebar + main content layout: a device list (name, battery, supported/unsupported) with a detail page per device for settings (currently DPI, for devices that support it). `sudo make install` (or `install-gui` alone) installs it with its desktop entry and icon — required on Wayland, since an app can't set its own taskbar/title-bar icon at runtime there; the desktop environment resolves it from the installed `.desktop` file instead. Not yet packaged by the PKGBUILD (CLI-only).

See [BUTTON_MAPPING.md](BUTTON_MAPPING.md) for detailed button mapping documentation.

## Project Structure

```
ctrlgear/
├── core/                           # ctrlgear-core: device/protocol library, no UI deps
│   └── src/
│       ├── lib.rs                   # Library root
│       ├── error.rs                 # Error types
│       ├── protocol.rs              # HID protocol constants
│       ├── hid.rs                   # HID communication wrapper
│       ├── buttons.rs               # Button mapping logic
│       ├── registry.rs              # Device discovery: product ID → concrete device
│       └── device/
│           ├── mod.rs                # Device trait definitions
│           ├── rival3_wireless.rs    # Rival 3 Wireless implementation
│           └── arctis_nova_7.rs      # Arctis Nova 7 Wireless implementation (battery only)
├── cli/                             # ctrlgear-cli: command-line interface (clap)
│   └── src/main.rs
├── gui/                             # ctrlgear: desktop GUI (Slint)
│   ├── build.rs
│   ├── ui/main.slint
│   ├── src/main.rs
│   ├── assets/app-icon.svg          # original app icon (not SteelSeries branding)
│   └── packaging/ctrlgear.desktop   # desktop entry (needed for the icon on Wayland)
├── udev/
│   └── 70-steelseries.rules       # Vendor-wide HID access rule (must sort before 73-seat-late.rules)
├── Cargo.toml                     # workspace manifest
├── Makefile                       # sudo make install / uninstall [-gui]
├── PKGBUILD                       # Arch Linux / CachyOS package (CLI only)
├── README.md
└── BUTTON_MAPPING.md              # Detailed button mapping guide
```

## Architecture

The project is a Cargo workspace with three crates:

### 1. `ctrlgear-core` (`core/`)
All device/protocol logic, with no CLI or UI dependencies — shared by both `ctrlgear-cli` and `ctrlgear` (GUI).

- **Protocol layer** (`protocol.rs`): HID report types and command constants; maps high-level concepts (polling rate, battery status) to byte values
- **HID communication layer** (`hid.rs`): wraps the `hidapi` library, handles low-level USB HID communication, implements command delays to prevent device overload
- **Device layer** (`device/`): trait-based abstraction for different device types, device-specific implementations with protocol commands, sensor-specific mappings (e.g., DPI tables)
- **Registry** (`registry.rs`): maps a connected device's product ID to its concrete implementation; `discover()` enumerates connected SteelSeries devices for UI consumption (used by the GUI's device list)

### 2. `ctrlgear-cli` (`cli/`)
Command-line interface using `clap`; parses user input and calls into `ctrlgear-core`.

### 3. `ctrlgear` GUI (`gui/`)
Desktop interface using [Slint](https://slint.dev); calls the same `ctrlgear-core` device traits as the CLI.

## Supported Devices

### Mice

| Device | Product ID | DPI | Polling Rate | Buttons | RGB | Battery |
|--------|------------|-----|--------------|---------|-----|---------|
| **Rival 3 Wireless** | 0x1830 | ✅ 100-18000 | ✅ | ✅ 6 buttons | ❌ | ✅ |
| **Aerox 3** | 0x1836 | ✅ 200-8500 | ✅ | ✅* | ✅ | ❌ (wired only) |
| **Aerox 5** | 0x1850 | ✅ 100-18000 | ✅ | ✅* | ✅ | ❌ (wired only) |
| **Rival 3 Gen 2** | 0x1870 | ⚠️ | ✅ | ✅* | ✅ | ❌ (wired only) |
| **Prime Mini** | 0x184D | ✅ 50-18000 | ✅ | ✅* | ✅ (single zone) | ❌ (wired only) |

\* Button mapping on the Aerox 3/5, Rival 3 Gen 2, and Prime Mini reuses the Rival 3 Wireless's 6-slot encoding, which does not yet reflect these devices' real per-model button layout — see the `BUTTONS` doc comment in `core/src/device/aerox_gen2_rgb.rs`.

Rival 3 Gen 2's DPI is marked ⚠️ because its command uses a dual-axis (X/Y) value shape the current `DpiProfile` model doesn't support yet — see `core/src/device/aerox_gen2_rgb.rs::RIVAL3_GEN2_PROFILE`.

### Headsets

| Device | Product ID | Battery | ChatMix | Sidetone | Mic Mute LED | Mic Volume | Equalizer | Sleep Timer | Volume Limiter | Bluetooth |
|--------|------------|---------|---------|----------|--------------|------------|-----------|--------------|-----------------|-----------|
| **Arctis Nova 7** | 0x22a1 (+ 11 more) | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ 10-band, 4 presets | ✅ | ✅ | ✅ |

The Arctis Nova 7 family covers 12 known product IDs across two battery-report protocols (percentage vs. discrete 0-4) — see `PERCENTAGE_BATTERY_PRODUCT_IDS` / `DISCRETE_BATTERY_PRODUCT_IDS` in `core/src/device/arctis_nova_7.rs` for the full list. It has no DPI, polling rate, or button-mapping concepts.

**Legend:**
- ✅ Supported
- ❌ Not available on hardware
- ⚠️ Hardware supports, not yet implemented

To check which SteelSeries devices are connected to your system:
```bash
ctrlgear-cli list-devices
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

Contributions are welcome! To add support for an additional SteelSeries device:

1. Declare a `const DeviceProfile` for it in a new (or existing family) module under `core/src/device/` — devices are data, not per-device trait implementations; `ProfiledDevice` (`core/src/device/profile.rs`) implements every capability generically from whichever profile fields are set
2. Add the device's protocol commands and DPI/RGB/equalizer tables as `const`s alongside its profile
3. Register the profile in `core/src/registry.rs::PROFILES` — the CLI and GUI both pick it up automatically, no further wiring needed

See the `add-steelseries-device` skill (`.mallet/skills/add-steelseries-device/`) for the full walkthrough.

## Troubleshooting

### Permission Denied on Linux

If you get permission errors when accessing the device:
1. Make sure you've added the udev rules (see Installation)
2. Unplug and replug your device
3. Verify the rules are loaded: `udevadm info -a -n /dev/hidraw0 | grep idVendor`

### Device Not Found

1. Check if your device is connected:
   ```bash
   ctrlgear-cli list-devices
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
