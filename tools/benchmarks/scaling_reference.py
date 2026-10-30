#!/usr/bin/env python3
"""Generate unscaled, 60-digit Mk pruning values using Python's Decimal matrices."""

import argparse
import json
from decimal import Decimal, localcontext
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
DESTINATION = ROOT / "fixtures/validation/scaling/exact.json"


def evaluate(states, categories, tips, distance, ladder=False):
    frequencies = [1.0 / states] * states
    beta = 1.0 - sum(f * f for f in frequencies)
    rates = [1.0] if categories == 1 else [0.25, 0.75, 1.25, 1.75]
    weights = [1.0 / categories] * categories
    joins = []
    available = list(range(tips))
    while len(available) > 1:
        a, b = available[:2]
        del available[:2]
        node = tips + len(joins)
        joins.append([a, b])
        if ladder:
            available.insert(0, node)
        else:
            available.append(node)
    # Three observed patterns, followed by each constant pattern for Mkv.
    cells = [[(tip + pattern) % states if (tip + 2 * pattern) % 7 else states
              for pattern in range(3)] + list(range(states)) for tip in range(tips)]
    pattern_count = 3 + states
    with localcontext() as ctx:
        ctx.prec = 60
        f = [Decimal(x) for x in frequencies]
        totals = [Decimal(0)] * pattern_count
        for rate, weight in zip(rates, weights):
            # Round branch distances exactly once, as in the Rust caller.
            stay = (-Decimal(distance * rate) / Decimal(beta)).exp()
            change = 1 - stay
            matrix = [[change * f[j] + (stay if i == j else 0)
                       for j in range(states)] for i in range(states)]
            partials = [[[Decimal(int(cell == s or cell == states)) for s in range(states)]
                         for cell in row] for row in cells]
            for a, b in joins:
                partials.append([
                    [sum(matrix[i][j] * partials[a][p][j] for j in range(states))
                     * sum(matrix[i][j] * partials[b][p][j] for j in range(states))
                     for i in range(states)] for p in range(pattern_count)])
            for p in range(pattern_count):
                totals[p] += Decimal(weight) * sum(f[s] * partials[-1][p][s] for s in range(states))
        logs = [x.ln() for x in totals]
        mk = sum(logs[:3])
        log_variable = (1 - sum(totals[3:])).ln()
        return {"states": states, "categories": categories, "tips": tips,
                "distance": distance, "ladder": ladder, "frequencies": frequencies,
                "rates": rates, "category_weights": weights,
                "pattern_log_likelihoods": [str(x) for x in logs],
                "mk": str(mk), "log_variable": str(log_variable),
                "mkv": str(mk - 3 * log_variable)}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    cases = [evaluate(k, c, 7, d) for k in [2, 3, 4, 8] for c in [1, 4]
             for d in [1e-9, 0.001, 1.0, 100.0]]
    cases += [evaluate(k, 4, 2048, 1.0, ladder) for k in [2, 4] for ladder in [False, True]]
    text = json.dumps({"precision_digits": 60, "method": "unscaled Decimal transition matrices",
                       "cases": cases}, indent=2) + "\n"
    if args.check:
        if DESTINATION.read_text() != text:
            raise SystemExit("scaling reference differs; regenerate the fixture")
    else:
        DESTINATION.parent.mkdir(parents=True, exist_ok=True)
        DESTINATION.write_text(text)
    print(f"Verified {len(cases)} cases" if args.check else f"Wrote {len(cases)} cases")


if __name__ == "__main__":
    main()
