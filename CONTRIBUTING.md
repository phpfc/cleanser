# Contributing to Cleanser

Thank you for your interest in contributing to Cleanser! This document provides guidelines and instructions for contributing.

## Code of Conduct

Please be respectful and constructive in all interactions. We aim to maintain a welcoming environment for all contributors.

## Getting Started

### Prerequisites

- Rust 1.70 or later
- For GUI development: Node.js 18+ and pnpm

### Setting Up the Development Environment

1. Clone the repository:
   ```bash
   git clone https://github.com/phpfc/cleanser.git
   cd cleanser
   ```

2. Build the project:
   ```bash
   cargo build
   ```

3. Run tests:
   ```bash
   cargo test --workspace --exclude cleanser-gui
   ```

### Project Structure

```
cleanser/
├── crates/
│   ├── cleanser-core/    # Core library (scanning, cleaning logic)
│   ├── cleanser-cli/     # Command-line interface
│   └── cleanser-gui/     # Desktop GUI (Tauri + React)
├── tests/                # Integration tests
└── Formula/              # Homebrew formula
```

## Development Workflow

### Branching Strategy

- `main` - Stable releases
- `develop` - Integration branch for features
- `feature/*` - New features
- `fix/*` - Bug fixes

### Making Changes

1. Create a new branch from `main`:
   ```bash
   git checkout -b feature/your-feature-name
   ```

2. Make your changes following the code style guidelines

3. Add tests for new functionality

4. Run the full test suite:
   ```bash
   cargo test --workspace --exclude cleanser-gui
   cargo clippy --workspace --exclude cleanser-gui -- -D warnings
   cargo fmt --all -- --check
   ```

5. Commit your changes with a clear message:
   ```bash
   git commit -m "feat: add new feature description"
   ```

### Commit Message Format

We follow [Conventional Commits](https://www.conventionalcommits.org/):

- `feat:` - New features
- `fix:` - Bug fixes
- `docs:` - Documentation changes
- `style:` - Code style changes (formatting, etc.)
- `refactor:` - Code refactoring
- `test:` - Adding or updating tests
- `chore:` - Maintenance tasks

### Pull Requests

1. Push your branch to GitHub
2. Create a Pull Request against `main`
3. Fill out the PR template
4. Wait for CI checks to pass
5. Request review from maintainers

## Code Style

### Rust

- Follow Rust standard formatting (`cargo fmt`)
- Use `clippy` with `-D warnings`
- Write doc comments for public APIs
- Prefer descriptive variable names
- Use `anyhow::Result` for error handling in application code
- Use specific error types in library code where appropriate

### Testing

- Write unit tests for new functions
- Add integration tests for user-facing features
- Test edge cases (empty inputs, large files, special characters)
- Ensure tests pass on all platforms (macOS, Linux, Windows)

## Areas for Contribution

### Good First Issues

Look for issues labeled `good first issue` on GitHub.

### Feature Ideas

- Improved duplicate detection algorithms
- Cloud storage integration (Dropbox, iCloud, Google Drive caches)
- Scheduled cleanup automation
- Historical cleanup reports
- Additional platform-specific cleaners

### Documentation

- Improve README examples
- Add architecture documentation
- Create user guides for common scenarios

## Security

### Reporting Vulnerabilities

For security issues, please email security@cleanser-claws.app instead of creating a public issue.

### Security Guidelines

When contributing:
- Never commit credentials or secrets
- Validate all file paths
- Be careful with symlinks and hardlinks
- Test deletion operations thoroughly
- Consider privilege escalation risks

## Questions?

- Open a GitHub Discussion for general questions
- Check existing issues before creating new ones
- Join our community chat (link TBD)

## License

By contributing, you agree that your contributions will be licensed under the MIT License.
