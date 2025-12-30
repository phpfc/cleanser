# Setting Up a Homebrew Tap for Cleanser

This guide explains how to publish Cleanser as a Homebrew package.

## Quick Overview

Homebrew "taps" are third-party repositories of formulae. By creating a tap, users can install Cleanser with:

```bash
brew tap phpfc/cleanser
brew install cleanser
```

## Step-by-Step Setup

### 1. Create a Tap Repository on GitHub

Create a new GitHub repository named `homebrew-cleanser` (must start with `homebrew-`):

```bash
# The naming convention is: homebrew-<tap-name>
https://github.com/pedrohenrique/homebrew-cleanser
```

### 2. Add the Formula to Your Tap

In your `homebrew-cleanser` repository, create a directory structure:

```
homebrew-cleanser/
└── Formula/
    └── cleanser.rb
```

Copy the `Formula/cleanser.rb` file from this project to your tap repository.

### 3. Create a GitHub Release

In the main `cleanser` repository:

1. Tag a release:
   ```bash
   git tag v0.1.0
   git push origin v0.1.0
   ```

2. Create a release on GitHub with the tag

3. GitHub will automatically create a source tarball at:
   ```
   https://github.com/phpfc/cleanser/archive/refs/tags/v0.1.0.tar.gz
   ```

### 4. Update the Formula

Calculate the SHA256 of the tarball:

```bash
curl -L https://github.com/phpfc/cleanser/archive/refs/tags/v0.1.0.tar.gz | shasum -a 256
```

Update `cleanser.rb` in your tap repository with:
- The correct `url` (the tarball URL)
- The correct `sha256` (the hash you just calculated)

### 5. Test the Formula Locally

```bash
# Install from your local tap
brew tap phpfc/cleanser
brew install cleanser --build-from-source

# Test it works
cleanser --version

# Uninstall
brew uninstall cleanser
brew untap phpfc/cleanser
```

### 6. Publish Your Tap

Commit and push the formula to your `homebrew-cleanser` repository:

```bash
cd homebrew-cleanser
git add Formula/cleanser.rb
git commit -m "Add cleanser formula v0.1.0"
git push origin main
```

### 7. Users Can Now Install

Anyone can now install Cleanser with:

```bash
brew tap phpfc/cleanser
brew install cleanser
```

Or in one command:

```bash
brew install phpfc/cleanser/cleanser
```

## Updating the Formula

When you release a new version:

1. Create a new GitHub release with a new tag (e.g., `v0.2.0`)
2. Calculate the new SHA256
3. Update the formula in `homebrew-cleanser`:
   ```ruby
   url "https://github.com/phpfc/cleanser/archive/refs/tags/v0.2.0.tar.gz"
   sha256 "new_sha256_here"
   ```
4. Commit and push

Users can then upgrade with:
```bash
brew update
brew upgrade cleanser
```

## Alternative: Submit to Homebrew Core

For wider distribution, you can submit to the official Homebrew repository:

1. Ensure your formula meets [Homebrew's requirements](https://docs.brew.sh/Acceptable-Formulae)
2. Fork [homebrew/homebrew-core](https://github.com/Homebrew/homebrew-core)
3. Add your formula to `Formula/c/cleanser.rb`
4. Submit a pull request

This makes it available as simply:
```bash
brew install cleanser
```

## Example: Complete Formula

Here's what a complete formula looks like:

```ruby
class Cleanser < Formula
  desc "Fast CLI tool for clearing macOS storage space"
  homepage "https://github.com/phpfc/cleanser"
  url "https://github.com/phpfc/cleanser/archive/refs/tags/v0.1.0.tar.gz"
  sha256 "a1b2c3d4e5f6..."  # Your actual SHA256
  license "MIT"
  head "https://github.com/phpfc/cleanser.git", branch: "main"

  depends_on "rust" => :build

  def install
    system "cargo", "install", *std_cargo_args
  end

  test do
    assert_match "cleanser 0.1.0", shell_output("#{bin}/cleanser --version")
  end
end
```

## Testing the Formula

Before publishing, test thoroughly:

```bash
# Audit the formula for issues
brew audit --strict --online cleanser

# Test installation
brew install --build-from-source cleanser
brew test cleanser

# Test uninstall
brew uninstall cleanser
```

## Resources

- [Homebrew Formula Cookbook](https://docs.brew.sh/Formula-Cookbook)
- [How to Create and Maintain a Tap](https://docs.brew.sh/How-to-Create-and-Maintain-a-Tap)
- [Acceptable Formulae](https://docs.brew.sh/Acceptable-Formulae)
