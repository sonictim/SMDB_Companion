#!/bin/bash

# 1. Extract version from tauri.conf.json
if command -v jq &> /dev/null; then
  # Using jq if available
  VERSION=$(jq -r '.version' ./src-tauri/tauri.conf.json)
else
  # Fallback to Python (pre-installed on macOS)
  VERSION=$(python3 -c "import json; print(json.load(open('./src-tauri/tauri.conf.json'))['version'])")
fi

echo "Found version: $VERSION in tauri.conf.json"

# 2. Update version in Cargo.toml - using a specific line number approach to only update the main package version
if [[ "$OSTYPE" == "darwin"* ]]; then
  # macOS version of sed - only replacing version in line 3 (the main package version)
  sed -i '' "3s/version = \"[0-9]*\.[0-9]*\.[0-9]*\"/version = \"$VERSION\"/" ./src-tauri/Cargo.toml
else
  # Linux/other version of sed
  sed -i "3s/version = \"[0-9]*\.[0-9]*\.[0-9]*\"/version = \"$VERSION\"/" ./src-tauri/Cargo.toml
fi

echo "Updated Cargo.toml with version $VERSION"

