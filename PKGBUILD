pkgname=hojicha-ai-git
pkgver=0.1.0
pkgrel=1
pkgdesc="Lightweight AI-powered Linux CLI assistant for beginners"
arch=('x86_64')
url="https://github.com/tealinuxos/hojicha-AI"
license=('GPL')
depends=('gcc-libs')
makedepends=('cargo' 'git')
provides=("hojicha-ai")
conflicts=("hojicha-ai")
source=("git+https://github.com/tealinuxos/hojicha-AI.git")
md5sums=('SKIP')

pkgver() {
  cd "$srcdir/hojicha-AI"
  printf "r%s.%s" "$(git rev-list --count HEAD)" "$(git rev-parse --short HEAD)"
}

build() {
  cd "$srcdir/hojicha-AI"
  export RUSTUP_TOOLCHAIN=stable
  export CARGO_TARGET_DIR=target
  cargo build --release
}

package() {
  cd "$srcdir/hojicha-AI"
  install -Dm0755 -t "$pkgdir/usr/bin/" "target/release/hojicha"
}

