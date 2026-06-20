#!/usr/bin/env bash
# install.sh — Install hojicha ke PATH agar bisa dipanggil dari mana saja
# Usage: chmod +x install.sh && ./install.sh

set -e

BINARY_NAME="hojicha"
INSTALL_DIR="$HOME/.local/bin"
BOLD='\033[1m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m' # No Color

echo ""
echo -e "${BOLD}╔══════════════════════════════════════╗${NC}"
echo -e "${BOLD}║     Hojicha-AI Installer             ║${NC}"
echo -e "${BOLD}╚══════════════════════════════════════╝${NC}"
echo ""

# ── 1. Cek Rust/Cargo ──────────────────────────────────────────
if ! command -v cargo &> /dev/null; then
    echo -e "${RED}✗ Cargo tidak ditemukan.${NC}"
    echo ""
    echo "  Install Rust terlebih dahulu:"
    echo "  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
    echo ""
    exit 1
fi

echo -e "  ${GREEN}✓${NC} Rust/Cargo ditemukan: $(cargo --version)"

# ── 2. Build release binary ─────────────────────────────────────
echo ""
echo -e "  ${BOLD}→ Membangun binary (mode release)...${NC}"
echo ""

cargo build --release

echo ""
echo -e "  ${GREEN}✓${NC} Build berhasil."

# ── 3. Siapkan install directory ────────────────────────────────
mkdir -p "$INSTALL_DIR"

# ── 4. Copy binary ──────────────────────────────────────────────
BINARY_SRC="target/release/$BINARY_NAME"

if [ ! -f "$BINARY_SRC" ]; then
    echo -e "${RED}✗ Binary tidak ditemukan di $BINARY_SRC${NC}"
    exit 1
fi

cp "$BINARY_SRC" "$INSTALL_DIR/$BINARY_NAME"
chmod +x "$INSTALL_DIR/$BINARY_NAME"

echo -e "  ${GREEN}✓${NC} Binary dipasang ke: ${BOLD}$INSTALL_DIR/$BINARY_NAME${NC}"

# ── 5. Cek PATH ──────────────────────────────────────────────────
echo ""
if echo "$PATH" | tr ':' '\n' | grep -qx "$INSTALL_DIR"; then
    echo -e "  ${GREEN}✓${NC} $INSTALL_DIR sudah ada di PATH."
else
    echo -e "  ${YELLOW}⚠${NC}  $INSTALL_DIR belum ada di PATH."
    echo ""
    echo "  Tambahkan baris berikut ke shell config kamu:"
    echo ""

    SHELL_NAME=$(basename "$SHELL")
    case "$SHELL_NAME" in
        zsh)
            RCFILE="$HOME/.zshrc"
            ;;
        bash)
            RCFILE="$HOME/.bashrc"
            ;;
        fish)
            RCFILE="$HOME/.config/fish/config.fish"
            ;;
        *)
            RCFILE="~/.bashrc atau ~/.zshrc"
            ;;
    esac

    echo -e "    ${BOLD}export PATH=\"\$HOME/.local/bin:\$PATH\"${NC}"
    echo ""
    echo "  Lalu jalankan: source $RCFILE"
    echo ""

    # Auto-append ke rcfile jika bisa
    if [ "$SHELL_NAME" = "fish" ]; then
        # Fish shell: gunakan fish_add_path atau set -gx
        mkdir -p "$(dirname "$RCFILE")"
        if command -v fish &> /dev/null && fish -c "fish_add_path $INSTALL_DIR" 2>/dev/null; then
            echo -e "  ${GREEN}✓${NC} PATH sudah diupdate via fish_add_path"
        else
            # Fallback: tulis ke config.fish
            echo "" >> "$RCFILE"
            echo "# Added by hojicha install.sh" >> "$RCFILE"
            echo "set -gx PATH \$HOME/.local/bin \$PATH" >> "$RCFILE"
            echo -e "  ${GREEN}✓${NC} PATH sudah diupdate di $RCFILE"
            echo -e "  ${YELLOW}→${NC}  Jalankan: source $RCFILE"
        fi
    elif [ "$SHELL_NAME" = "zsh" ] || [ "$SHELL_NAME" = "bash" ]; then
        read -r -p "  Otomatis tambahkan ke $RCFILE? [y/N] " response
        if [[ "$response" =~ ^[Yy]$ ]]; then
            echo "" >> "$RCFILE"
            echo "# Added by hojicha install.sh" >> "$RCFILE"
            echo "export PATH=\"\$HOME/.local/bin:\$PATH\"" >> "$RCFILE"
            echo -e "  ${GREEN}✓${NC} PATH sudah diupdate di $RCFILE"
            echo -e "  ${YELLOW}→${NC}  Jalankan: source $RCFILE"
        fi
    fi
fi

# ── 6. Done ─────────────────────────────────────────────────────
echo ""
echo -e "${GREEN}${BOLD}✓ Instalasi selesai!${NC}"
echo ""
echo "  Sekarang kamu bisa ketik:"
echo -e "    ${BOLD}hojicha${NC}              → mode interaktif"
echo -e "    ${BOLD}hojicha \"cek ram\"${NC}     → mode langsung"
echo -e "    ${BOLD}hojicha --help${NC}        → lihat semua opsi"
echo ""
echo "  (Jika perintah belum ditemukan, reload shell dulu: source $RCFILE)"
echo ""
