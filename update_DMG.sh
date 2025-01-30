#!/bin/bash
echo "Updating DMG..."
BINARY_NAME="SMDB_Companion"
VERSION=$(awk '/\[package\]/ {flag=1} flag && /^version =/ {print $3; exit}' Cargo.toml | tr -d '"')
SOURCE_BINARY_PATH="/Users/tfarrell/Documents/CODE/SMDB_Companion/target/universal/release"
SOURCE_DMG="/Users/tfarrell/Documents/CODE/SMDB_Companion/assets/SMDBC.dmg"
TEMP_DMG="/Users/tfarrell/Documents/CODE/SMDB_Companion/assets/SMDBCt.dmg"
WEBSITE_PATH="/Users/tfarrell/Documents/Website/smdbc.com/private"
APP_PATH="$SOURCE_BINARY_PATH/$BINARY_NAME.app"
ZIP_NAME="$BINARY_NAME.v$VERSION.zip"
DMG_NAME="$BINARY_NAME.v$VERSION.dmg"
DMG_PATH="$WEBSITE_PATH/$DMG_NAME"
GDRIVE_VERSION_FILE="/Users/tfarrell/Library/CloudStorage/GoogleDrive-tim@farrellsound.com/Shared drives/PUBLIC/$BINARY_NAME/latest_ver"
WEB_VERSION_FILE="$WEBSITE_PATH/latest_ver"
CODESIGN_CERTIFICATE_ID="CD96C81E43F0FFA026939DC37BF69875A96FEF81"
NOTARIZE_USERNAME="soundguru@gmail.com"
NOTARIZE_PASSWORD="ndtq-xhsn-wxyl-lzji"
NOTARIZE_TEAM_ID="22D9VBGAWF"



cp /Users/tfarrell/Documents/CODE/SMDB_Companion/assets/DMG\ Source/DS_Store $SOURCE_BINARY_PATH/.DS_Store
cp /Users/tfarrell/Documents/CODE/SMDB_Companion/assets/DMG\ Source/background.png $SOURCE_BINARY_PATH/.background/background.png

hdiutil create -volname "SMDB Companion" -srcfolder $SOURCE_BINARY_PATH -ov -format UDBZ -o $DMG_PATH




# # rm $WEBSITE_PATH/$BINARY_NAME*

# echo "Creating Temp DMG..."
# hdiutil convert $SOURCE_DMG  -format UDRW -ov -o $TEMP_DMG


# echo "Mounting Temp DMG..."
# hdiutil attach $TEMP_DMG

# echo "Copying SMDB Companion to Mounted DMG..."
# cp -R $APP_PATH /Volumes/SMDB\ Companion/

# echo "UnMounting Temp DMG..."
# hdiutil detach /Volumes/SMDB\ Companion/


# echo "Creating Website DMG..."
# hdiutil convert $TEMP_DMG  -format UDZO -ov -o $DMG_PATH



# # Update version files
# echo "$VERSION" | tee "$GDRIVE_VERSION_FILE" "$WEB_VERSION_FILE"