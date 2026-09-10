PREFIX ?= /usr/local
BINDIR ?= $(PREFIX)/bin
DATADIR ?= $(PREFIX)/share
UDEV_RULES_DIR ?= /etc/udev/rules.d
DESTDIR ?=

.PHONY: build build-cli build-gui install install-cli install-gui uninstall uninstall-cli uninstall-gui clean

build: build-cli build-gui

build-cli:
	cargo build --release -p ctrlgear-cli

# Slint's dependency tree is much heavier than the CLI's, hence the separate
# build-cli/build-gui targets — but `make build`/`install`/`uninstall` cover
# both by default so a plain `sudo make install` gives you either interface.
# Use `install-cli`/`install-gui` directly if you only want one.
build-gui:
	cargo build --release -p ctrlgear-gui

# Installs the CLI binary and udev rule (vendor-wide, covers every SteelSeries
# device). Run as `sudo make install-cli` on the target machine. When
# packaging (DESTDIR set), the udev reload step is skipped — that belongs to
# the packaged install, not the build root.
install-cli: build-cli
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
install-gui: build-gui
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
