#!/bin/sh
## bash script to build UNIVERSAL BINARY for MAC of SMDB_Companion


# Set the binary name
BINARY_NAME="SMDB_Companion"

# Add the necessary targets
rustup target add aarch64-apple-darwin x86_64-apple-darwin

# Build for both architectures
cargo bundle --release --target aarch64-apple-darwin
cargo bundle --release --target x86_64-apple-darwin

cp -R target/x86_64-apple-darwin/release/bundle/osx/$BINARY_NAME.app target/universal/release/

# Create the universal binary
lipo -create -output target/universal/release/$BINARY_NAME.app/Contents/MacOS/$BINARY_NAME target/aarch64-apple-darwin/release/$BINARY_NAME target/x86_64-apple-darwin/release/$BINARY_NAME

# Verify the binary
file target/universal/release/$BINARY_NAME.app/Contents/MacOS/$BINARY_NAME

cp -R target/x86_64-apple-darwin/release/bundle/osx/$BINARY_NAME.app /Users/tfarrell/Library/CloudStorage/GoogleDrive-tim@farrellsound.com/Shared\ drives/PUBLIC/SMDB_Companion/
#mv $BINARY_NAME Mac\ Universal\ Binary/$BINARY_NAME
