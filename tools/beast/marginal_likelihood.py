#!/usr/bin/env python3
"""Estimate a marginal likelihood with BEAST 2.7.8's PathSampler and with
`rove marginal` on one strict-clock configuration, recompute both estimates
from their logs with one implementation, and compare them with the criteria
in fixtures/validation/MARGINAL-LIKELIHOOD.md."""

import argparse
import hashlib
import json
import math
import platform
import re
import subprocess
import time
import xml.etree.ElementTree as ET
from pathlib import Path

from strict_clock import configuration, rove_command

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]
MAX_LAG = 2000
PAIR_TOLERANCE = 4.0
CONSISTENCY_TOLERANCE = 1e-6
POWER_CONCERN = 2


def sha256(path):
    digest = hashlib.sha256()
    with open(path, "rb") as source:
        for block in iter(lambda: source.read(1 << 20), b""):
            digest.update(block)
    return digest.hexdigest()


def schedule(powers, alpha):
    return [(i / (powers - 1)) ** (1.0 / alpha) for i in range(powers)]


def read_column(path, column, burnin_percent):
    rows = [line.split() for line in path.read_text().splitlines() if line and not line.startswith("#")]
    index = rows[0].index(column)
    values = [float(row[index]) for row in rows[1:]]
    if not all(math.isfinite(v) for v in values):
        raise ValueError(f"{path} has non-finite {column} values")
    return values[len(values) * burnin_percent // 100:]


def mean(values):
    return math.fsum(values) / len(values)


def ess(values):
    """Initial positive sequence estimator, as in Rove's diagnostics: the
    autocovariance is summed in pairs of lags until a pair is not positive."""
    n = len(values)
    centre = mean(values)
    deviations = [v - centre for v in values]
    variance = math.fsum(d * d for d in deviations) / n
    if variance == 0.0:
        return None, 0.0
    if n < 4:
        return None, variance

    def covariance(lag):
        return math.fsum(a * b for a, b in zip(deviations, deviations[lag:])) / n

    integral = variance
    for lag in range(2, min(MAX_LAG, n), 2):
        pair = covariance(lag - 1) + covariance(lag)
        if pair <= 0.0:
            break
        integral += 2.0 * pair
    estimate = n * variance / integral
    return (min(estimate, n) if math.isfinite(estimate) and estimate > 0 else None), variance


def mean_variance(values):
    """Mean and the variance of the mean, sd² / ESS; None without a usable ESS."""
    effective, variance = ess(values)
    if variance == 0.0:
        return mean(values), 0.0, None
    if effective is None:
        return mean(values), None, None
    return mean(values), variance / effective, effective


def estimate(samples):
    """Stepping-stone and path-sampling estimates with standard errors from
    [(power, log-likelihoods)] in increasing power from zero to one."""
    powers = [p for p, _ in samples]
    if powers[0] != 0.0 or powers[-1] != 1.0 or any(a >= b for a, b in zip(powers, powers[1:])):
        raise ValueError("powers must increase from zero to one")
    summaries = []
    for power, values in samples:
        m, v, effective = mean_variance(values)
        summaries.append({"power": power, "samples": len(values), "mean": m, "variance_of_mean": v, "ess": effective})
    stones = []
    for k in range(1, len(samples)):
        lower, upper = powers[k - 1], powers[k]
        difference = upper - lower
        values = samples[k - 1][1]
        largest = max(values)
        weights = [math.exp(difference * (v - largest)) for v in values]
        m, v, effective = mean_variance(weights)
        stones.append({"lower": lower, "upper": upper, "log_ratio": difference * largest + math.log(m),
                       "variance": None if v is None else v / (m * m), "ess": effective})
    ss_variance = None if any(s["variance"] is None for s in stones) else math.fsum(s["variance"] for s in stones)
    coefficients = [((powers[k] - powers[k - 1]) if k > 0 else 0.0) / 2 + ((powers[k + 1] - powers[k]) if k + 1 < len(powers) else 0.0) / 2
                    for k in range(len(powers))]
    ps_variance = None if any(s["variance_of_mean"] is None for s in summaries) else math.fsum(
        c * c * s["variance_of_mean"] for c, s in zip(coefficients, summaries))
    return {
        "stepping_stone": {"log_marginal_likelihood": math.fsum(s["log_ratio"] for s in stones),
                           "standard_error": None if ss_variance is None else math.sqrt(ss_variance)},
        "path_sampling": {"log_marginal_likelihood": math.fsum(c * s["mean"] for c, s in zip(coefficients, summaries)),
                          "standard_error": None if ps_variance is None else math.sqrt(ps_variance)},
        "powers": summaries,
        "stones": stones,
    }


def rove_samples(directory, powers, burnin_percent):
    return [(power, read_column(directory / f"power-{index:02d}" / "trace.log", "likelihood", burnin_percent))
            for index, power in enumerate(powers)]


def beast_samples(steps, powers, burnin_percent):
    # PathSampler runs step i at the (n − 1 − i)-th power of the ascending schedule.
    n = len(powers)
    return [(power, read_column(steps / f"step{n - 1 - index}" / "likelihood.log", "likelihood", burnin_percent))
            for index, power in enumerate(powers)]


def jars(args):
    return [args.launcher.resolve(), *sorted(args.packages.resolve().glob("*/lib/*.jar"))]


def path_sampler_xml(xml, output, args):
    """The configuration's MCMC wrapped in a PathSampler run. The step script
    starts each step's BeastMCMC directly on the pinned jars, as the reference
    runs do, instead of the launcher's installation lookup."""
    text = xml.read_text()
    match = re.search(r'<run id="mcmc" spec="MCMC" chainLength="\d+">', text)
    if match is None:
        raise ValueError(f"{xml} has no MCMC run element")
    script = ("\ncd $(dir)\n" + str(args.java) + " -Xss8m -Djava.awt.headless=true -Dbeast.user.package.dir="
              + str(args.packages.resolve()) + " -cp " + ":".join(str(j) for j in jars(args))
              + " beastfx.app.beast.BeastMCMC -batch $(resume/overwrite) -seed $(seed) -threads 1 beast.xml\n\n")
    head = (f'<run id="pathSampler" spec="modelselection.inference.PathSampler" nrOfSteps="{args.powers}" alpha="{args.alpha}"'
            f' chainLength="{args.chain_length}" burnInPercentage="{args.burnin_percent}" preBurnin="{args.pre_burnin}"'
            f' posterior2prior="true" deleteOldLogs="true" rootdir="{output.resolve()}">' + script)
    inner = match.group(0).replace('<run id="mcmc"', '<mcmc id="mcmc"')
    text = text[:match.start()] + head + inner + text[match.end():]
    text = text.replace("</run>", "</mcmc>\n</run>")
    return text


def run_beast(args, xml, seed, directory, log):
    directory.mkdir(parents=True)
    steps = directory / "steps"
    configured = directory / "path-sampler.xml"
    configured.write_text(path_sampler_xml(xml, steps, args))
    command = [str(args.java), "-Xms256m", "-Xmx2g", "-Xss8m", "-Djava.awt.headless=true", "-Duser.language=en",
               f"-Dbeast.user.package.dir={args.packages.resolve()}", "-cp", ":".join(str(j) for j in jars(args)),
               "beastfx.app.beast.BeastMCMC", "-batch", "-overwrite", "-seed", str(seed), "-threads", "1", str(configured.resolve())]
    started = time.perf_counter()
    with open(log, "w") as console:
        subprocess.run(command, stdout=console, stderr=subprocess.STDOUT, check=True, cwd=directory)
    seconds = time.perf_counter() - started
    text = log.read_text()
    reported = re.findall(r"marginal L estimate = (\S+)", text)
    if len(reported) != 1:
        raise RuntimeError(f"PathSampler did not report one estimate; see {log}")
    return {"command": command, "seed": seed, "wall_seconds": seconds, "reported_log_marginal_likelihood": float(reported[0]),
            "steps": steps}


def run_rove(args, config, seed, directory, log):
    command = rove_command(args.rove, args.matrix, directory, seed, config)
    command[1] = "marginal"
    command[command.index("--steps") + 1] = str(args.chain_length)
    command[command.index("--sample-every") + 1] = str(args.chain_length // 1000)
    command += ["--powers", str(args.powers), "--alpha", str(args.alpha), "--burnin-percent", str(args.burnin_percent)]
    started = time.perf_counter()
    with open(log, "w") as console:
        subprocess.run(command, stdout=console, stderr=subprocess.STDOUT, check=True)
    seconds = time.perf_counter() - started
    results = json.loads((directory / "marginal.json").read_text())
    return {"command": command, "seed": seed, "wall_seconds": seconds, "marginal": results}


def combined(a, b):
    return math.sqrt(a["standard_error"] ** 2 + b["standard_error"] ** 2)


def compare(rove_runs, beast_runs, powers):
    pairs = []
    for r in rove_runs:
        for b in beast_runs:
            x, y = r["recomputed"]["stepping_stone"], b["recomputed"]["stepping_stone"]
            error = combined(x, y)
            pairs.append({"rove_seed": r["seed"], "beast_seed": b["seed"],
                          "difference": x["log_marginal_likelihood"] - y["log_marginal_likelihood"],
                          "combined_standard_error": error,
                          "agrees": abs(x["log_marginal_likelihood"] - y["log_marginal_likelihood"]) <= PAIR_TOLERANCE * error})
    power_flags = []
    for r in rove_runs:
        for b in beast_runs:
            flagged = 0
            for rp, bp in zip(r["recomputed"]["powers"], b["recomputed"]["powers"]):
                if rp["variance_of_mean"] is None or bp["variance_of_mean"] is None:
                    flagged += 1
                    continue
                error = math.sqrt(rp["variance_of_mean"] + bp["variance_of_mean"])
                if abs(rp["mean"] - bp["mean"]) > PAIR_TOLERANCE * error:
                    flagged += 1
            power_flags.append({"rove_seed": r["seed"], "beast_seed": b["seed"], "powers_beyond_tolerance": flagged})
    consistency = {
        "rove": [abs(r["recomputed"]["stepping_stone"]["log_marginal_likelihood"]
                     - r["marginal"]["stepping_stone"]["log_marginal_likelihood"]) for r in rove_runs],
        "beast": [abs(b["recomputed"]["stepping_stone"]["log_marginal_likelihood"] - b["reported_log_marginal_likelihood"])
                  for b in beast_runs],
    }
    return {
        "pairs": pairs,
        "all_pairs_agree": all(p["agrees"] for p in pairs),
        "consistency": consistency,
        "consistent": all(d <= CONSISTENCY_TOLERANCE for d in consistency["rove"] + consistency["beast"]),
        "power_flags": power_flags,
        "power_concern": any(f["powers_beyond_tolerance"] > POWER_CONCERN for f in power_flags),
        "powers": len(powers),
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--packages", type=Path, required=True, help="BEAST package directory including MODEL_SELECTION")
    parser.add_argument("--launcher", type=Path, required=True)
    parser.add_argument("--java", type=Path, default=Path("java"))
    parser.add_argument("--xml", type=Path, required=True, help="strict-clock configuration")
    parser.add_argument("--matrix", type=Path, required=True)
    parser.add_argument("--rove", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True, help="new directory")
    parser.add_argument("--powers", type=int, default=32)
    parser.add_argument("--alpha", type=float, default=0.3)
    parser.add_argument("--chain-length", type=int, default=1_000_000)
    parser.add_argument("--burnin-percent", type=int, default=20)
    parser.add_argument("--pre-burnin", type=int, default=200_000)
    parser.add_argument("--beast-seeds", type=int, nargs="+", default=[24101, 24102, 24103])
    parser.add_argument("--rove-seeds", type=int, nargs="+", default=[24001, 24002, 24003])
    args = parser.parse_args()
    args.output.mkdir(parents=True, exist_ok=False)
    for package in ["BEAST.app", "BEAST.base"]:
        version = ET.parse(args.packages / package / "version.xml").getroot().attrib["version"]
        if version != "2.7.8":
            raise ValueError(f"{package} must be pinned to 2.7.8, found {version}")
    selection = ET.parse(args.packages / "MODEL_SELECTION" / "version.xml").getroot().attrib["version"]
    config = configuration(args.xml)
    config["chain_length"] = args.chain_length
    config["log_every"] = args.chain_length // 1000
    powers = schedule(args.powers, args.alpha)
    beast_runs, rove_runs = [], []
    for seed in args.beast_seeds:
        directory = args.output / f"beast-{seed}"
        run = run_beast(args, args.xml, seed, directory, args.output / f"beast-{seed}.console.log")
        run["recomputed"] = estimate(beast_samples(run.pop("steps"), powers, args.burnin_percent))
        beast_runs.append(run)
        print(json.dumps({"beast": seed, "reported": run["reported_log_marginal_likelihood"],
                          "recomputed": run["recomputed"]["stepping_stone"], "seconds": run["wall_seconds"]}), flush=True)
    for seed in args.rove_seeds:
        directory = args.output / f"rove-{seed}"
        run = run_rove(args, config, seed, directory, args.output / f"rove-{seed}.console.log")
        run["recomputed"] = estimate(rove_samples(directory, powers, args.burnin_percent))
        rove_runs.append(run)
        print(json.dumps({"rove": seed, "reported": run["marginal"]["stepping_stone"],
                          "recomputed": run["recomputed"]["stepping_stone"], "seconds": run["wall_seconds"]}), flush=True)
    comparison = compare(rove_runs, beast_runs, powers)
    print(json.dumps({k: comparison[k] for k in ["all_pairs_agree", "consistent", "power_concern"]}), flush=True)
    record = {
        "protocol_sha256": sha256(ROOT / "fixtures/validation/MARGINAL-LIKELIHOOD.md"),
        "script_sha256": sha256(Path(__file__)),
        "xml": {"path": str(args.xml), "sha256": sha256(args.xml)},
        "matrix_sha256": sha256(args.matrix),
        "rove_sha256": sha256(args.rove),
        "packages": {p.parent.name: ET.parse(p).getroot().attrib["version"] for p in sorted(args.packages.glob("*/version.xml"))},
        "model_selection_version": selection,
        "jars_sha256": {str(p.relative_to(args.packages.resolve())): sha256(p) for p in sorted(args.packages.resolve().glob("*/lib/*.jar"))},
        "launcher_sha256": sha256(args.launcher),
        "java": subprocess.run([str(args.java), "-version"], text=True, capture_output=True, check=True).stderr.strip(),
        "platform": {"machine": platform.machine(), "system": platform.platform()},
        "settings": {"powers": args.powers, "alpha": args.alpha, "schedule": powers, "chain_length": args.chain_length,
                     "log_every": args.chain_length // 1000, "burnin_percent": args.burnin_percent,
                     "beast_pre_burnin": args.pre_burnin, "configuration": config},
        "beast": beast_runs,
        "rove": rove_runs,
        "comparison": comparison,
    }
    (args.output / "comparison.json").write_text(json.dumps(record, indent=2, allow_nan=False) + "\n")


if __name__ == "__main__":
    main()
