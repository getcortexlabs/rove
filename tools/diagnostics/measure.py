#!/usr/bin/env python3
"""Measure standalone trace diagnostics; this does not benchmark MCMC sampling."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import platform
import statistics
import subprocess
import time

ROOT = Path(__file__).resolve().parents[2]

def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('--engine', type=Path, default=ROOT / 'target/release/rove')
    p.add_argument('--hardware', type=Path, required=True, help='Non-identifying hardware JSON recorded on this machine')
    p.add_argument('--work', type=Path, required=True)
    p.add_argument('--output', type=Path, required=True)
    args = p.parse_args()
    args.work.mkdir(parents=True, exist_ok=True)
    fixture = ROOT / 'fixtures/diagnostics/synthetic.tsv'
    lines = fixture.read_text().splitlines()
    patterns = [line.split('\t', 1)[1] for line in lines[1:]]
    measured = []
    for count in [10001, 100001, 250001]:
        trace = args.work / f'trace-{count}.tsv'
        with trace.open('w') as output:
            output.write(lines[0] + '\n')
            for i in range(count):
                output.write(f'{i * 100}\t{patterns[i % len(patterns)]}\n')
        runs = []
        for _ in range(3):
            start = time.perf_counter()
            command = [str(args.engine.resolve()), 'diagnose', str(trace.resolve()), '--burnin-percent', '25']
            errors = args.work / f'trace-{count}.stderr'
            pid = os.posix_spawn(command[0], command, os.environ, file_actions=[
                (os.POSIX_SPAWN_OPEN, 1, os.devnull, os.O_WRONLY, 0o644),
                (os.POSIX_SPAWN_OPEN, 2, str(errors), os.O_WRONLY | os.O_CREAT | os.O_TRUNC, 0o644)])
            _, status, usage = os.wait4(pid, 0)
            if status != 0:
                raise RuntimeError(errors.read_text())
            elapsed = time.perf_counter() - start
            rss = usage.ru_maxrss * (1 if platform.system() == 'Darwin' else 1024)
            runs.append({'wall_seconds': elapsed, 'maximum_rss_bytes':rss})
        measured.append({'rows':count, 'columns':len(patterns[0].split('\t')), 'bytes':trace.stat().st_size,
            'sha256':hashlib.sha256(trace.read_bytes()).hexdigest(), 'runs':runs,
            'median_wall_seconds':statistics.median(run['wall_seconds'] for run in runs)})
    report = {'platform':platform.platform(),'machine':platform.machine(),
        'hardware':json.loads(args.hardware.read_text()),
        'rustc':subprocess.check_output(['rustc','--version'],text=True).strip(),
        'build':'cargo build --release; LTO=fat, codegen-units=1; no target-cpu override',
        'engine_sha256':hashlib.sha256(args.engine.read_bytes()).hexdigest(),
        'fixture_sha256':hashlib.sha256(fixture.read_bytes()).hexdigest(),
        'method':'three separate CLI processes per size; stdout discarded; wait4 per-child peak RSS converted to bytes',
        'caveat':'Rows repeat a synthetic trace. This measures parsing, retained statistics and bounded plot serialization, not convergence or inference performance.',
        'cases':measured}
    args.output.parent.mkdir(parents=True,exist_ok=True)
    args.output.write_text(json.dumps(report,indent=2,sort_keys=True)+'\n')
    print(json.dumps([(x['rows'],x['median_wall_seconds']) for x in measured]))

if __name__ == '__main__':
    main()
