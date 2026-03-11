# Code Signing and Notarization Setup

This document explains how to set up code signing and notarization for macOS releases.

## Prerequisites

- **Apple Developer Account** ($99/year)
- **Developer ID Application Certificate** from Apple Developer portal
- **App-specific password** for notarization

## Required GitHub Secrets

Add these secrets to your GitHub repository at `Settings → Secrets and variables → Actions`:

### 1. `APPLE_CERTIFICATE`

Your **Developer ID Application** certificate in base64 format.

**How to get it:**

1. Open **Keychain Access** on macOS
2. Find your "Developer ID Application" certificate
3. Right-click → **Export "Developer ID Application: Your Name"**
4. Save as `.p12` file with a password
5. Convert to base64:
   ```bash
   base64 -i /path/to/certificate.p12 | pbcopy
   ```
6. Paste the output as the secret value

### 2. `APPLE_CERTIFICATE_PASSWORD`

The password you used when exporting the `.p12` certificate.

### 3. `APPLE_SIGNING_IDENTITY`

The full name of your signing identity. Usually looks like:
```
Developer ID Application: Your Name (TEAM_ID)
```

**How to find it:**
```bash
security find-identity -v -p codesigning
```

### 4. `APPLE_ID`

Your Apple ID email address (the one used for your Developer account).

### 5. `APPLE_PASSWORD`

An **app-specific password** (NOT your Apple ID password).

**How to create it:**

1. Go to [appleid.apple.com](https://appleid.apple.com)
2. Sign in
3. Go to **Security** → **App-Specific Passwords**
4. Click **Generate an app-specific password**
5. Enter a label (e.g., "Cleanser Notarization")
6. Copy the generated password

### 6. `APPLE_TEAM_ID`

Your 10-character Team ID.

**How to find it:**

1. Go to [developer.apple.com/account](https://developer.apple.com/account)
2. Go to **Membership** section
3. Copy the **Team ID** (format: `AB1CD2EF34`)

## Verification

After setting up all secrets, the next release will:

1. ✅ **Import** the signing certificate
2. ✅ **Sign** the macOS app bundle
3. ✅ **Notarize** with Apple's servers
4. ✅ **Staple** the notarization ticket to the DMG

Users will be able to open the app without any security warnings.

## Testing Locally

To test signing locally before pushing:

```bash
# Build the app
cd crates/cleanser-gui
npm run tauri build

# Check if it's signed
codesign -dv --verbose=4 target/release/bundle/macos/Cleanser.app

# Verify the signature
codesign -v target/release/bundle/macos/Cleanser.app
spctl -a -vvv -t install target/release/bundle/macos/Cleanser.app
```

## Troubleshooting

### "No identity found" error

Make sure:
- Certificate is valid and not expired
- `APPLE_SIGNING_IDENTITY` matches exactly (run `security find-identity -v -p codesigning`)
- Certificate is a **Developer ID Application** certificate (not "Mac App Distribution")

### Notarization fails

Common issues:
- `APPLE_PASSWORD` is the regular password, not app-specific → Create app-specific password
- `APPLE_TEAM_ID` is incorrect → Check developer.apple.com/account
- App has issues (hardened runtime violations, missing entitlements) → Check notarization logs

### How to check notarization status

```bash
xcrun notarytool log <submission-id> \
  --apple-id "$APPLE_ID" \
  --password "$APPLE_PASSWORD" \
  --team-id "$APPLE_TEAM_ID"
```

## References

- [Apple Code Signing Guide](https://developer.apple.com/support/code-signing/)
- [Notarization Documentation](https://developer.apple.com/documentation/security/notarizing_macos_software_before_distribution)
- [Tauri Code Signing](https://tauri.app/v1/guides/distribution/sign-macos)
