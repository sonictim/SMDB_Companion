#!/bin/bash

NOTARIZE_USERNAME="soundguru@gmail.com"
NOTARIZE_PASSWORD="ndtq-xhsn-wxyl-lzji"
NOTARIZE_TEAM_ID="22D9VBGAWF"

xcrun notarytool log "$1" --apple-id "$NOTARIZE_USERNAME" --password "$NOTARIZE_PASSWORD" --team-id "$NOTARIZE_TEAM_ID" 