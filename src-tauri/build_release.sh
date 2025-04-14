#!/bin/bash
APPLE_CERTIFICATE="CD96C81E43F0FFA026939DC37BF69875A96FEF81"
APPLE_ID="soundguru@gmail.com"
APPLE_PASSWORD="ndtq-xhsn-wxyl-lzji"
APPLE_TEAM_ID="22D9VBGAWF"

# export DYLD_LIBRARY_PATH=/Users/tfarrell/Documents/CODE/SMDB_Companion/src-tauri/resources/libchromaprint.dylib:$DYLD_LIBRARY_PATH

export APPLE_CERTIFICATE
export APPLE_ID
export APPLE_PASSWORD
export APPLE_TEAM_ID

# Define destination directory
DEST_DIR="/Users/tfarrell/Documents/Website/smdbc.com/private"
VERSION_FILE="$DEST_DIR/latest_ver"

# 1. Extract version from tauri.conf.json
if command -v jq &> /dev/null; then
  # Using jq if available
  VERSION=$(jq -r '.version' ./tauri.conf.json)
else
  # Fallback to Python (pre-installed on macOS)
  VERSION=$(python3 -c "import json; print(json.load(open('./tauri.conf.json'))['version'])")
fi

echo "Found version: $VERSION in tauri.conf.json"
DEST_PATH="$DEST_DIR/SMDB_Companion_v$VERSION.dmg"

# 2. Update version in Cargo.toml
if [[ "$OSTYPE" == "darwin"* ]]; then
  # macOS version of sed
  sed -i '' "s/^version = \".*\"/version = \"$VERSION\"/" ./Cargo.toml
else
  # Linux/other version of sed
  sed -i "s/^version = \".*\"/version = \"$VERSION\"/" ./Cargo.toml
fi

echo "Updated Cargo.toml with version $VERSION"


# Check if required tools are installed
command -v rustup >/dev/null 2>&1 || { echo "rustup is required but not installed. Aborting." >&2; exit 1; }
command -v cargo-bundle >/dev/null 2>&1 || { echo "cargo-bundle is required but not installed. Installing..." >&2; cargo install cargo-bundle; }

# Add the necessary targets
rustup target add aarch64-apple-darwin x86_64-apple-darwin

# Add this check to your build script
check_file() {
  if [ ! -f "$1" ]; then
    echo "ERROR: Required library file not found: $1"
    exit 1
  else
    echo "Found library file: $1"
  fi
}

# Check source files
check_file "./resources/libchromaprint.a"

# Add this to your build script before compiling
echo "Checking library architecture:"
lipo -info ./resources/libchromaprint.a

# Sign FFmpeg binaries
echo "Signing FFmpeg binaries..."

# Make sure binaries are executable
chmod +x ./resources/ffmpeg/arm64/ffmpeg
chmod +x ./resources/ffmpeg/x86_64/ffmpeg

# Sign each binary with your developer certificate
codesign --force --timestamp --options runtime --sign "Developer ID Application: Tim Farrell (${APPLE_TEAM_ID})" ./resources/ffmpeg/arm64/ffmpeg
codesign --force --timestamp --options runtime --sign "Developer ID Application: Tim Farrell (${APPLE_TEAM_ID})" ./resources/ffmpeg/x86_64/ffmpeg

# Verify signatures
echo "Verifying signatures..."
codesign -vvv --deep ./resources/ffmpeg/arm64/ffmpeg
codesign -vvv --deep ./resources/ffmpeg/x86_64/ffmpeg

# cargo clean

cargo tauri build --target universal-apple-darwin





# Create destination directory if it doesn't exist
mkdir -p "$DEST_DIR"

# Find the DMG file in the output directory
DMG_PATH=$(find ./target/universal-apple-darwin/release/bundle/dmg -name "*.dmg" 2>/dev/null)

if [ -z "$DMG_PATH" ]; then
  echo "Error: No DMG file found in the output directory."
  exit 1
fi

# Copy the DMG file to the destination
echo "Copying DMG file to $DEST_DIR..."
cp "$DMG_PATH" "$DEST_PATH"

# # Get just the filename
# DMG_FILENAME=$(basename "$DMG_PATH")

# Check if the copy was successful
if [ $? -eq 0 ]; then
  echo "Successfully copied $DMG_FILENAME to $DEST_PATH"
#   echo "Full path: $DEST_DIR/$DMG_FILENAME"
else
  echo "Failed to copy the DMG file to $DEST_PATH"
  exit 1
fi

# Update version files
echo "$VERSION" | tee  "$VERSION_FILE"

# sudo cp -R  ./target/universal-apple-darwin/release/bundle/macos/SMDB\ Companion.app /Applications/