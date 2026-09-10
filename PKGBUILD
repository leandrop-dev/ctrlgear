# Maintainer: Leandro Paixão <inoobik@gmail.com>
pkgname=steelctl-cli
pkgver=0.1.0
pkgrel=1
pkgdesc="A Rust command-line utility for managing SteelSeries hardware"
arch=('x86_64' 'aarch64')
url="https://github.com/leandrop-dev/steelctl"
license=('WTFPL')
depends=('hidapi' 'libusb')
makedepends=('cargo')

# Builds directly from this checkout (run `makepkg` from the repo root) —
# no tagged release exists yet to fetch as a source tarball. Only the CLI
# workspace member is built/packaged; the Slint GUI (gui/) is not packaged
# here.
build() {
  cd "$startdir"
  cargo build --release --locked -p steelctl-cli
}

check() {
  cd "$startdir"
  cargo test --release --locked -p steelctl-core -p steelctl-cli
}

package() {
  cd "$startdir"
  install -Dm755 target/release/steelctl-cli "$pkgdir/usr/bin/steelctl-cli"
  install -Dm644 udev/70-steelseries.rules "$pkgdir/usr/lib/udev/rules.d/70-steelseries.rules"
  install -Dm644 LICENSE "$pkgdir/usr/share/licenses/$pkgname/LICENSE"
  install -Dm644 README.md "$pkgdir/usr/share/doc/$pkgname/README.md"
  install -Dm644 BUTTON_MAPPING.md "$pkgdir/usr/share/doc/$pkgname/BUTTON_MAPPING.md"
}
