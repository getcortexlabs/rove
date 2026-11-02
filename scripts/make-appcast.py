"""Sign a notarized DMG and prepare a feed for review, without publishing it."""
import argparse
import base64
from datetime import datetime, timezone
from email.utils import format_datetime
from pathlib import Path
import plistlib
import subprocess
import tempfile
from urllib.parse import quote
import xml.etree.ElementTree as ET

SPARKLE = "http://www.andymatuschak.org/xml-namespaces/sparkle"
ET.register_namespace("sparkle", SPARKLE)
ROOT = Path(__file__).resolve().parent.parent


def feed_item(info, archive, size, signature, notes):
    version = info["CFBundleShortVersionString"]
    build = info["CFBundleVersion"]
    if not build.isdecimal() or int(build) <= 0:
        raise ValueError("Use a positive integer CFBundleVersion")
    if archive != f"Rove-{version}.dmg":
        raise ValueError("The DMG filename must match the app version")
    if len(base64.b64decode(signature, validate=True)) != 64 or size <= 0:
        raise ValueError("Invalid archive signature or length")
    item = ET.Element("item")
    ET.SubElement(item, "title").text = f"Rove {version}"
    ET.SubElement(item, "pubDate").text = format_datetime(datetime.now(timezone.utc))
    ET.SubElement(item, "link").text = "https://rove.getcortexlabs.com/"
    ET.SubElement(item, f"{{{SPARKLE}}}version").text = build
    ET.SubElement(item, f"{{{SPARKLE}}}shortVersionString").text = version
    ET.SubElement(item, f"{{{SPARKLE}}}minimumSystemVersion").text = info["LSMinimumSystemVersion"]
    ET.SubElement(item, f"{{{SPARKLE}}}hardwareRequirements").text = "arm64"
    ET.SubElement(item, "description", {f"{{{SPARKLE}}}format": "plain-text"}).text = notes
    ET.SubElement(item, "enclosure", {
        "url": f"https://github.com/getcortexlabs/rove/releases/download/{quote('v' + version, safe='')}/{quote(archive, safe='')}",
        "length": str(size), "type": "application/octet-stream",
        f"{{{SPARKLE}}}edSignature": signature,
    })
    return item


def append_release(feed, item):
    channel = feed.find("channel")
    if channel is None:
        raise ValueError("Missing RSS channel")
    build = int(item.findtext(f"{{{SPARKLE}}}version"))
    for previous in channel.findall("item"):
        if int(previous.findtext(f"{{{SPARKLE}}}version")) >= build:
            raise ValueError("CFBundleVersion must increase for every published build")
    channel.insert(0, item)
    ET.indent(feed, space="  ")
    return ET.tostring(feed, encoding="utf-8", xml_declaration=True) + b"\n"


def run(*args):
    return subprocess.run(args, check=True, capture_output=True).stdout


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("dmg", type=Path)
    parser.add_argument("--key-file", type=Path, required=True)
    parser.add_argument("--notes", type=Path, required=True)
    parser.add_argument("--feed", type=Path, default=ROOT / "docs/public/updates/appcast.xml")
    parser.add_argument("--output", type=Path, default=ROOT / "build/macos/appcast.xml")
    args = parser.parse_args()
    dmg = args.dmg.resolve()
    signer = ROOT / "apps/macos/.build/artifacts/sparkle/Sparkle/bin/sign_update"
    run("codesign", "--verify", "--strict", str(dmg))
    run("xcrun", "stapler", "validate", str(dmg))
    run("spctl", "--assess", "--type", "open", "--context", "context:primary-signature", str(dmg))
    with tempfile.TemporaryDirectory(prefix="rove-update-") as mount:
        run("hdiutil", "attach", "-readonly", "-nobrowse", "-mountpoint", mount, str(dmg))
        try:
            app = Path(mount) / "Rove.app"
            run("codesign", "--verify", "--deep", "--strict", str(app))
            run("xcrun", "stapler", "validate", str(app))
            run("spctl", "--assess", "--type", "execute", str(app))
            info = plistlib.loads((app / "Contents/Info.plist").read_bytes())
            if info.get("RoveUpdatesEnabled") is not True or info.get("CFBundleIdentifier") != "com.getcortexlabs.rove.preview":
                raise ValueError("The app is not configured for Rove updates")
            if info.get("SUFeedURL") != "https://rove.getcortexlabs.com/updates/appcast.xml":
                raise ValueError("Unexpected update feed")
        finally:
            run("hdiutil", "detach", mount, "-quiet")
    # Sparkle can print malformed key material in an error. Capture its output
    # and report only the failing program, never its signing error text.
    signature = run(str(signer), "--ed-key-file", str(args.key_file.resolve()), "-p", str(dmg)).decode().strip()
    run("swift", str(ROOT / "scripts/verify-update.swift"), info["SUPublicEDKey"], str(dmg), signature)
    item = feed_item(info, dmg.name, dmg.stat().st_size, signature, args.notes.read_text())
    data = append_release(ET.fromstring(args.feed.read_bytes()), item)
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_bytes(data)
    print(f"Prepared {args.output}. Upload the exact DMG before deploying this feed.")


if __name__ == "__main__":
    main()
