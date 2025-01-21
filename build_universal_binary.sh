#!/bin/sh
## bash script to build UNIVERSAL BINARY for MAC of SMDB_Companion

# Set the binary name
BINARY_NAME="SMDB_Companion"
VERSION=$(awk '/\[package\]/ {flag=1} flag && /^version =/ {print $3; exit}' Cargo.toml | tr -d '"')

export MACOSX_DEPLOYMENT_TARGET=12.0

echo Building and Bundling Version: $VERSION

# Add the necessary targets
rustup target add aarch64-apple-darwin x86_64-apple-darwin
cargo install cargo-bundle

# Build for both architectures
cargo bundle --release --target aarch64-apple-darwin
cargo bundle --release --target x86_64-apple-darwin

# Build a directory for universal builds
mkdir -p target/universal/release
cp -R target/x86_64-apple-darwin/release/bundle/osx/$BINARY_NAME.app target/universal/release/

# Create the universal binary
lipo -create -output target/universal/release/$BINARY_NAME.app/Contents/MacOS/$BINARY_NAME target/aarch64-apple-darwin/release/$BINARY_NAME target/x86_64-apple-darwin/release/$BINARY_NAME

# Verify the binary
file target/universal/release/$BINARY_NAME.app/Contents/MacOS/$BINARY_NAME


# codesign the binary
codesign --sign "Developer ID Application: Tim Farrell (22D9VBGAWF)" --deep --force target/universal/release/$BINARY_NAME.app/Contents/MacOS/$BINARY_NAME
codesign --verify --deep --strict --verbose=2 target/universal/release/$BINARY_NAME.app/Contents/MacOS/$BINARY_NAME



# Define variables for paths
APP_PATH="target/universal/release/$BINARY_NAME.app"
ZIP_NAME="$BINARY_NAME.v$VERSION.zip"
# ZIP_PATH="/Users/tfarrell/Library/CloudStorage/GoogleDrive-tim@farrellsound.com/Shared drives/PUBLIC/$BINARY_NAME/$BINARY_NAME.v$VERSION.zip"
# WEB_ZIP_PATH="/Users/tfarrell/Documents/Website/smdbc.com/private/$BINARY_NAME.v$VERSION.zip"
# GDRIVE_VERSION_FILE="/Users/tfarrell/Library/CloudStorage/GoogleDrive-tim@farrellsound.com/Shared drives/PUBLIC/$BINARY_NAME/latest_ver"
# WEB_VERSION_FILE="/Users/tfarrell/Documents/Website/smdbc.com/private/latest_ver"

# Remove old files
/bin/rm -rf /Users/tfarrell/Library/CloudStorage/GoogleDrive-tim@farrellsound.com/Shared\ drives/PUBLIC/$BINARY_NAME/$BINARY_NAME*
/bin/rm -rf /Users/tfarrell/Documents/Website/smdbc.com/private/$BINARY_NAME*

# Create a temporary directory to hold just the .app bundle
TEMP_DIR=$(mktemp -d)

# # Copy only the .app bundle to the temporary directory
# cp -R "$APP_PATH" "$TEMP_DIR/"
# cp -R "$APP_PATH" /Applications

# Navigate to the temporary directory
cd "$TEMP_DIR"

# Create the zip file with only the .app bundle at the root
echo "Creating zip file for notarization..."
zip -r "$ZIP_NAME" "$$APP_PATH"


# Write the version number to latest_ver
echo "$VERSION" > "$GDRIVE_VERSION_FILE"
echo "$VERSION" > "$WEB_VERSION_FILE"

# Clean up
rm -rf "$TEMP_DIR"