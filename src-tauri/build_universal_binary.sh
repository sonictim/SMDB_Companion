#!/bin/bash
APPLE_CERTIFICATE="CD96C81E43F0FFA026939DC37BF69875A96FEF81"
APPLE_ID="soundguru@gmail.com"
APPLE_PASSWORD="ndtq-xhsn-wxyl-lzji"
APPLE_TEAM_ID="22D9VBGAWF"


export APPLE_CERTIFICATE
export APPLE_ID
export APPLE_PASSWORD
export APPLE_TEAM_ID

# 1. Extract version from tauri.conf.json
if command -v jq &> /dev/null; then
  # Using jq if available
  VERSION=$(jq -r '.version' ./tauri.conf.json)
else
  # Fallback to Python (pre-installed on macOS)
  VERSION=$(python3 -c "import json; print(json.load(open('./tauri.conf.json'))['version'])")
fi

echo "Found version: $VERSION in tauri.conf.json"

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


# Build for both architectures
cargo tauri build --target universal-apple-darwin
