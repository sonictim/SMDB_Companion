#!/bin/sh
## bash script to build UNIVERSAL BINARY for MAC of SMDB_Companion

set -e  # Exit on any error

# Set the binary name
BINARY_NAME="SMDB_Companion"
VERSION=$(awk '/\[package\]/ {flag=1} flag && /^version =/ {print $3; exit}' Cargo.toml | tr -d '"')
SOURCE_BINARY_PATH="/Users/tfarrell/Documents/CODE/SMDB_Companion/target/universal/release"
APP_PATH="target/universal/release/$BINARY_NAME.app"
ZIP_NAME="$BINARY_NAME.v$VERSION.zip"
DMG_NAME="$BINARY_NAME.v$VERSION.dmg"
DMG_PATH="/Users/tfarrell/Documents/Website/smdbc.com/private/$DMG_NAME"
GDRIVE_VERSION_FILE="/Users/tfarrell/Library/CloudStorage/GoogleDrive-tim@farrellsound.com/Shared drives/PUBLIC/$BINARY_NAME/latest_ver"
WEB_VERSION_FILE="/Users/tfarrell/Documents/Website/smdbc.com/private/latest_ver"
CODESIGN_CERTIFICATE_ID="C8DA3674F1466B4F2D8EB196D23DCE9B5A44F1D9"
NOTARIZE_USERNAME="soundguru@gmail.com"
NOTARIZE_PASSWORD="ndtq-xhsn-wxyl-lzji"
NOTARIZE_TEAM_ID="22D9VBGAWF"

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
echo "Code signing for ARM64..."
codesign --sign $CODESIGN_CERTIFICATE_ID \
    --deep --force --options runtime \
    --entitlements "entitlements.plist" \
    "target/aarch64-apple-darwin/release/bundle/osx/$BINARY_NAME.app"
echo "Verifying code signature..."
codesign --verify --deep --strict --verbose=2 "target/aarch64-apple-darwin/release/bundle/osx/$BINARY_NAME.app"

echo "Building for x86_64..."
cargo bundle --release --target x86_64-apple-darwin || { echo "x86_64 build failed"; exit 1; }
echo "Code signing for x86_64..."
codesign --sign $CODESIGN_CERTIFICATE_ID \
    --deep --force --options runtime \
    --entitlements "entitlements.plist" \
    "target/x86_64-apple-darwin/release/bundle/osx/$BINARY_NAME.app"
echo "Verifying code signature..."
codesign --verify --deep --strict --verbose=2 "target/x86_64-apple-darwin/release/bundle/osx/$BINARY_NAME.app"

# Build a directory for universal builds
# mkdir -p target/universal/release
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
codesign --sign $CODESIGN_CERTIFICATE_ID \
    --deep --force --options runtime \
    --entitlements "entitlements.plist" \
    "$APP_PATH"

# Verify code signing
echo "Verifying code signature..."
codesign --verify --deep --strict --verbose=2 "$APP_PATH"

# # Create DMG
# echo "Creating DMG..."
# hdiutil create -volname "$BINARY_NAME" -srcfolder "$SOURCE_BINARY_PATH" -ov -format UDZO "$DMG_PATH"


# # Prompt the user
# echo "Continue to notarization? (y/n)"
# read -p "Enter your choice: " choice

# # Convert to lowercase to handle both uppercase and lowercase input
# choice=$(echo "$choice" | tr '[:upper:]' '[:lower:]')

# # Check if the user answered 'y' or 'yes'
# if [[ "$choice" == "y" || "$choice" == "yes" ]]; then
#     echo "Continuing..."
#     # Put the rest of your script here that should execute if the user chooses 'yes'
# else
#     echo "Exiting..."
#     exit 0  # Exit the script
# fi



# Before creating temp directory
ORIGINAL_DIR=$(pwd)

# Create temporary directory for notarization
TEMP_DIR=$(mktemp -d)
# Create the zip file
echo "Creating zip file for notarization..."
ditto -c -k --keepParent "$APP_PATH" "$TEMP_DIR/$ZIP_NAME"

cd "$TEMP_DIR"


# Get bundle identifier
# BUNDLE_IDENTIFIER=$(defaults read "$APP_PATH/Contents/Info.plist" CFBundleIdentifier)
BUNDLE_IDENTIFIER="com.SMDB_Companion"
echo "Bundle identifier: $BUNDLE_IDENTIFIER"

# Submit for notarization
echo "Submitting for notarization..."
NOTARIZE_RESPONSE=$(xcrun notarytool submit "$ZIP_NAME" --wait --apple-id "$NOTARIZE_USERNAME" --password "$NOTARIZE_PASSWORD" --team-id "$NOTARIZE_TEAM_ID" | tee /dev/tty)

# Send a text message using osascript
RECIPIENT="2133615559"  # Replace with phone number or iMessage contact name
MESSAGE="Notariation of SMDB_Companion v$VERSION has completed. Status: $NOTARIZE_RESPONSE"

osascript <<EOF
tell application "Messages"
    set targetService to 1st service whose service type = iMessage
    set targetBuddy to buddy "$RECIPIENT" of targetService
    send "$MESSAGE" to targetBuddy
end tell
EOF


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


# Create DMG
echo "Creating Website DMG..."
hdiutil create -volname "$BINARY_NAME" -srcfolder "$SOURCE_BINARY_PATH" -ov -format UDZO "$DMG_PATH"
hdiutil internet-enable -yes "$DMG_PATH"

# # Create final distribution zip
# ditto -c -k --keepParent "$APP_PATH" "$ZIP_NAME"

# # Copy to destination paths
# echo "Copying to distribution locations..."
# for DIR in "/Users/tfarrell/Documents/Website/smdbc.com/private" \
#           "/Users/tfarrell/Library/CloudStorage/GoogleDrive-tim@farrellsound.com/Shared drives/PUBLIC/$BINARY_NAME"; do
#     rm -f "$DIR/$BINARY_NAME"*
#     cp "$ZIP_NAME" "$DIR/$ZIP_NAME"
# done

# Update version files
echo "$VERSION" | tee "$GDRIVE_VERSION_FILE" "$WEB_VERSION_FILE"

echo "Build and notarization process completed successfully!"