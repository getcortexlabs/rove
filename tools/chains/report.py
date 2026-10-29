#!/usr/bin/env python3
"""Publish timings, diagnostic validation and sample identity from local runs."""
import argparse
import hashlib
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]


def write(path, value):
    path.write_text(json.dumps(value, indent=2) + "\n")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--timings", type=Path, required=True)
    parser.add_argument("--validation", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    args.output.mkdir(parents=True, exist_ok=True)
    report = json.loads(args.timings.read_text())
    identities = {}
    quality = []
    checked = 0
    for row in report["rows"]:
        path = Path(row["command"][row["command"].index("--output") + 1]).resolve()
        count = row["chains"]
        for index in range(count):
            chain = path / f"chain-{index+1:02}" if count > 1 else path
            record = json.loads((chain / "run.json").read_text())
            assert record["executable_sha256"] == report["engine_sha256"]
            assert record["input"]["sha256"] == report["matrix_sha256"]
            assert record["status"] == "completed"
            assert record["completed_steps"] == report["steps_per_chain"]
            seed = str(record["settings"]["seed"])
            hashes = {}
            for name in ["trace.log", "trees.nexus"]:
                digest = hashlib.sha256()
                with (chain / name).open("rb") as stream:
                    for line in stream:
                        if line.startswith(b"# ") or line.startswith(b"[command "):
                            continue
                        digest.update(line)
                hashes[name] = digest.hexdigest()
            if seed in identities:
                assert hashes == identities[seed]["scientific_sha256"], chain
            else:
                identities[seed] = {"scientific_sha256": hashes,
                    "first_observed_at": str(chain.relative_to(ROOT))}
            checked += 1
        if count > 1 and row["repeat"] == 1:
            summary = json.loads((path / "comparison.json").read_text())
            quality.append({"chains": count, "retained_per_chain": summary["retained_per_chain"],
                "statistics": summary["statistics"],
                "max_unconstrained_clade_difference": summary["max_unconstrained_clade_difference"],
                "topology_convergence_assessed": False})
        row["command"] = [arg.replace(str(ROOT), "$REPO") for arg in row["command"]]
    validation = json.loads(args.validation.read_text())
    assert validation["engine_sha256"] == report["engine_sha256"]
    validation["revision"] = report["revision"]
    write(args.output / "throughput.json", report)
    write(args.output / "identity.json", {"checked_chain_outputs": checked,
        "method": "Compare trace and tree scientific content by seed across every timed run; omit trace provenance comments and tree command comments. Other tree metadata, taxon mappings and every sample remain part of the hash.",
        "seeds": identities})
    write(args.output / "sampling-quality.json", quality)
    write(args.output / "archive-validation.json", validation)
    print(f"Verified {checked} chain outputs, {len(identities)} distinct seeds")
    print(f"Engine: {report['engine_sha256']}")
    for item in report["summary"]:
        values = [row["wall_seconds"] for row in report["rows"] if row["chains"] == item["chains"]]
        print(f"| {item['chains']} | {item['median_wall_seconds']:.3f} | {min(values):.3f}–{max(values):.3f} | {item['median_sampling_seconds']:.3f} | {item['aggregate_speedup']:.2f}× |")


if __name__ == "__main__":
    main()
