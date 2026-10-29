#!/usr/bin/env python3
"""Validate Rust diagnostics against BEAST 2.7.8 and Python's inclusive quantiles."""
import argparse
import hashlib
import json
import math
import os
from pathlib import Path
import platform
import random
import statistics
import subprocess
import sys
import tempfile
import xml.etree.ElementTree as ET

ROOT = Path(__file__).resolve().parents[2]

def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()

def generate(path):
    rng = random.Random(34033)
    ar = negative = 0.0
    rows = ['Sample\twhite\tar_positive\tar_negative\ttrend\tconstant\tspike\tskew']
    for i in range(2049):
        white = rng.gauss(0, 1)
        ar = .9 * ar + math.sqrt(1 - .9**2) * rng.gauss(0, 1)
        negative = -.8 * negative + .6 * rng.gauss(0, 1)
        values = [white, ar, negative, i / 2049, 7.0, 100.0 if i == 1201 else white, math.exp(white)]
        rows.append(str(i * 100) + '\t' + '\t'.join(map(repr, values)))
    path.write_text('\n'.join(rows) + '\n')

def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('--packages', type=Path, required=True)
    p.add_argument('--launcher', type=Path, required=True)
    p.add_argument('--engine', type=Path, default=ROOT / 'target/release/rove')
    p.add_argument('--java', default='java')
    p.add_argument('--javac', default='javac')
    p.add_argument('--output', type=Path, required=True)
    p.add_argument('--trace', type=Path)
    p.add_argument('--burnin', type=int, default=25)
    args = p.parse_args()
    for package in ['BEAST.base', 'BEAST.app']:
        version = ET.parse(args.packages / package / 'version.xml').getroot().get('version')
        if version != '2.7.8':
            raise ValueError(f'{package}: expected 2.7.8, got {version}')
    source = ROOT / 'tools/diagnostics/TraceReference.java'
    jars = [args.launcher.resolve()] + [args.packages.resolve() / name / 'lib' / f'{name}.jar' for name in ['BEAST.base', 'BEAST.app']]
    cp = os.pathsep.join(map(str, jars))
    trace = args.trace or ROOT / 'fixtures/diagnostics/synthetic.tsv'
    if args.trace is None:
        generate(trace)
    rust = json.loads(subprocess.check_output([str(args.engine.resolve()), 'diagnose', str(trace.resolve()), '--burnin-percent', str(args.burnin)]))
    with tempfile.TemporaryDirectory(prefix='rove-trace-reference-') as temp:
        subprocess.run([args.javac, '--release', '17', '-cp', cp, '-d', temp, str(source)], check=True)
        output = Path(temp) / 'reference.json'
        subprocess.run([args.java, '-Djava.awt.headless=true', f'-Dbeast.user.package.dir={args.packages.resolve()}', '-cp', temp + os.pathsep + cp, 'rove.reference.TraceReference', str(trace.resolve()), str(args.burnin), str(output)], check=True)
        reference = json.loads(output.read_text())
    rows = [line.split() for line in trace.read_text().splitlines() if line.strip() and not line.startswith('#')]
    labels = rows.pop(0)
    skip = len(rows) * args.burnin // 100
    retained = rows[skip:]
    expected = {}
    differences = {}
    for column in rust['columns']:
        name = column['name']
        values = [float(row[labels.index(name)]) for row in retained]
        cuts = statistics.quantiles(values, n=40, method='inclusive')
        quantiles = [cuts[0], cuts[-1]]
        expected[name] = {'mean': statistics.fmean(values), 'sd': statistics.pstdev(values), 'interval': quantiles,
                          'ess': None if len(set(values)) == 1 else reference[name]['ess']}
        errors = {}
        for field in ['mean', 'sd', 'ess']:
            actual, target = column[field], expected[name][field]
            if target is None:
                assert actual is None, (name, field, actual)
            else:
                errors[field] = abs(actual - target) / max(1, abs(target))
                assert errors[field] < 1e-8, (name, field, actual, target)
        for field in ['mean', 'sd']:
            target = reference[name][field]
            if target is not None:
                errors['beast_' + field] = abs(column[field] - target) / max(1, abs(target))
                assert errors['beast_' + field] < 1e-8, (name, field, column[field], target)
        assert all(abs(a - b) <= 1e-10 * max(1, abs(b)) for a, b in zip(column['interval'], quantiles))
        differences[name] = errors
    report = {'trace_sha256': digest(trace), 'burnin_percent': args.burnin, 'total_samples':len(rows), 'discarded_samples':skip,
        'statistics':expected, 'relative_errors':differences, 'reference_hpd':reference,
        'ess_method':rust['ess_method'], 'interval_method':rust['interval_method'],
        'provenance':{'BEAST.base':'2.7.8','BEAST.app':'2.7.8', 'jar_sha256':{jar.name:digest(jar) for jar in jars},
            'helper_sha256':digest(source), 'python':sys.version, 'platform':platform.platform(),
            'java':subprocess.run([args.java,'-version'],capture_output=True,text=True,check=True).stderr}}
    args.output.parent.mkdir(parents=True,exist_ok=True)
    args.output.write_text(json.dumps(report,indent=2,sort_keys=True)+'\n')
    print(f'Validated {len(expected)} columns, {len(retained)} retained samples; saved {args.output}')

if __name__ == '__main__':
    main()
