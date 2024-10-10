#!/bin/sh
## bash script to build UNIVERSAL BINARY for MAC of SMDB_Companion


# Set the binary name
BINARY_NAME="SMDB_Companion"
VERSION=$(awk '/\[package\]/ {flag=1} flag && /^version =/ {print $3; exit}' Cargo.toml | tr -d '"')

export MACOSX_DEPLOYMENT_TARGET=12.0

echo Building and Bundling Version: $VERSION

# Add the necessary targets
#rustup target add aarch64-apple-darwin x86_64-apple-darwin
#cargo install cargo-bundle

# Build for both architectures
cargo bundle --release --target aarch64-apple-darwin
cargo bundle --release --target x86_64-apple-darwin

#build a directory for universal builds
mkdir -p target/universal/release
cp -R target/x86_64-apple-darwin/release/bundle/osx/$BINARY_NAME.app target/universal/release/

# Create the universal binary
lipo -create -output target/universal/release/$BINARY_NAME.app/Contents/MacOS/$BINARY_NAME target/aarch64-apple-darwin/release/$BINARY_NAME target/x86_64-apple-darwin/release/$BINARY_NAME

# Verify the binary
file target/universal/release/$BINARY_NAME.app/Contents/MacOS/$BINARY_NAME




# Define variables for paths
APP_PATH="target/universal/release/$BINARY_NAME.app"
ZIP_PATH="/Users/tfarrell/Library/CloudStorage/GoogleDrive-tim@farrellsound.com/Shared drives/PUBLIC/$BINARY_NAME/$BINARY_NAME.v$VERSION.zip"

/bin/rm -rf /Users/tfarrell/Library/CloudStorage/GoogleDrive-tim@farrellsound.com/Shared\ drives/PUBLIC/$BINARY_NAME/$BINARY_NAME*

# Create a temporary directory to hold just the .app bundle
TEMP_DIR=$(mktemp -d)

# Copy only the .app bundle to the temporary directory
cp -R "$APP_PATH" "$TEMP_DIR/"

cp -R "$APP_PATH" /Applications

# Navigate to the temporary directory
cd "$TEMP_DIR"

# Create the zip file with only the .app bundle at the root
zip -r "$ZIP_PATH" "$(basename "$APP_PATH")"

# Clean up
rm -rf "$TEMP_DIR"



# cd target/universal/release
# zip -r /Users/tfarrell/Library/CloudStorage/GoogleDrive-tim@farrellsound.com/Shared\ drives/PUBLIC/SMDB_Companion/SMDB_Companion_universal.zip SMDB_Companion.app 

#cp -R target/universal/release/$BINARY_NAME.app /Users/tfarrell/Library/CloudStorage/GoogleDrive-tim@farrellsound.com/Shared\ drives/PUBLIC/SMDB_Companion/
#chmod +x /Users/tfarrell/Library/CloudStorage/GoogleDrive-tim@farrellsound.com/Shared\ drives/PUBLIC/SMDB_Companion/SMDB_Companion.app/Contents/MacOS/SMDB_Companion
#cp -R target/x86_64-apple-darwin/release/bundle/osx/$BINARY_NAME.app /Users/tfarrell/Library/CloudStorage/GoogleDrive-tim@farrellsound.com/Shared\ drives/PUBLIC/SMDB_Companion/Intel_Mac/
#cp -R target/aarch64-apple-darwin/release/bundle/osx/$BINARY_NAME.app /Users/tfarrell/Library/CloudStorage/GoogleDrive-tim@farrellsound.com/Shared\ drives/PUBLIC/SMDB_Companion/Apple_Silicon/


