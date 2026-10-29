#!/bin/bash
set -euo pipefail

repo_root="$(cd "$(dirname "$0")/.." && pwd)"
configuration="${1:-debug}"
if [[ "$configuration" != debug && "$configuration" != release ]]; then
    echo "usage: scripts/build-macos.sh [debug|release]" >&2
    exit 2
fi
cd "$repo_root"

cargo build --locked --release --bin rove
export CLANG_MODULE_CACHE_PATH="$repo_root/apps/macos/.build/ModuleCache"
export SWIFTPM_MODULECACHE_OVERRIDE="$CLANG_MODULE_CACHE_PATH"
swift_options=(--package-path apps/macos --configuration "$configuration"
    --cache-path "$repo_root/apps/macos/.build/cache"
    --config-path "$repo_root/apps/macos/.build/config"
    --security-path "$repo_root/apps/macos/.build/security")
if [[ "${ROVE_NESTED_BUILD_SANDBOX:-0}" == 1 ]]; then
    swift_options+=(--disable-sandbox)
    # dsymutil needs services unavailable in some nested build environments.
    if [[ "$configuration" == release ]]; then
        swift_options+=(-debug-info-format none)
    fi
fi
swift build "${swift_options[@]}"
swift_bin="$(swift build "${swift_options[@]}" --show-bin-path)"
app="$repo_root/build/macos/Rove.app"
mkdir -p "$app/Contents/MacOS" "$app/Contents/Helpers" "$app/Contents/Resources"
cp "$swift_bin/Rove" "$app/Contents/MacOS/Rove"
cp target/release/rove "$app/Contents/Helpers/rove"
rm -rf "$app/Contents/Resources/Rove_RoveApp.bundle"
cp -R "$swift_bin/Rove_RoveApp.bundle" "$app/Contents/Resources/"
cp apps/macos/Info.plist "$app/Contents/Info.plist"

iconset="$repo_root/build/macos/Rove.iconset"
mkdir -p "$iconset"
for size in 16 32 128 256 512; do
    sips -z "$size" "$size" docs/public/rove-logo.png --out "$iconset/icon_${size}x${size}.png" >/dev/null
    double=$((size * 2))
    sips -z "$double" "$double" docs/public/rove-logo.png --out "$iconset/icon_${size}x${size}@2x.png" >/dev/null
done
python3 - "$iconset" "$app/Contents/Resources/Rove.icns" <<'PY'
import struct
import sys
from pathlib import Path

source, destination = map(Path, sys.argv[1:])
representations = {
    b'icp4': 'icon_16x16.png', b'ic11': 'icon_16x16@2x.png',
    b'icp5': 'icon_32x32.png', b'ic12': 'icon_32x32@2x.png',
    b'ic07': 'icon_128x128.png', b'ic13': 'icon_128x128@2x.png',
    b'ic08': 'icon_256x256.png', b'ic14': 'icon_256x256@2x.png',
    b'ic09': 'icon_512x512.png', b'ic10': 'icon_512x512@2x.png',
}
chunks = []
for kind, filename in representations.items():
    png = (source / filename).read_bytes()
    chunks.append(kind + struct.pack('>I', len(png) + 8) + png)
payload = b''.join(chunks)
destination.write_bytes(b'icns' + struct.pack('>I', len(payload) + 8) + payload)
PY
codesign --force --sign - "$app/Contents/Helpers/rove"
codesign --force --sign - "$app"
echo "Built $app"
