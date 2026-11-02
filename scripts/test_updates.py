import base64
import importlib.util
from pathlib import Path
import unittest
import xml.etree.ElementTree as ET


def load(name):
    spec = importlib.util.spec_from_file_location(name, Path(__file__).with_name(name + ".py"))
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


configure = load("configure-updates").configure
appcast = load("make-appcast")


class UpdateTests(unittest.TestCase):
    def test_development_build_removes_distribution_configuration(self):
        info = configure({}, {"ROVE_UPDATES_ENABLED": "1", "ROVE_SPARKLE_PUBLIC_KEY": base64.b64encode(b"k" * 32).decode()})
        self.assertTrue(info["SUVerifyUpdateBeforeExtraction"])
        self.assertFalse(info["SUAutomaticallyUpdate"])
        self.assertFalse(info["SUEnableSystemProfiling"])
        self.assertEqual(configure(info, {}), {})
        with self.assertRaises(ValueError):
            configure({}, {"ROVE_UPDATES_ENABLED": "1"})

    def test_feed_escapes_notes_and_preserves_existing_releases(self):
        info = {"CFBundleVersion": "3", "CFBundleShortVersionString": "0.1.2", "LSMinimumSystemVersion": "14.0"}
        signature = base64.b64encode(b"s" * 64).decode()
        item = appcast.feed_item(info, "Rove-0.1.2.dmg", 1234, signature, "A & B <notes>")
        feed = ET.fromstring('<rss version="2.0"><channel><title>Rove</title></channel></rss>')
        data = appcast.append_release(feed, item)
        parsed = ET.fromstring(data)
        self.assertEqual(parsed.findtext("channel/item/description"), "A & B <notes>")
        enclosure = parsed.find("channel/item/enclosure")
        self.assertEqual(enclosure.attrib["url"], "https://github.com/getcortexlabs/rove/releases/download/v0.1.2/Rove-0.1.2.dmg")
        self.assertEqual(enclosure.attrib[f"{{{appcast.SPARKLE}}}edSignature"], signature)
        with self.assertRaisesRegex(ValueError, "increase"):
            appcast.append_release(parsed, item)
        info["CFBundleVersion"] = "4"
        newer = appcast.feed_item(info, "Rove-0.1.2.dmg", 1234, signature, "Next build")
        self.assertEqual(len(ET.fromstring(appcast.append_release(parsed, newer)).findall("channel/item")), 2)

    def test_feed_rejects_version_mismatch_and_invalid_signature(self):
        info = {"CFBundleVersion": "3", "CFBundleShortVersionString": "0.1.2", "LSMinimumSystemVersion": "14.0"}
        signature = base64.b64encode(b"s" * 64).decode()
        with self.assertRaises(ValueError):
            appcast.feed_item(info, "Rove-0.1.1.dmg", 123, signature, "")
        with self.assertRaises(ValueError):
            appcast.feed_item(info, "Rove-0.1.2.dmg", 123, "", "")


if __name__ == "__main__":
    unittest.main()
