#!/bin/sh
set -e

RED='\033[0;31m'
GREEN='\033[0;32m'
BLUE='\033[0;34m'
NC='\033[0m'

MARROW_DIR="$HOME/.marrow"
BIN_DIR="$MARROW_DIR/bin"
STD_DIR="$MARROW_DIR/std"

echo "${BLUE}Installing Marrow v0.1.0 Toolchain...${NC}"

OS="$(uname -s)"
ARCH="$(uname -m)"

case "$OS" in
    Linux*)     PLATFORM="unknown-linux-gnu";;
    Darwin*)    PLATFORM="apple-darwin";;
    *)          echo "${RED}Unsupported OS: $OS${NC}"; exit 1;;
esac

case "$ARCH" in
    x86_64)         ARCH="x86_64";;
    aarch64|arm64)  ARCH="aarch64";;
    *)              echo "${RED}Unsupported Architecture: $ARCH${NC}"; exit 1;;
esac

TARGET="${ARCH}-${PLATFORM}"
echo "Detected platform: ${TARGET}"

mkdir -p "$BIN_DIR"
mkdir -p "$STD_DIR"

REPO="zuygui/marrow"
TARBALL_URL="https://github.com/${REPO}/releases/latest/download/marrow-${TARGET}.tar.gz"

echo "Downloading release from ${TARBALL_URL}..."
TMP_DIR="$(mktemp -d)"
curl -sSL "$TARBALL_URL" | tar -xz -C "$TMP_DIR"

cp "$TMP_DIR/marrow" "$BIN_DIR/marrow"
chmod +x "$BIN_DIR/marrow"

if [ -d "$TMP_DIR/std" ]; then
    cp -r "$TMP_DIR/std/"* "$STD_DIR/"
fi

rm -rf "$TMP_DIR"

SHELL_NAME="$(basename "$SHELL")"
PROFILE=""

if [ "$SHELL_NAME" = "zsh" ]; then
    PROFILE="$HOME/.zshrc"
elif [ "$SHELL_NAME" = "bash" ]; then
    PROFILE="$HOME/.bashrc"
else
    PROFILE="$HOME/.profile"
fi

EXPORT_LINE='export PATH="$HOME/.marrow/bin:$PATH"'

if ! grep -q "$BIN_DIR" "$PROFILE" 2>/dev/null; then
    echo "" >> "$PROFILE"
    echo "# Marrow Programming Language" >> "$PROFILE"
    echo "$EXPORT_LINE" >> "$PROFILE"
    echo "${GREEN}Added ~/.marrow/bin to $PROFILE${NC}"
fi

echo ""
echo "${GREEN}Marrow v0.1.0 installed successfully!${NC}"
echo "Run 'source $PROFILE' or open a new terminal, then test with:"
echo "  ${BLUE}marrow --version${NC}"