# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.6.0] - 2025-03-10

### Added
- Version update checker with GitHub releases API integration
- File locking for cache operations to prevent race conditions
- Protection against symlinks pointing outside directories during deletion
- Structured logging with `tracing` (configurable via `CLEANSER_LOG` env var)
- CLI configuration system (`cleanser config` command)
- Cross-platform CI testing (Linux, Windows, macOS)
- Partial hash pre-filtering for faster duplicate detection
- GUI: Configurable scan settings (min file size, find duplicates)
- GUI: Improved accessibility with ARIA attributes
- Edge case tests for Unicode paths, symlinks, deep nesting, etc.

### Changed
- Consolidated duplicate `get_dir_size` functions into single optimized implementation
- Improved error handling: critical errors are now logged instead of silently ignored
- Whitelist now validates paths before adding (must exist and be a directory)

### Fixed
- Race conditions in cache file operations
- Removed unused `#[allow(dead_code)]` attributes

## [0.5.2] - 2024-12-XX

### Added
- Filesystem map for faster scanning
- Smart scan that updates stale entries
- Whitelist suggestions based on map analysis
- Map verification command

### Changed
- Improved heuristics for directory classification
- Better confidence scoring for cleanable items

## [0.5.1] - 2024-11-XX

### Added
- Interactive TUI mode for scan results
- Size range filtering (`--size-range`)
- Age-based filtering (`--older-than`, `--newer-than`)

### Fixed
- Duplicate nested path detection

## [0.5.0] - 2024-10-XX

### Added
- Initial public release
- CLI with scan, clean, whitelist, cache commands
- GUI application using Tauri
- Support for macOS, Linux, and Windows
- Detection of caches, build artifacts, logs, and large files
- Risk level classification (Safe, Moderate, Risky)
- Dry-run mode for safe preview

[Unreleased]: https://github.com/phpfc/cleanser/compare/v0.6.0...HEAD
[0.6.0]: https://github.com/phpfc/cleanser/compare/v0.5.2...v0.6.0
[0.5.2]: https://github.com/phpfc/cleanser/compare/v0.5.1...v0.5.2
[0.5.1]: https://github.com/phpfc/cleanser/compare/v0.5.0...v0.5.1
[0.5.0]: https://github.com/phpfc/cleanser/releases/tag/v0.5.0
