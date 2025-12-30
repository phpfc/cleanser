#!/bin/bash
# Uninstallation script for Cleanser

set -e

echo "🧹 Cleanser Uninstallation Script"
echo "=================================="
echo ""

INSTALL_DIR="/usr/local/bin"
BINARY_PATH="$INSTALL_DIR/cleanser"
CACHE_DIR="$HOME/.cache/cleanser"

# Check if cleanser is installed
if [ ! -f "$BINARY_PATH" ]; then
    echo "ℹ️  cleanser is not installed at $BINARY_PATH"
else
    # Remove the binary
    if [ ! -w "$INSTALL_DIR" ]; then
        echo "🗑️  Removing cleanser binary (requires sudo)..."
        sudo rm -f "$BINARY_PATH"
    else
        echo "🗑️  Removing cleanser binary..."
        rm -f "$BINARY_PATH"
    fi
    echo "✅ Binary removed"
fi

# Ask about cache directory
if [ -d "$CACHE_DIR" ]; then
    echo ""
    read -p "Remove cache directory ($CACHE_DIR)? [y/N] " -n 1 -r
    echo
    if [[ $REPLY =~ ^[Yy]$ ]]; then
        rm -rf "$CACHE_DIR"
        echo "✅ Cache directory removed"
    else
        echo "ℹ️  Cache directory kept"
    fi
fi

echo ""
echo "✅ Uninstallation complete!"
