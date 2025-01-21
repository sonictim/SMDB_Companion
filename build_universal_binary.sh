#!/bin/sh
## bash script to build UNIVERSAL BINARY for MAC of SMDB_Companion

set -e  # Exit on any error

# Set the binary name
BINARY_NAME="SMDB_Companion"
VERSION=$(awk '/\[package\]/ {flag=1} flag && /^version =/ {print $3; exit}' Cargo.toml | tr -d '"')
APP_PATH="target/universal/release/$BINARY_NAME.app"
ZIP_NAME="$BINARY_NAME.v$VERSION.zip"
GDRIVE_VERSION_FILE="/Users/tfarrell/Library/CloudStorage/GoogleDrive-tim@farrellsound.com/Shared drives/PUBLIC/$BINARY_NAME/latest_ver"
WEB_VERSION_FILE="/Users/tfarrell/Documents/Website/smdbc.com/private/latest_ver"
NOTARIZE_USERNAME="farrelltim@me.com"
# Consider using keychain or environment variable for password
NOTARIZE_PASSWORD=$(security find-generic-password -a "farrelltim@me.com" -s "AC_PASSWORD" -w)

# Ensure clean state
clean_up() {
    echo "Cleaning up temporary files..."
    [ -n "$TEMP_DIR" ] && rm -rf "$TEMP_DIR"
}
trap clean_up EXIT

export MACOSX_DEPLOYMENT_TARGET=12.0

echo "Building and Bundling Version: $VERSION"

# Check if required tools are installed
command -v rustup >/dev/null 2>&1 || { echo "rustup is required but not installed. Aborting." >&2; exit 1; }
command -v cargo-bundle >/dev/null 2>&1 || { echo "cargo-bundle is required but not installed. Installing..." >&2; cargo install cargo-bundle; }

# Add the necessary targets
rustup target add aarch64-apple-darwin x86_64-apple-darwin

# Build for both architectures
echo "Building for ARM64..."
cargo bundle --release --target aarch64-apple-darwin || { echo "ARM64 build failed"; exit 1; }
echo "Building for x86_64..."
cargo bundle --release --target x86_64-apple-darwin || { echo "x86_64 build failed"; exit 1; }

# Build a directory for universal builds
mkdir -p target/universal/release
cp -R target/x86_64-apple-darwin/release/bundle/osx/$BINARY_NAME.app target/universal/release/

# Create the universal binary
echo "Creating universal binary..."
lipo -create -output "$APP_PATH/Contents/MacOS/$BINARY_NAME" \
    "target/aarch64-apple-darwin/release/$BINARY_NAME" \
    "target/x86_64-apple-darwin/release/$BINARY_NAME"

# Verify the binary
echo "Verifying universal binary..."
file "$APP_PATH/Contents/MacOS/$BINARY_NAME"

# Code signing
echo "Code signing application..."
codesign --sign "Developer ID Application: Tim Farrell (22D9VBGAWF)" \
    --deep --force --options runtime \
    --entitlements "entitlements.plist" \
    "$APP_PATH"

# Verify code signing
echo "Verifying code signature..."
codesign --verify --deep --strict --verbose=2 "$APP_PATH"



# Before creating temp directory
ORIGINAL_DIR=$(pwd)

# Create temporary directory for notarization
TEMP_DIR=$(mktemp -d)
cd "$TEMP_DIR"

# Create the zip file
echo "Creating zip file for notarization..."
ditto -c -k --keepParent "$APP_PATH" "$ZIP_NAME"

# Get bundle identifier
BUNDLE_IDENTIFIER=$(defaults read "$APP_PATH/Contents/Info.plist" CFBundleIdentifier)
echo "Bundle identifier: $BUNDLE_IDENTIFIER"

# Submit for notarization
echo "Submitting for notarization..."
NOTARIZE_RESPONSE=$(xcrun notarytool submit "$ZIP_NAME" \
    --wait \
    --progress \
    --apple-id "$NOTARIZE_USERNAME" \
    --password "$NOTARIZE_PASSWORD" \
    --team-id "22D9VBGAWF")

# Check notarization status
if ! echo "$NOTARIZE_RESPONSE" | grep -q "status: Accepted"; then
    echo "Notarization failed:"
    echo "$NOTARIZE_RESPONSE"
    exit 1
fi

echo "Notarization successful!"

# Staple the notarization ticket
echo "Stapling notarization ticket..."
xcrun stapler staple "$APP_PATH"

# Verify stapling
if ! xcrun stapler validate "$APP_PATH"; then
    echo "Stapling verification failed"
    exit 1
fi

# Create final distribution zip
ditto -c -k --keepParent "$APP_PATH" "$ZIP_NAME"

# Copy to destination paths
echo "Copying to distribution locations..."
for DIR in "/Users/tfarrell/Documents/Website/smdbc.com/private" \
          "/Users/tfarrell/Library/CloudStorage/GoogleDrive-tim@farrellsound.com/Shared drives/PUBLIC/$BINARY_NAME"; do
    rm -f "$DIR/$BINARY_NAME"*
    cp "$ZIP_NAME" "$DIR/$ZIP_NAME"
done

# Update version files
echo "$VERSION" | tee "$GDRIVE_VERSION_FILE" "$WEB_VERSION_FILE"

echo "Build and notarization process completed successfully!"