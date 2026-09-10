# Maintainer: Leandro Paixão <inoobik@gmail.com>
pkgname=steelctl
pkgver=0.1.0
pkgrel=1
pkgdesc="A Rust utility for managing SteelSeries hardware"
arch=('x86_64' 'aarch64')
url="https://github.com/leandrop-dev/steelctl"
license=('WTFPL')
depends=('hidapi' 'libusb')
makedepends=('cargo')

# Builds directly from this checkout (run `makepkg` from the repo root) —
# no tagged release exists yet to fetch as a source tarball.
build() {
  cd "$startdir"
  cargo build --release --locked
}

check() {
  cd "$startdir"
  cargo test --release --locked
}

package() {
  cd "$startdir"
  install -Dm755 target/release/steelctl "$pkgdir/usr/bin/steelctl"
  install -Dm644 udev/70-steelseries.rules "$pkgdir/usr/lib/udev/rules.d/70-steelseries.rules"
  install -Dm644 LICENSE "$pkgdir/usr/share/licenses/$pkgname/LICENSE"
  install -Dm644 README.md "$pkgdir/usr/share/doc/$pkgname/README.md"
  install -Dm644 BUTTON_MAPPING.md "$pkgdir/usr/share/doc/$pkgname/BUTTON_MAPPING.md"
}
