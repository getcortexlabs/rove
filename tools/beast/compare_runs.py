#!/usr/bin/env python3
"""Compare independent traces using rank-normalized split R-hat."""

import argparse
import json
import math
import statistics
from pathlib import Path


def read_trace(path, burnin=10):
    rows = [line.split() for line in path.read_text().splitlines() if line and not line.startswith("#")]
    labels, values = rows[0], rows[1:]
    retained = values[len(values) * burnin // 100:]
    columns = {label: [float(row[index]) for row in retained] for index, label in enumerate(labels)}
    if not all(math.isfinite(value) for column in columns.values() for value in column):
        raise ValueError("Traces must contain finite values")
    return columns


def rank_normalize(chains):
    pooled = [value for chain in chains for value in chain]
    order = sorted(range(len(pooled)), key=pooled.__getitem__)
    transformed = [0.0] * len(pooled)
    normal = statistics.NormalDist()
    start = 0
    while start < len(order):
        end = start + 1
        while end < len(order) and pooled[order[end]] == pooled[order[start]]:
            end += 1
        average_rank = (start + 1 + end) / 2
        quantile = normal.inv_cdf((average_rank - 3 / 8) / (len(pooled) + 1 / 4))
        for index in order[start:end]:
            transformed[index] = quantile
        start = end
    width = len(chains[0])
    return [transformed[index:index + width] for index in range(0, len(transformed), width)]


def variance_ratio(chains):
    count = len(chains[0])
    within = statistics.fmean(statistics.variance(chain) for chain in chains)
    between = count * statistics.variance(statistics.fmean(chain) for chain in chains)
    if within == 0:
        return math.inf
    return math.sqrt(((count - 1) / count * within + between / count) / within)


def rhat(chains):
    if len(chains) < 2 or len({len(chain) for chain in chains}) != 1 or len(chains[0]) < 4:
        raise ValueError("Need at least two equal-length chains with four samples")
    if len({value for chain in chains for value in chain}) == 1:
        return None
    half = len(chains[0]) // 2
    split = [segment for chain in chains for segment in (chain[:half], chain[-half:])]
    median = statistics.median(value for chain in split for value in chain)
    folded = [[abs(value - median) for value in chain] for chain in split]
    # Vehtari et al. (2021): retain the larger rank and folded-rank diagnostic.
    bulk = variance_ratio(rank_normalize(split))
    if len({value for chain in folded for value in chain}) == 1:
        return bulk
    return max(bulk, variance_ratio(rank_normalize(folded)))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("logs", nargs="+", type=Path)
    parser.add_argument("--burnin", type=int, default=10)
    args = parser.parse_args()
    if not 0 <= args.burnin < 100:
        parser.error("Burn-in must be between 0 and 99 percent")
    traces = [read_trace(path, args.burnin) for path in args.logs]
    if any(set(trace) != set(traces[0]) for trace in traces):
        parser.error("Trace columns differ")
    diagnostics = {}
    for label in traces[0]:
        if label.lower() == "sample":
            continue
        value = rhat([trace[label] for trace in traces])
        diagnostics[label] = value if value is None or math.isfinite(value) else "infinite"
    print(json.dumps({"method": "rank-normalized split R-hat, including folded ranks",
                      "reference": "https://doi.org/10.1214/20-BA1221",
                      "burnin_percent": args.burnin, "rhat": diagnostics}, indent=2))


if __name__ == "__main__":
    main()
