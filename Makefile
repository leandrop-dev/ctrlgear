PREFIX ?= /usr/local
BINDIR ?= $(PREFIX)/bin
DATADIR ?= $(PREFIX)/share
UDEV_RULES_DIR ?= /etc/udev/rules.d
DESTDIR ?=

.PHONY: build build-cli build-gui install install-cli install-gui uninstall uninstall-cli uninstall-gui clean

build: build-cli build-gui

build-cli:
	cargo build --release -p ctrlgear-cli

# Builds the frontend first, then the Tauri binary with the custom-protocol
# feature (required for a release build launched directly, not via `tauri
# dev`).
build-gui:
	cd gui && npm ci && npm run build
	cargo build --release -p ctrlgear-gui --features ctrlgear-gui/custom-protocol

# Install targets only copy prebuilt artifacts: run `make build` as your user
# first, then `sudo make install`. Building under sudo would leave root-owned
# files in target/, gui/node_modules and gui/dist.
CORE_SRCS := $(shell find core/src -type f) core/Cargo.toml Cargo.lock
CLI_SRCS := $(CORE_SRCS) $(shell find cli/src -type f) cli/Cargo.toml
GUI_SRCS := $(CORE_SRCS) $(shell find gui/src gui/src-tauri/src gui/src-tauri/capabilities -type f) \
	gui/src-tauri/Cargo.toml gui/src-tauri/tauri.conf.json gui/index.html gui/package-lock.json

target/release/ctrlgear-cli: $(CLI_SRCS)
	@echo "$@ is missing or older than its sources; run 'make build-cli' as your user first." >&2; exit 1

target/release/ctrlgear: $(GUI_SRCS)
	@echo "$@ is missing or older than its sources; run 'make build-gui' as your user first." >&2; exit 1

# Installs the CLI binary and udev rule (vendor-wide, covers every SteelSeries
# device). When packaging (DESTDIR set), the udev reload step is skipped — that
# belongs to the packaged install, not the build root.
install-cli: target/release/ctrlgear-cli
	install -Dm755 target/release/ctrlgear-cli $(DESTDIR)$(BINDIR)/ctrlgear-cli
	install -Dm644 udev/70-steelseries.rules $(DESTDIR)$(UDEV_RULES_DIR)/70-steelseries.rules
ifeq ($(DESTDIR),)
	udevadm control --reload-rules
	udevadm trigger
endif

uninstall-cli:
	rm -f $(DESTDIR)$(BINDIR)/ctrlgear-cli
	rm -f $(DESTDIR)$(UDEV_RULES_DIR)/70-steelseries.rules
ifeq ($(DESTDIR),)
	udevadm control --reload-rules
	udevadm trigger
endif

# Installs the GUI binary, desktop entry, icon, and udev rule — the desktop
# entry and icon are required on Wayland, where an app can't set its window
# icon at runtime; the desktop environment resolves it from these instead.
install-gui: target/release/ctrlgear
	install -Dm755 target/release/ctrlgear $(DESTDIR)$(BINDIR)/ctrlgear
	install -Dm644 gui/packaging/ctrlgear.desktop $(DESTDIR)$(DATADIR)/applications/ctrlgear.desktop
	install -Dm644 gui/assets/app-icon.svg $(DESTDIR)$(DATADIR)/icons/hicolor/scalable/apps/ctrlgear.svg
	install -Dm644 udev/70-steelseries.rules $(DESTDIR)$(UDEV_RULES_DIR)/70-steelseries.rules
ifeq ($(DESTDIR),)
	udevadm control --reload-rules
	udevadm trigger
endif

uninstall-gui:
	rm -f $(DESTDIR)$(BINDIR)/ctrlgear
	rm -f $(DESTDIR)$(DATADIR)/applications/ctrlgear.desktop
	rm -f $(DESTDIR)$(DATADIR)/icons/hicolor/scalable/apps/ctrlgear.svg

install: install-cli install-gui

uninstall: uninstall-cli uninstall-gui

clean:
	cargo clean
