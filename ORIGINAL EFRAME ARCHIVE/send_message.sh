# Send a text message using osascript
RECIPIENT="2133615559"  # Replace with phone number or iMessage contact name
MESSAGE="Hello! This is a message sent from a shell script."

osascript <<EOF
tell application "Messages"
    set targetService to 1st service whose service type = iMessage
    set targetBuddy to buddy "$RECIPIENT" of targetService
    send "$MESSAGE" to targetBuddy
end tell
EOF