# Maintainer: Leandro Paixão <inoobik@gmail.com>
pkgname=ctrlgear
pkgver=0.1.0
pkgrel=1
pkgdesc="A Rust CLI and desktop GUI for managing SteelSeries hardware"
arch=('x86_64' 'aarch64')
url="https://github.com/leandrop-dev/ctrlgear"
license=('WTFPL')
depends=('hidapi' 'libusb')
makedepends=('cargo')

# Builds directly from this checkout (run `makepkg` from the repo root) —
# no tagged release exists yet to fetch as a source tarball. Builds both the
# CLI and the Slint GUI workspace members; see gui/packaging/ for the GUI's
# desktop entry and icon.
build() {
  cd "$startdir"
  cargo build --release --locked -p ctrlgear-cli -p ctrlgear-gui
}

check() {
  cd "$startdir"
  cargo test --release --locked -p ctrlgear-core -p ctrlgear-cli
}

package() {
  cd "$startdir"
  install -Dm755 target/release/ctrlgear-cli "$pkgdir/usr/bin/ctrlgear-cli"
  install -Dm755 target/release/ctrlgear "$pkgdir/usr/bin/ctrlgear"
  install -Dm644 gui/packaging/ctrlgear.desktop "$pkgdir/usr/share/applications/ctrlgear.desktop"
  install -Dm644 gui/assets/app-icon.svg "$pkgdir/usr/share/icons/hicolor/scalable/apps/ctrlgear.svg"
  install -Dm644 udev/70-steelseries.rules "$pkgdir/usr/lib/udev/rules.d/70-steelseries.rules"
  install -Dm644 LICENSE "$pkgdir/usr/share/licenses/$pkgname/LICENSE"
  install -Dm644 README.md "$pkgdir/usr/share/doc/$pkgname/README.md"
  install -Dm644 BUTTON_MAPPING.md "$pkgdir/usr/share/doc/$pkgname/BUTTON_MAPPING.md"
}
