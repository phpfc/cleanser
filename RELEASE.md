# Release Checklist

Use this checklist when creating a new release of Cleanser.

## Pre-Release

- [ ] Update version in `Cargo.toml`
- [ ] Update version in `Formula/cleanser.rb`
- [ ] Update CHANGELOG.md (if you have one)
- [ ] Run tests: `cargo test`
- [ ] Build release binary: `cargo build --release`
- [ ] Test the binary manually
- [ ] Commit all changes

## Create Release

1. **Tag the release:**
   ```bash
   git tag v0.1.0
   git push origin v0.1.0
   ```

2. **Create GitHub Release:**
   - Go to https://github.com/phpfc/cleanser/releases/new
   - Select the tag you just created
   - Title: `v0.1.0`
   - Description: List of changes and improvements
   - Attach the binary (optional): `target/release/cleanser`
   - Publish release

3. **Get the tarball SHA256:**
   ```bash
   curl -L https://github.com/phpfc/cleanser/archive/refs/tags/v0.1.0.tar.gz | shasum -a 256
   ```

## Update Homebrew Tap (if you have one)

1. **Update the formula in `homebrew-cleanser` repository:**
   ```ruby
   url "https://github.com/phpfc/cleanser/archive/refs/tags/v0.1.0.tar.gz"
   sha256 "calculated_sha256_here"
   ```

2. **Test the formula:**
   ```bash
   brew tap phpfc/cleanser
   brew install cleanser --build-from-source
   brew test cleanser
   cleanser --version
   ```

3. **Commit and push:**
   ```bash
   git add Formula/cleanser.rb
   git commit -m "Update cleanser to v0.1.0"
   git push origin main
   ```

## Post-Release

- [ ] Test installation from all methods (brew, cargo, install.sh)
- [ ] Update README if needed
- [ ] Announce on social media / relevant forums
- [ ] Close related issues/PRs on GitHub

## Quick Commands

```bash
# Version bump workflow
vim Cargo.toml                    # Update version
cargo build --release             # Test build
git add Cargo.toml
git commit -m "Bump version to 0.1.0"
git tag v0.1.0
git push origin main
git push origin v0.1.0

# Calculate SHA256 for Homebrew
curl -L https://github.com/phpfc/cleanser/archive/refs/tags/v0.1.0.tar.gz | shasum -a 256
```
