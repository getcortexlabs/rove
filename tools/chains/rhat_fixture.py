#!/usr/bin/env python3
"""Regenerate the rank-normalized R-hat oracle cases used by Rust tests."""
import json
import math
import random
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "tools/beast"))
from compare_runs import rhat

randomness = random.Random(23)
cases = {
    "independent": [[randomness.gauss(0, 1) for _ in range(101)] for _ in range(4)],
    "shifted": [[randomness.gauss(i, 1) for _ in range(100)] for i in range(4)],
    "scale": [[randomness.gauss(0, i + 1) for _ in range(100)] for i in range(4)],
    "tied": [[randomness.randrange(3) for _ in range(101)] for _ in range(4)],
    "constant": [[1.] * 100] * 4,
    "different_constants": [[float(i)] * 100 for i in range(4)],
    "trend": [[float(i) for i in range(101)], [float(i) for i in reversed(range(101))]],
    "folded_constant": [[0., 1., 0., 1.] * 10, [1., 0., 1., 0.] * 10],
}
results = []
for name, chains in cases.items():
    value = rhat(chains)
    results.append({"name": name, "chains": chains,
                    "rhat": "infinite" if value is not None and not math.isfinite(value) else value})
(ROOT / "fixtures/diagnostics/rhat.json").write_text(json.dumps(results, indent=2) + "\n")
