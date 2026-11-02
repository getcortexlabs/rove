"""Configure a distribution bundle; ordinary local builds never poll the feed."""
import base64
import os
from pathlib import Path
import plistlib
import sys


def configure(info, environment):
    info = dict(info)
    for key in ("RoveUpdatesEnabled", "SUFeedURL", "SUPublicEDKey", "SUEnableAutomaticChecks",
                "SUAutomaticallyUpdate", "SUEnableSystemProfiling", "SUVerifyUpdateBeforeExtraction"):
        info.pop(key, None)
    if environment.get("ROVE_UPDATES_ENABLED") != "1":
        return info
    public_key = environment.get("ROVE_SPARKLE_PUBLIC_KEY", "")
    if len(base64.b64decode(public_key, validate=True)) != 32:
        raise ValueError("ROVE_SPARKLE_PUBLIC_KEY must contain a 32-byte Ed25519 public key")
    info.update({
        "RoveUpdatesEnabled": True,
        "SUFeedURL": "https://rove.getcortexlabs.com/updates/appcast.xml",
        "SUPublicEDKey": public_key,
        "SUEnableAutomaticChecks": True,
        "SUAutomaticallyUpdate": False,
        "SUEnableSystemProfiling": False,
        "SUVerifyUpdateBeforeExtraction": True,
    })
    return info


if __name__ == "__main__":
    path = Path(sys.argv[1])
    info = configure(plistlib.loads(path.read_bytes()), os.environ)
    path.write_bytes(plistlib.dumps(info))
