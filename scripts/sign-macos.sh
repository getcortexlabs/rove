#!/bin/bash
set -euo pipefail
app="${1:?usage: sign-macos.sh app identity}"
identity="${2:?usage: sign-macos.sh app identity}"
sign=(codesign --force --sign "$identity")
if [[ "$identity" != - ]]; then sign+=(--options runtime --timestamp); fi
framework="$app/Contents/Frameworks/Sparkle.framework"
# Preserve the framework symlinks and sign nested code from the inside out.
for component in Autoupdate Updater.app XPCServices/Downloader.xpc XPCServices/Installer.xpc; do
    "${sign[@]}" "$framework/Versions/B/$component"
done
"${sign[@]}" "$framework"
"${sign[@]}" "$app/Contents/Helpers/rove"
resources="$app/Contents/Resources/Rove_RoveApp.bundle"
# Xcode creates a macOS bundle; SwiftPM can create a plain resource directory.
# The latter is sealed by the enclosing app's signature.
if [[ -f "$resources/Contents/Info.plist" ]]; then
    "${sign[@]}" "$resources"
fi
"${sign[@]}" "$app"
