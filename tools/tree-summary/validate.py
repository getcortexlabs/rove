#!/usr/bin/env python3
"""Compare Rove summaries with pinned external tools; none are runtime dependencies."""
import argparse
import hashlib
import json
import math
from pathlib import Path
import subprocess
import xml.etree.ElementTree as ET

ROOT = Path(__file__).resolve().parents[2]


def run(command, log):
    result = subprocess.run([str(x) for x in command], cwd=ROOT, capture_output=True)
    log.write_bytes(result.stdout + result.stderr)
    if result.returncode:
        raise RuntimeError(f"Command failed; see {log}")
    return result.stdout.decode()


def rows(text):
    result = {}
    for line in text.splitlines():
        fields = line.split('\t')
        if len(fields) == 4:
            result[fields[0]] = [None if x == 'null' else float(x) for x in fields[1:]]
    if not result:
        raise ValueError('No parsed nodes')
    return result


def compare(a, b):
    assert a.keys() == b.keys(), 'Taxa or topology differ'
    differences = [0.0, 0.0, 0.0]
    for clade in a:
        for i, (x, y) in enumerate(zip(a[clade], b[clade])):
            assert (x is None) == (y is None), (clade, i, x, y)
            if x is not None:
                assert math.isclose(x, y, rel_tol=1e-9, abs_tol=1e-12), (clade, i, x, y)
                differences[i] = max(differences[i], abs(x-y))
    return dict(zip(['height', 'posterior', 'rate'], differences))


def digest(path):
    h = hashlib.sha256()
    with open(path, 'rb') as source:
        for block in iter(lambda: source.read(1024 * 1024), b''):
            h.update(block)
    return h.hexdigest()


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('--java', required=True, help='Java 17 with JavaFX, as bundled with BEAST')
    p.add_argument('--classpath', required=True, help='Pinned BEAST.base, BEAST.app and launcher jars')
    p.add_argument('--packages', type=Path, required=True)
    p.add_argument('--figtree', type=Path, required=True, help='FigTree 1.4.4 jar')
    p.add_argument('--input', type=Path, required=True, help='Completed Rove reference-clock trees.nexus')
    p.add_argument('--output', type=Path, required=True, help='New validation directory')
    args = p.parse_args()
    for package in ['BEAST.app', 'BEAST.base']:
        version = ET.parse(args.packages/package/'version.xml').getroot().attrib['version']
        if version != '2.7.8':
            raise ValueError(f'{package} must be pinned to 2.7.8, found {version}')
    if digest(args.figtree) != '23ab8691cfc0c40bb279e466854f04156194a2b9bd2ba51686fcabfdd0b3fa62':
        raise ValueError('Expected the FigTree 1.4.4 jar from the official tgz release')
    args.output.mkdir(parents=True, exist_ok=False)
    out = args.output.resolve()
    figtree = args.figtree.resolve()
    for helper, cp in [('ReadFigTree', str(figtree)), ('ReadBeast', args.classpath)]:
        run(['javac', '-d', out, '-cp', cp, ROOT/'tools/tree-summary'/f'{helper}.java'], out/f'{helper}-compile.log')
    evidence = {'comparisons': [], 'inputs': {}, 'tools': {
        'BEAST.app': '2.7.8 (banner reports 2.7.7)', 'BEAST.base': '2.7.8', 'FigTree': '1.4.4',
        'figtree_sha256': digest(figtree), 'rove_sha256': digest(ROOT/'target/release/rove'),
        'beast_jars': {Path(x).name: digest(x) for x in args.classpath.split(':') if x.endswith('.jar')},
    }}
    inputs = [('toy', ROOT/'fixtures/summary/four-taxa.trees', 25),
              ('tie', ROOT/'fixtures/summary/four-taxa.trees', 0),
              ('reference', args.input.resolve(), 25)]
    for name, source, burnin in inputs:
        evidence['inputs'][name] = {'sha256': digest(source), 'bytes': source.stat().st_size}
        for heights in ['keep', 'ca']:
            key = f'{name}-{heights}'
            run([ROOT/'target/release/rove', 'summarize', source, '--output', out/key,
                 '--burnin-percent', burnin, '--heights', heights], out/f'{key}-rove.log')
            target = out/f'{key}-beast.tree'
            run([args.java, '-Dbeast.user.package.dir='+str(args.packages.resolve()), '-cp', args.classpath,
                 'beastfx.app.treeannotator.TreeAnnotator', '-burnin', burnin,
                 '-height', 'CA' if heights == 'ca' else 'keep', source, target], out/f'{key}-beast.log')
            parsed = []
            for engine, file in [('rove', out/key/'mcc.tree'), ('beast', target)]:
                parsed.append(rows(run([args.java, '-cp', str(out)+':'+str(figtree), 'ReadFigTree', file], out/f'{key}-{engine}-figtree.tsv')))
            independent = rows(run([args.java, '-cp', str(out)+':'+args.classpath, 'ReadBeast', out/key/'mcc.tree'], out/f'{key}-independent.tsv'))
            comparison = {'case': key, 'nodes': len(parsed[0]), 'max_abs_difference': compare(*parsed),
                          'independent_parser_max_abs_difference': compare(parsed[0], independent)}
            evidence['comparisons'].append(comparison)
            print(json.dumps(comparison), flush=True)
    (out/'validation.json').write_text(json.dumps(evidence, indent=2)+'\n')


if __name__ == '__main__':
    main()
