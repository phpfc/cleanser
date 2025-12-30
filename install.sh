#!/bin/bash
# Installation script for Cleanser
# This script builds and installs cleanser to /usr/local/bin

set -e

echo "🧹 Cleanser Installation Script"
echo "================================"
echo ""

# Check if Rust is installed
if ! command -v cargo &> /dev/null; then
    echo "❌ Error: Rust is not installed."
    echo "Please install Rust from https://rustup.rs/"
    exit 1
fi

# Check if we're in the right directory
if [ ! -f "Cargo.toml" ]; then
    echo "❌ Error: Please run this script from the cleanser directory"
    exit 1
fi

# Build the release binary
echo "📦 Building cleanser in release mode..."
cargo build --release

# Check if build was successful
if [ ! -f "target/release/cleanser" ]; then
    echo "❌ Error: Build failed"
    exit 1
fi

# Determine installation directory
INSTALL_DIR="/usr/local/bin"
if [ ! -w "$INSTALL_DIR" ]; then
    echo "⚠️  /usr/local/bin is not writable, will use sudo"
    USE_SUDO=true
else
    USE_SUDO=false
fi

# Install the binary
echo "📥 Installing cleanser to $INSTALL_DIR..."
if [ "$USE_SUDO" = true ]; then
    sudo cp target/release/cleanser "$INSTALL_DIR/cleanser"
    sudo chmod +x "$INSTALL_DIR/cleanser"
else
    cp target/release/cleanser "$INSTALL_DIR/cleanser"
    chmod +x "$INSTALL_DIR/cleanser"
fi

# Verify installation
if command -v cleanser &> /dev/null; then
    VERSION=$(cleanser --version)
    echo ""
    echo "✅ Installation successful!"
    echo "   Installed: $VERSION"
    echo "   Location: $(which cleanser)"
    echo ""
    echo "Try it out:"
    echo "  cleanser scan --speed quick"
    echo "  cleanser clean --dry-run"
else
    echo "❌ Installation may have failed. cleanser command not found in PATH"
    exit 1
fi
