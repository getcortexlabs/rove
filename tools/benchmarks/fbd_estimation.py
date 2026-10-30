#!/usr/bin/env python3
"""Run the constructed four-tip FBD workload; retain traces in a local directory."""
import argparse
import csv
import hashlib
import json
import math
from pathlib import Path
import platform
import statistics
import subprocess
import time

ROOT = Path(__file__).resolve().parents[2]
MATRIX = Path('fixtures/validation/dating/matrix.nex')
SETTINGS = {
    'fixed': Path('fixtures/validation/dating/dating.json'),
    'estimated': Path('fixtures/validation/fbd-estimation/dating.json'),
}


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def capture(*args):
    return subprocess.check_output(args, cwd=ROOT, text=True).strip()


def trace_rows(path, burnin):
    with path.open() as f:
        rows = list(csv.DictReader((x for x in f if not x.startswith('#')), delimiter='\t'))
    return [{k: float(v) for k, v in row.items()} for row in rows[burnin:]]


def correlation(rows, left, right):
    x, y = [r[left] for r in rows], [r[right] for r in rows]
    mx, my = statistics.mean(x), statistics.mean(y)
    xx = sum((v-mx)**2 for v in x)
    yy = sum((v-my)**2 for v in y)
    return sum((a-mx)*(b-my) for a, b in zip(x, y)) / math.sqrt(xx*yy)


def summarize(directory, elapsed, mode, preset, rep, cmd):
    comparison = json.loads((directory / 'comparison.json').read_text())
    reports = [json.loads(p.read_text()) for p in sorted(directory.glob('chain-*/run.json'))]
    chains = [trace_rows(p, comparison['discarded_per_chain'])
              for p in sorted(directory.glob('chain-*/trace.log'))]
    variables = [s for s in comparison['statistics'] if not s.get('fixed_by_model', False)]
    min_ess = min(v for s in variables for v in s['ess'] if v is not None)
    correlations = {}
    pairs = [('clock_rate', 'tree_height'), ('clock_rate', 'tip_age_2')]
    if mode == 'estimated':
        pairs += [('fbd_origin', 'tree_height'), ('fbd_birth_rate', 'fbd_origin'),
                  ('fbd_death_rate', 'fbd_fossil_sampling_rate')]
    for left, right in pairs:
        correlations[f'{left}:{right}'] = [correlation(rows, left, right) for rows in chains]
    result = {
        'mode': mode, 'preset': preset, 'repetition': rep+1,
        'command': cmd[1:6] + ['<local-output>'] + cmd[7:],
        'chain_seeds': [r['settings']['seed'] for r in reports],
        'group_wall_seconds': elapsed,
        'retained_per_chain': comparison['retained_per_chain'],
        'burnin_percent': comparison['burnin_percent'],
        'minimum_ess_per_chain': min_ess,
        'minimum_ess_per_group_wall_second': min_ess/elapsed,
        'max_rhat': max(s['rhat'] for s in variables if s['rhat'] is not None),
        'unavailable_sampled_diagnostics': [s['name'] for s in variables
            if s['rhat'] is None or any(x is None for x in s['ess'])],
        'max_unconstrained_clade_difference': comparison['max_unconstrained_clade_difference'],
        'pearson_correlations_per_chain': correlations,
        'statistics': comparison['statistics'],
        'clades': comparison['clades'],
        'topology_convergence_assessed': comparison['topology_convergence_assessed'],
        'trace_sha256': [sha(p) for p in sorted(directory.glob('chain-*/trace.log'))],
        'tree_sha256': [sha(p) for p in sorted(directory.glob('chain-*/trees.nexus'))],
    }
    return result


def metadata(binary):
    hardware = json.loads(capture('system_profiler', 'SPHardwareDataType', '-json'))['SPHardwareDataType'][0]
    record = {
        'protocol': 'fbd-estimation-4x8-v1',
        'matrix_sha256': sha(ROOT / MATRIX),
        'settings_sha256': {k: sha(ROOT/v) for k, v in SETTINGS.items()},
        'binary_sha256': sha(binary),
        'base_commit': capture('git', 'rev-parse', 'HEAD'),
        'engine_source_sha256': {str(p.relative_to(ROOT)): sha(p) for p in
            sorted((ROOT/'src').rglob('*.rs')) + [ROOT/'Cargo.toml', ROOT/'Cargo.lock']},
        'platform': platform.platform(),
        'cpu': hardware['chip_type'],
        'processors': hardware['number_processors'],
        'memory': hardware['physical_memory'],
        'rustc': capture('rustc','-Vv'),
        'build': 'cargo build --release --bin rove; fat LTO, one codegen unit; default target features; NEON runtime backend',
        'method': 'Sequential groups of four concurrent chains, alternating workload order across repetitions; no separate warmup. External wall time includes startup, trace/tree IO, checkpoints and final comparison. Burn-in 10% by sample count. No BEAST timing comparison.',
    }
    return record


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--output', type=Path, required=True, help='New local output directory')
    parser.add_argument('--binary', type=Path, default=ROOT / 'target/release/rove')
    parser.add_argument('--steps', type=int, default=1_000_000)
    parser.add_argument('--repetitions', type=int, default=3)
    parser.add_argument('--modes', nargs='+', choices=list(SETTINGS), default=list(SETTINGS))
    parser.add_argument('--presets', nargs='+', choices=['strict', 'reference'], default=['strict', 'reference'])
    args = parser.parse_args()
    if args.steps < 1000 or args.repetitions < 1:
        parser.error('Use at least 1,000 steps and one repetition.')
    binary = args.binary.resolve()
    output = args.output.resolve()
    output.mkdir(parents=True, exist_ok=False)
    record = metadata(binary)
    results = []
    cases = [(a, b) for a in args.modes for b in args.presets]
    for rep in range(args.repetitions):
        # Alternate order to expose, rather than hide, startup/order variation.
        ordered = cases if rep % 2 == 0 else list(reversed(cases))
        for mode, preset in ordered:
            directory = output / f'{mode}-{preset}-{rep+1}'
            cmd = [str(binary), 'sample', str(MATRIX), '--dating', str(SETTINGS[mode]),
                   '--output', str(directory), '--model', preset, '--seed', str(41+rep*10),
                   '--steps', str(args.steps), '--sample-every', '100', '--chains', '4']
            with (output / f'{directory.name}.log').open('w') as log:
                start = time.perf_counter()
                subprocess.run(cmd, cwd=ROOT, stdout=log, stderr=log, check=True)
                elapsed = time.perf_counter()-start
            result = summarize(directory, elapsed, mode, preset, rep, cmd)
            results.append(result)
            record['results'] = results
            (output/'report.json').write_text(json.dumps(record,indent=2)+'\n')
            print(f'{directory.name}: {elapsed:.3f}s; min ESS {result["minimum_ess_per_chain"]:.1f}; max R-hat {result["max_rhat"]:.4f}', flush=True)
    record['results'] = results
    (output/'report.json').write_text(json.dumps(record,indent=2)+'\n')


if __name__ == '__main__':
    main()
