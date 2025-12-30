# Cleanser

A blazing-fast CLI tool for clearing macOS storage space, written in Rust.

## Quick Start

```bash
# Install
brew tap phpfc/cleanser
brew install cleanser

# Scan your system
cleanser scan --speed normal

# Clean safe items (with preview)
cleanser clean --dry-run

# Actually clean
cleanser clean --risk safe
```

## Features

- **Smart Caching**: Scan results are cached (`~/.cache/cleanser/last-scan.json`) so `clean` doesn't re-scan unnecessarily
- **Dynamic Discovery**: Pattern-based scanning finds cache directories, build artifacts, and logs anywhere in your filesystem
- **Three scan speeds**: Quick (depth 3), Normal (depth 6), or Thorough (unlimited depth)
- **Large file detection**: Find files above a configurable size threshold (default 100MB)
- **Duplicate file finder**: SHA-256 based detection of duplicate files with parallel hashing
- **Custom scan paths**: Scan specific directories instead of just your home folder
- **Risk-based cleanup**: Safe, Moderate, or Risky levels to control what gets deleted
- **Interactive confirmations**: Prevent accidental deletions with built-in prompts
- **Dry-run mode**: Preview what would be deleted without actually deleting
- **JSON output**: Machine-readable output for integration with other tools
- **Parallel scanning**: Leverages Rust's Rayon for blazing-fast concurrent operations
- **Progress indicators**: Real-time feedback during long-running scans

## What Gets Cleaned (Dynamically Discovered)

### Safe (Low Risk)
- **Cache directories**: Any directory matching patterns like `*cache*`, `*Cache*`, `.cache`
  - Browser caches (Chrome, Firefox, Safari)
  - System caches (`~/Library/Caches`)
  - Package manager caches (npm, pip, cargo, homebrew)
- **Log files**: `.log` files over 10MB in common log directories
- **Python artifacts**: `__pycache__`, `.pytest_cache` directories
- **Temporary files**: System temp directories

### Moderate Risk
- **Node.js**: `node_modules` directories (validated against `package.json`)
- **Build outputs**: `build/`, `dist/`, `out/` directories (validated against project files)
- **Rust**: `target/` directories (validated against `Cargo.toml`)
- **Java/Gradle**: `.gradle`, `.maven` directories
- **Modern frameworks**: `.next`, `.nuxt` build caches

### Risky (Requires Review)
- **Large files**: Files exceeding size threshold (configurable, default 100MB)
- **Duplicate files**: Exact copies detected via SHA-256 hashing

## Installation

### Option 1: Homebrew (Recommended)

```bash
brew tap phpfc/cleanser
brew install cleanser
```

### Option 2: Using the Install Script

```bash
# Clone and run the install script
git clone https://github.com/phpfc/cleanser.git
cd cleanser
./install.sh
```

This builds the release binary and copies it to `/usr/local/bin` (may require sudo).

### Option 3: Cargo Install (Rust Users)

```bash
# Install directly from source
cargo install --git https://github.com/phpfc/cleanser.git

# Or clone first
git clone https://github.com/phpfc/cleanser.git
cd cleanser
cargo install --path .
```

This installs to `~/.cargo/bin/cleanser` (make sure `~/.cargo/bin` is in your PATH).

### Option 4: Manual Build

```bash
# Clone the repository
git clone https://github.com/phpfc/cleanser.git
cd cleanser

# Build the release binary
cargo build --release

# The binary will be at target/release/cleanser
# Copy it to a directory in your PATH:
sudo cp target/release/cleanser /usr/local/bin/
```

### Uninstallation

**If installed via Homebrew:**
```bash
brew uninstall cleanser
```

**If installed via install script or manually:**
```bash
# Use the uninstall script
./uninstall.sh

# Or manually remove
sudo rm /usr/local/bin/cleanser
rm -rf ~/.cache/cleanser  # Optional: remove cache
```

**If installed via cargo:**
```bash
cargo uninstall cleanser
```

## Usage

### Scan for cleanable files

```bash
# Quick scan (depth 3, fastest)
cleanser scan --speed quick

# Normal scan (depth 6, default)
cleanser scan

# Thorough scan (unlimited depth, finds everything)
cleanser scan --speed thorough

# Scan specific directories
cleanser scan --paths ~/Projects ~/Downloads

# Find large files over 500MB
cleanser scan --min-size 500

# Find duplicate files (uses SHA-256 hashing)
cleanser scan --find-duplicates

# Limit scan depth
cleanser scan --max-depth 4

# Combine options: scan Projects for large files and duplicates
cleanser scan --paths ~/Projects --min-size 100 --find-duplicates --speed thorough

# Output as JSON
cleanser scan --json
```

### Clean files

```bash
# Clean only safe items (default)
# Uses cached scan results if available (< 1 hour old)
cleanser clean

# Force a fresh scan instead of using cache
cleanser clean --force-scan

# Clean up to moderate risk items
cleanser clean --risk moderate

# Clean all items including risky ones
cleanser clean --risk risky

# Dry-run mode (see what would be deleted)
cleanser clean --dry-run

# Skip confirmation prompt
cleanser clean --yes

# Combine options
cleanser clean --risk moderate --dry-run
```

### Caching Behavior

Scan results are automatically cached to `~/.cache/cleanser/last-scan.json` for 1 hour. This means:

```bash
# First, run a scan to see what can be cleaned
cleanser scan --speed normal

# Later, clean without re-scanning (uses cache)
cleanser clean --risk safe

# Output: "Using cached scan results from 5 min 23 sec ago"
```

The cache is invalidated after 1 hour or when you run `cleanser scan` again. Use `--force-scan` to bypass the cache:

```bash
# Always scan fresh, ignore cache
cleanser clean --force-scan --risk moderate
```

To prevent caching during a scan:

```bash
# Don't save results to cache
cleanser scan --no-cache
```

## Examples

### Find out how much space you can free

```bash
$ cleanser scan --speed thorough

Scanning with thorough speed...
Scanning 12 locations...

=== Scan Results ===
Total cleanable space: 4.2 GB

Safe Risk (1.8 GB)
  System Cache - 512 MB - ~/Library/Caches
  Browser Cache - 890 MB - ~/Library/Caches/Google/Chrome
  System Logs - 156 MB - ~/Library/Logs
  Temporary Files - 242 MB - /tmp

Moderate Risk (2.4 GB)
  Node Modules - 1.2 GB - ~/Dev/project1/node_modules
  Node Modules - 890 MB - ~/Dev/project2/node_modules
  Cargo Cache - 310 MB - ~/.cargo/registry

Run 'cleanser clean --risk <level>' to clean files
```

### Preview what will be deleted

```bash
$ cleanser clean --risk moderate --dry-run

Scanning for cleanable items...
Scanning 8 locations...

=== Items to Clean ===
Total space to free: 2.1 GB

✓ System Cache - 512 MB - ~/Library/Caches
✓ Browser Cache - 890 MB - ~/Library/Caches/Google/Chrome
⚠ Node Modules - 1.2 GB - ~/Dev/old-project/node_modules

DRY RUN: No files were deleted.
```

### Actually clean files

```bash
$ cleanser clean --risk safe

Scanning for cleanable items...

=== Items to Clean ===
Total space to free: 1.6 GB

✓ System Cache - 512 MB - ~/Library/Caches
✓ Browser Cache - 890 MB - ~/Library/Caches/Google/Chrome
✓ System Logs - 156 MB - ~/Library/Logs

This will delete files. Continue? (y/N)
y

✓ Cleaned: ~/Library/Caches
✓ Cleaned: ~/Library/Caches/Google/Chrome
✓ Cleaned: ~/Library/Logs

=== Cleanup Summary ===
Cleaned: 3 items
Failed: 0 items
Space freed: 1.6 GB
```

## Safety Features

- **Smart validation**: Build directories are validated against project files (e.g., `target/` must have `Cargo.toml`)
- **Pattern matching**: Uses regex patterns to identify safe-to-delete directories
- **Skip system directories**: Automatically skips `/System`, `/Library`, `Applications`, etc.
- **Confirmation prompts**: By default, you'll be asked to confirm before deletion
- **Dry-run mode**: Test what will be deleted with `--dry-run`
- **Risk levels**: Control what gets deleted with `--risk` flag
- **Detailed output**: See exactly what's being deleted with file sizes and categories

## Performance

Cleanser is built with performance in mind:
- **Written in Rust**: Maximum speed, memory safety, and zero-cost abstractions
- **Parallel everything**: Directory scanning, file hashing, and size calculations use Rayon
- **Efficient hashing**: SHA-256 with 8KB buffers for fast duplicate detection
- **Smart traversal**: Configurable depth limits to avoid scanning unnecessary directories
- **Minimal dependencies**: Fast compilation and small binary size
- **Single binary**: No runtime required, just download and run

## Development

### Prerequisites

- Rust 1.70 or higher
- macOS (this tool is macOS-specific)

### Building

```bash
cargo build
```

### Running tests

```bash
cargo test
```

### Running in development

```bash
cargo run -- scan --speed quick
cargo run -- clean --dry-run
```

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## License

MIT

## Roadmap

- [x] Add large file detection with configurable thresholds
- [x] Add duplicate file detection with SHA-256 hashing
- [x] Add support for custom scan locations
- [x] Add more development tool caches (Gradle, Maven, .next, .nuxt, etc.)
- [x] Dynamic pattern-based discovery of caches and build artifacts
- [x] Smart caching system to avoid re-scanning on clean operations
- [ ] Add configurable exclusion patterns (regex-based)
- [ ] Add scheduled cleanup support (cron integration)
- [ ] Add interactive TUI mode for reviewing files before deletion
- [ ] Generate detailed cleanup reports (HTML/PDF)
- [ ] Add compression detection (find already-compressed files in archives)
- [ ] Support for other operating systems (Linux, Windows)
- [ ] Config file support (~/.cleanser.toml)

## Safety Disclaimer

This tool permanently deletes files. While it's designed to only target safe temporary files and caches, always:
- Review what will be deleted using `--dry-run`
- Start with `--risk safe`
- Have backups of important data
- Use at your own risk

The authors are not responsible for any data loss.
