# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.7.0] - 2026-03-10

### Added
- **Secure Delete**: Overwrite file data before deletion (DoD 5220.22-M standard)
  - `--secure` flag for secure deletion
  - `--secure-passes` option (1, 3, 7, or 35 for Gutmann method)
  - Configurable patterns: zeros, random, DoD, Gutmann
- **Trash/Recovery System**: Move files to trash instead of permanent deletion
  - `--trash` flag to move files to cleanser's trash
  - `cleanser trash list` - list items in trash
  - `cleanser trash restore <id>` - restore items from trash
  - `cleanser trash delete <id>` - permanently delete from trash
  - `cleanser trash empty` - empty entire trash
  - `cleanser trash stats` - show trash statistics
  - Auto-cleanup of old items (configurable)
- **Scheduler**: Automated cleanup jobs with platform-specific integration
  - `cleanser schedule set` - create scheduled cleanup jobs
  - `cleanser schedule list` - list all jobs
  - `cleanser schedule remove` - remove a job
  - `cleanser schedule enable/disable` - toggle jobs
  - `cleanser schedule run` - run a job immediately
  - `cleanser schedule history` - view job execution history
  - macOS: launchd integration (plist in ~/Library/LaunchAgents)
  - Linux: systemd user timers
  - Windows: Task Scheduler (schtasks)
  - Supports hourly, daily, weekly, monthly, and cron schedules

### Changed
- Clean command now accepts `--secure`, `--secure-passes`, and `--trash` flags
- Deletion operations use strategy pattern for extensibility

## [0.6.0] - 2026-03-10

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

## [0.5.2] - 2026-02-15

### Added
- Filesystem map for faster scanning
- Smart scan that updates stale entries
- Whitelist suggestions based on map analysis
- Map verification command

### Changed
- Improved heuristics for directory classification
- Better confidence scoring for cleanable items

## [0.5.1] - 2026-01-20

### Added
- Interactive TUI mode for scan results
- Size range filtering (`--size-range`)
- Age-based filtering (`--older-than`, `--newer-than`)

### Fixed
- Duplicate nested path detection

## [0.5.0] - 2025-12-15

### Added
- Initial public release
- CLI with scan, clean, whitelist, cache commands
- GUI application using Tauri
- Support for macOS, Linux, and Windows
- Detection of caches, build artifacts, logs, and large files
- Risk level classification (Safe, Moderate, Risky)
- Dry-run mode for safe preview

[Unreleased]: https://github.com/phpfc/cleanser/compare/v0.7.0...HEAD
[0.7.0]: https://github.com/phpfc/cleanser/compare/v0.6.0...v0.7.0
[0.6.0]: https://github.com/phpfc/cleanser/compare/v0.5.2...v0.6.0
[0.5.2]: https://github.com/phpfc/cleanser/compare/v0.5.1...v0.5.2
[0.5.1]: https://github.com/phpfc/cleanser/compare/v0.5.0...v0.5.1
[0.5.0]: https://github.com/phpfc/cleanser/releases/tag/v0.5.0
