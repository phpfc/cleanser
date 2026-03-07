# Cleanser

<img src="icon.png" alt="Cleanser" width="120" align="right">

[![CI](https://github.com/phpfc/cleanser/actions/workflows/ci.yml/badge.svg)](https://github.com/phpfc/cleanser/actions/workflows/ci.yml)
[![Release](https://github.com/phpfc/cleanser/actions/workflows/release.yml/badge.svg)](https://github.com/phpfc/cleanser/actions/workflows/release.yml)

A blazing-fast cross-platform CLI tool for clearing storage space, written in Rust.

**Works on macOS, Linux, and Windows.**

## Quick Start

```bash
# Install (macOS)
brew tap phpfc/cleanser && brew install cleanser

# Or with cargo
cargo install --git https://github.com/phpfc/cleanser.git

# Scan your system
cleanser scan

# Preview what would be deleted
cleanser clean --dry-run

# Clean safe items
cleanser clean --risk safe
```

## Commands

### `scan` - Find cleanable files

```bash
cleanser scan                      # Normal scan
cleanser scan --speed quick        # Fast scan (depth 3)
cleanser scan --speed thorough     # Deep scan (unlimited)
cleanser scan --interactive        # Visual TUI browser
cleanser scan --paths ~/Projects   # Scan specific paths
cleanser scan --min-size 500       # Find files > 500MB
cleanser scan --older-than 90d     # Files not modified in 90 days
cleanser scan --find-duplicates    # Find duplicate files
cleanser scan --json               # JSON output
```

### `clean` - Delete files by risk level

```bash
cleanser clean                     # Clean safe items (uses cache)
cleanser clean --risk moderate     # Include build artifacts
cleanser clean --risk risky        # Include large files
cleanser clean --dry-run           # Preview only
cleanser clean --force-scan        # Bypass cache
cleanser clean --interactive       # Review files one by one
cleanser clean -y                  # Skip confirmation
```

### `map` - Filesystem mapping

Cleanser builds an intelligent map of your filesystem for faster repeated scans.

```bash
cleanser map show                  # View mapped directories
cleanser map stats                 # Detailed breakdown by category
cleanser map rebuild               # Force rebuild the map
cleanser map clear                 # Delete the map
```

### `whitelist` - Permanent exclusions

```bash
cleanser whitelist add ~/important # Never scan this path
cleanser whitelist remove ~/old    # Remove from whitelist
cleanser whitelist list            # Show all whitelisted paths
```

### `cache` - Manage scan cache

```bash
cleanser cache show                # View cached scan results
cleanser cache clear               # Clear the cache
```

## What Gets Cleaned

| Risk Level | What's Included |
|------------|-----------------|
| **Safe** | System caches, browser caches, package manager caches (npm, pip, cargo), logs, `__pycache__`, temp files |
| **Moderate** | `node_modules`, `target/` (Rust), `build/`, `dist/`, `.gradle`, `.next`, `.nuxt` |
| **Risky** | Large files (> 100MB), duplicate files |

Build directories are validated against project files (e.g., `target/` only if `Cargo.toml` exists).

## Platform Support

| Platform | Cache Locations |
|----------|-----------------|
| macOS | `~/Library/Caches`, `~/.cache`, `~/Library/Logs` |
| Linux | `~/.cache`, XDG paths, `/var/log` |
| Windows | `%LOCALAPPDATA%`, `%TEMP%` |

## Installation

**Homebrew (macOS):**
```bash
brew tap phpfc/cleanser && brew install cleanser
```

**Cargo:**
```bash
cargo install --git https://github.com/phpfc/cleanser.git
```

**From source:**
```bash
git clone https://github.com/phpfc/cleanser.git
cd cleanser
cargo build --release
sudo cp target/release/cleanser /usr/local/bin/
```

## Safety

- Always preview with `--dry-run` first
- Start with `--risk safe`
- Use `whitelist add` to protect important directories
- Build directories are validated against project files

**This tool permanently deletes files. Use at your own risk.**

## License

MIT
