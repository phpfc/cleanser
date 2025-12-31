# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.3] - 2024-12-31

### Fixed
- Fixed double-counting bug in scan results where nested directories were counted multiple times, causing inflated size reports (reduced reported size from 11.73 GiB to accurate 46.41 MiB in test case)
- Implemented path deduplication to ensure accurate size calculations
- Fixed all clippy linting warnings

### Added
- Comprehensive CI/CD pipeline with GitHub Actions
- Automated testing on every push and pull request
- Multi-architecture builds (Intel and Apple Silicon)
- Automated releases with binary artifacts
- Code formatting and linting checks in CI

## [0.1.1] - 2024-XX-XX

### Added
- Homebrew bottle support
- Install and uninstall scripts
- Smart caching for scan results

## [0.1.0] - Initial Release

### Added
- Dynamic filesystem scanning with configurable depth
- Pattern-based cache directory detection
- Build artifact detection (node_modules, target, etc.)
- Log file detection
- Large file finder with configurable threshold
- Duplicate file detection using SHA-256
- Risk-based cleanup (Safe, Moderate, Risky)
- Dry-run mode for previewing deletions
- Interactive confirmation prompts
- JSON output support
- Parallel scanning with Rayon
- Progress indicators
- Smart cache validation for build directories
