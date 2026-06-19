pkgname=hojicha-ai-git
pkgver=r8.b5a3bf4
pkgrel=1
pkgdesc="Lightweight AI-powered Linux CLI assistant for beginners"
arch=('x86_64')
url="https://github.com/tealinuxos/hojicha-AI"
license=('GPL')
depends=('gcc-libs')
makedepends=('cargo' 'git')
provides=("hojicha-ai")
conflicts=("hojicha-ai")
source=()
md5sums=()

pkgver() {
  cd "$startdir"
  printf "r%s.%s" "$(git rev-list --count HEAD)" "$(git rev-parse --short HEAD)"
}

build() {
	CFLAGS+=' -ffat-lto-objects'
  cd "$startdir"
  export RUSTUP_TOOLCHAIN=stable
  export CARGO_TARGET_DIR=target
  cargo build --release
}

package() {
  cd "$startdir"
  install -Dm0755 -t "$pkgdir/usr/bin/" "target/release/hojicha"
}
