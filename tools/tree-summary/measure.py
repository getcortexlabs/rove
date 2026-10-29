#!/usr/bin/env python3
"""Measure streaming tree summary time and peak RSS on macOS; repeated trees test I/O scaling."""
import argparse
import json
from pathlib import Path
import platform
import os
import time
import subprocess

from validate import ROOT, digest


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('--input', type=Path, required=True)
    p.add_argument('--output', type=Path, required=True)
    p.add_argument('--hardware', type=Path, required=True, help='JSON with non-identifying chip, RAM and model fields')
    args = p.parse_args()
    args.output.mkdir(parents=True, exist_ok=False)
    expanded = args.output/'repeated.trees'
    # Ten copies with new state identifiers. This is not an independent MCMC run.
    text = args.input.read_text()
    first = text.index('tree STATE_')
    prefix = text[:first]
    samples = [line.split('=', 1)[1] for line in text[first:].splitlines() if line.startswith('tree STATE_')]
    with expanded.open('w') as out:
        out.write(prefix)
        for i in range(10 * len(samples)):
            out.write(f'tree STATE_{i * 100} ='+samples[i % len(samples)]+'\n')
        out.write('End;\n')
    result = {'hardware': json.loads(args.hardware.read_text()), 'measurements': [], 'inputs': {},
              'rove_sha256': digest(ROOT/'target/release/rove'),
              'rustc': subprocess.check_output(['rustc', '--version'], text=True).strip(),
              'os': platform.platform(), 'build': 'cargo build --locked --release; no RUSTFLAGS'}
    for label, source in [('original', args.input), ('repeated', expanded)]:
        result['inputs'][label] = {'bytes': source.stat().st_size, 'sha256': digest(source)}
        for trial in range(3):
            output = args.output/f'{label}-{trial}'
            command = [str(ROOT/'target/release/rove'), 'summarize', str(source),
                       '--output', str(output), '--burnin-percent', '25', '--heights', 'ca']
            log = args.output/f'{label}-{trial}.json'
            errors = args.output/f'{label}-{trial}.stderr'
            start = time.perf_counter()
            pid = os.posix_spawn(command[0], command, os.environ, file_actions=[
                (os.POSIX_SPAWN_OPEN, 1, str(log), os.O_WRONLY | os.O_CREAT | os.O_TRUNC, 0o644),
                (os.POSIX_SPAWN_OPEN, 2, str(errors), os.O_WRONLY | os.O_CREAT | os.O_TRUNC, 0o644)])
            _, status, usage = os.wait4(pid, 0)
            real = time.perf_counter() - start
            if status != 0:
                raise RuntimeError(f'Summary failed; see {errors}')
            rss = usage.ru_maxrss * (1 if platform.system() == 'Darwin' else 1024)
            manifest = json.loads(log.read_text())
            row = {'input': label, 'trial': trial, 'wall_seconds': real, 'peak_rss_bytes': rss,
                   'total_samples': manifest['total_samples'], 'distinct_clades': manifest['distinct_clades'],
                   'retained_samples': manifest['retained_samples']}
            result['measurements'].append(row)
            print(json.dumps(row), flush=True)
    (args.output/'measurements.json').write_text(json.dumps(result, indent=2)+'\n')


if __name__ == '__main__':
    main()
