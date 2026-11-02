"""Catch CLI additions that have not been assigned a native control or wiring."""
import json
from pathlib import Path
import re

ROOT = Path(__file__).resolve().parent.parent
mapping = json.loads((ROOT / "tools/native/cli-parity.json").read_text())
flags = set()
for source in (ROOT / "src/bin/rove").glob("*.rs"):
    code = source.read_text().split("#[cfg(test)]")[0]
    flags.update(re.findall(r'"(--[a-z][a-z0-9-]+)"', code))
documented = set(mapping["flags"])
assert flags == documented, f"Update native parity: missing={flags - documented}, stale={documented - flags}"
main = (ROOT / "src/bin/rove/main.rs").read_text()
commands = set(re.findall(r'a == "([a-z][a-z-]*)"', main))
commands.update(re.findall(r'Some\("([a-z-]+)"\) => Command::', main))
assert commands == set(mapping["commands"]), "Update the native command map"
for path in [*mapping["commands"].values(), *(value["native"] for value in mapping["flags"].values())]:
    assert (ROOT / path).is_file(), f"Missing native implementation: {path}"
print(f"Native map covers {len(commands)} commands and {len(flags)} options. Behavioral tests still apply.")
