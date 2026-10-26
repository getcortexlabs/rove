#!/usr/bin/env python3
"""Independently check the exported equal-frequency Mk/Mkv likelihood."""

import argparse
import json
import math
from pathlib import Path


def logsumexp(values):
    maximum = max(values)
    return maximum + math.log(math.fsum(math.exp(value - maximum) for value in values))


def check(snapshot):
    nodes = {node["index"]: node for node in snapshot["nodes"]}
    taxa = {taxon: index for index, taxon in enumerate(snapshot["taxa"])}
    frequencies = snapshot["frequencies"]
    states = len(frequencies)
    if any(abs(value - 1 / states) > 1e-12 for value in frequencies):
        raise ValueError("This independent check supports equal-frequency Mk")
    normalization = 1 - math.fsum(value * value for value in frequencies)

    def partials(index, pattern, rate):
        node = nodes[index]
        if not node["children"]:
            observed = pattern[taxa[node["taxon"]]]
            if observed not in range(states):
                raise ValueError("This check requires fully observed states")
            return [float(state == observed) for state in range(states)], 0.0
        result = [1.0] * states
        log_scale = 0.0
        for child_index in node["children"]:
            child = nodes[child_index]
            values, scale = partials(child_index, pattern, rate)
            distance = (node["height"] - child["height"]) * child["branch_rate"] * rate
            if distance < 0:
                raise ValueError("Negative branch distance")
            stay = math.exp(-distance / normalization)
            change = -math.expm1(-distance / normalization)
            equilibrium = math.fsum(f * value for f, value in zip(frequencies, values))
            result = [old * (stay * value + change * equilibrium) for old, value in zip(result, values)]
            log_scale += scale
        maximum = max(result)
        return [value / maximum for value in result], log_scale + math.log(maximum)

    pattern_logs = []
    for pattern in snapshot["patterns"]:
        categories = []
        for rate, weight in zip(snapshot["category_rates"], snapshot["category_weights"]):
            values, scale = partials(snapshot["root"], pattern["states"], rate)
            categories.append(math.log(weight) + math.log(math.fsum(f * value for f, value in zip(frequencies, values))) + scale)
        pattern_logs.append(logsumexp(categories))
    excluded = [index for index, pattern in enumerate(snapshot["patterns"]) if pattern["weight"] == 0]
    constants = {tuple(pattern["states"]) for pattern in snapshot["patterns"] if pattern["weight"] == 0}
    if constants != {tuple([state] * len(taxa)) for state in range(states)}:
        raise ValueError("Expected exactly the excluded constant patterns")
    log_constant_probability = logsumexp([pattern_logs[index] for index in excluded])
    correction = math.log(-math.expm1(log_constant_probability))
    likelihood = math.fsum(pattern["weight"] * (value - correction)
                           for pattern, value in zip(snapshot["patterns"], pattern_logs))
    pattern_error = max(abs(value - pattern["log_likelihood"])
                        for pattern, value in zip(snapshot["patterns"], pattern_logs))
    likelihood_error = abs(likelihood - snapshot["reconstructed_log_likelihood"])
    if pattern_error > 1e-9 or likelihood_error > 1e-9:
        raise AssertionError(f"Likelihood mismatch: pattern={pattern_error}, total={likelihood_error}")
    return {"log_likelihood": likelihood, "max_pattern_error": pattern_error, "total_error": likelihood_error}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("snapshots", nargs="+", type=Path)
    args = parser.parse_args()
    for path in args.snapshots:
        print(path.name, json.dumps(check(json.loads(path.read_text()))))


if __name__ == "__main__":
    main()
