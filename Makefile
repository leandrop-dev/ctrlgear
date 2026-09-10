PREFIX ?= /usr/local
BINDIR ?= $(PREFIX)/bin
UDEV_RULES_DIR ?= /etc/udev/rules.d
DESTDIR ?=

.PHONY: build install uninstall clean

build:
	cargo build --release

# Installs the binary and udev rule (vendor-wide, covers every SteelSeries
# device). Run as `sudo make install` on the target machine. When packaging
# (DESTDIR set), the udev reload step is skipped — that belongs to the
# packaged install, not the build root.
install: build
	install -Dm755 target/release/steelctl $(DESTDIR)$(BINDIR)/steelctl
	install -Dm644 udev/70-steelseries.rules $(DESTDIR)$(UDEV_RULES_DIR)/70-steelseries.rules
ifeq ($(DESTDIR),)
	udevadm control --reload-rules
	udevadm trigger
endif

uninstall:
	rm -f $(DESTDIR)$(BINDIR)/steelctl
	rm -f $(DESTDIR)$(UDEV_RULES_DIR)/70-steelseries.rules
ifeq ($(DESTDIR),)
	udevadm control --reload-rules
	udevadm trigger
endif

clean:
	cargo clean
