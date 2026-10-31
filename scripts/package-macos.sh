#!/bin/bash
# Builds the release app, signs it, notarizes it, wraps it in a DMG and
# notarizes the image too, so the app ships with its ticket stapled.
#
# Without environment variables the result is an unsigned image around the
# ad-hoc signed app, for local use. For a release set:
#   CODESIGN_ID     a "Developer ID Application" identity name or SHA-1 hash
#   NOTARY_PROFILE  a notarytool keychain profile (notarytool store-credentials)
set -euo pipefail

repo_root="$(cd "$(dirname "$0")/.." && pwd)"
cd "$repo_root"

if [[ -n "${NOTARY_PROFILE:-}" && -z "${CODESIGN_ID:-}" ]]; then
    echo "NOTARY_PROFILE needs CODESIGN_ID: Apple rejects ad-hoc signatures" >&2
    exit 2
fi

app="$repo_root/build/macos/Rove.app"
version="$(/usr/libexec/PlistBuddy -c 'Print :CFBundleShortVersionString' apps/macos/Info.plist)"
dmg="$repo_root/build/macos/Rove-$version.dmg"
identifier="$(/usr/libexec/PlistBuddy -c 'Print :CFBundleIdentifier' apps/macos/Info.plist)"

# hdiutil and dmgbuild work through a helper that lacks the terminal's access
# to ~/Documents, so everything is assembled under /tmp and moved into build/
# only once it is complete.
staging="$(mktemp -d /tmp/rove-dmg.XXXXXX)"
trap 'rm -rf "$staging"' EXIT

# Submits one file and stops with the notary log when it is not accepted.
notarize() {
    local file="$1" result="$staging/submission.plist" id status
    xcrun notarytool submit "$file" --keychain-profile "$NOTARY_PROFILE" --wait \
        --output-format plist > "$result"
    id="$(/usr/libexec/PlistBuddy -c 'Print :id' "$result")"
    status="$(/usr/libexec/PlistBuddy -c 'Print :status' "$result")"
    echo "Notarization of $(basename "$file"): $status ($id)"
    if [[ "$status" != Accepted ]]; then
        xcrun notarytool log "$id" --keychain-profile "$NOTARY_PROFILE" >&2
        exit 1
    fi
}

scripts/build-macos.sh release

if [[ -n "${CODESIGN_ID:-}" ]]; then
    echo "Signing with $CODESIGN_ID"
    # Nested code first, then the bundle that seals it. --deep is deprecated
    # for signing and would not apply the hardened runtime to the helper.
    sign=(codesign --force --options runtime --timestamp --sign "$CODESIGN_ID")
    "${sign[@]}" "$app/Contents/Helpers/rove"
    "${sign[@]}" "$app/Contents/Resources/Rove_RoveApp.bundle"
    "${sign[@]}" "$app"
    codesign --verify --deep --strict --verbose=2 "$app"
fi

if [[ -n "${NOTARY_PROFILE:-}" ]]; then
    ditto -c -k --keepParent "$app" "$staging/Rove.zip"
    notarize "$staging/Rove.zip"
    xcrun stapler staple "$app"
fi

# The volume name carries the version so each release mounts at a new path.
root="$staging/root"
mkdir -p "$root"
cp -R "$app" "$root/"
image="$staging/Rove-$version.dmg"
if command -v uvx >/dev/null; then
    uvx --from "dmgbuild==1.6.7" dmgbuild -s scripts/dmg-settings.py \
        -D app="$root/Rove.app" -D icon="$app/Contents/Resources/Rove.icns" \
        "Rove $version" "$image" >/dev/null
else
    ln -s /Applications "$root/Applications"
    hdiutil create -volname "Rove $version" -srcfolder "$root" -ov -format UDZO \
        "$image" >/dev/null
fi

if [[ -n "${CODESIGN_ID:-}" ]]; then
    codesign --force --timestamp --identifier "$identifier.dmg" --sign "$CODESIGN_ID" "$image"
fi

if [[ -n "${NOTARY_PROFILE:-}" ]]; then
    notarize "$image"
    xcrun stapler staple "$image"
    xcrun stapler validate "$image"
    spctl --assess --type open --context context:primary-signature --verbose=2 "$image"
    # Check the app as it ships, not the copy in build/.
    mount="$staging/mount"
    hdiutil attach -nobrowse -readonly -mountpoint "$mount" "$image" >/dev/null
    xcrun stapler validate "$mount/Rove.app"
    spctl --assess --type execute --verbose=2 "$mount/Rove.app"
    hdiutil detach "$mount" -quiet
fi

rm -f "$dmg"
mv "$image" "$dmg"
shasum -a 256 "$dmg"
echo "Built $dmg"
