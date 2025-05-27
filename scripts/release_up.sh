#!/bin/bash

# Get the path to the tauri.conf.json file
TAURI_CONF_PATH="./src-tauri/tauri.conf.json"

# Check if the file exists
if [ ! -f "$TAURI_CONF_PATH" ]; then
  echo "Error: $TAURI_CONF_PATH does not exist!"
  exit 1
fi

# Extract current version
if command -v jq &> /dev/null; then
  # Using jq if available
  CURRENT_VERSION=$(jq -r '.version' "$TAURI_CONF_PATH")
else
  # Fallback to Python (pre-installed on macOS)
  CURRENT_VERSION=$(python3 -c "import json; print(json.load(open('$TAURI_CONF_PATH'))['version'])")
fi

echo "Current version: $CURRENT_VERSION"

# Split version into components
IFS='.' read -r MAJOR MINOR PATCH <<< "$CURRENT_VERSION"

# Increment patch version
NEW_MINOR=$((MINOR + 1))
NEW_PATCH="0"
NEW_VERSION="$MAJOR.$NEW_MINOR.$NEW_PATCH"

echo "New version: $NEW_VERSION"

# Update tauri.conf.json
if command -v jq &> /dev/null; then
  # Using jq to update the version (preserves formatting)
  TEMP_FILE=$(mktemp)
  jq --arg version "$NEW_VERSION" '.version = $version' "$TAURI_CONF_PATH" > "$TEMP_FILE"
  mv "$TEMP_FILE" "$TAURI_CONF_PATH"
else
  # Fallback with sed (macOS compatible)
  if [[ "$OSTYPE" == "darwin"* ]]; then
    # macOS version of sed
    sed -i '' "s/\"version\": \"$CURRENT_VERSION\"/\"version\": \"$NEW_VERSION\"/" "$TAURI_CONF_PATH"
  else
    # Linux/other version of sed
    sed -i "s/\"version\": \"$CURRENT_VERSION\"/\"version\": \"$NEW_VERSION\"/" "$TAURI_CONF_PATH"
  fi
fi

# Now run the update_cargo.toml.sh script to sync the Cargo.toml version
if [ -f "./scripts/update_cargo.toml.sh" ]; then
  echo "Running update_cargo.toml.sh to sync Cargo.toml version..."
  bash ./scripts/update_cargo.toml.sh
else
  echo "Warning: update_cargo.toml.sh not found. Cargo.toml will not be updated."
fi

echo "Version updated successfully from $CURRENT_VERSION to $NEW_VERSION"