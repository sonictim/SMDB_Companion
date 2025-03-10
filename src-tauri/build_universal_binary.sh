#!/bin/bash

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


# Build for both architectures
# cargo tauri build --target universal-apple-darwin
