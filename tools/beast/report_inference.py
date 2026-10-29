#!/usr/bin/env python3
"""Archive compact inference results and render tables from their recorded values."""

import argparse
import hashlib
import json
import re
import shutil
import statistics
import sys
from pathlib import Path

from prepare_inference import read_matrix
from compare_inference import STATISTICS

ROOT = Path(__file__).resolve().parents[2]
CASES = ['reference-mkv', 'reference-mk', 'control', 'homoplasy', 'missing', 'reduced']


def write_json(path, data):
    path.write_text(json.dumps(data, indent=2, allow_nan=False) + '\n')


def sanitize(value, root):
    if isinstance(value, str):
        return value.replace(str(root), '<repository>')
    if isinstance(value, list):
        return [sanitize(v, root) for v in value]
    if isinstance(value, dict):
        return {k: sanitize(v, root) for k, v in value.items()}
    return value


def operator_counts(directory, metadata):
    result = {'rove': {}, 'beast': {}}
    for seed in metadata['seeds']:
        run = json.loads((directory / 'rove' / str(seed) / 'run.json').read_text())
        result['rove'][str(seed)] = {'settings': run['settings'], 'moves': run['moves']}
        checkpoint = (directory / 'beast' / str(seed) / 'analysis.xml.state').read_text()
        schedules = [json.loads(block) for block in re.findall(r'<!--\s*(\{.*?\})\s*-->', checkpoint, re.S)]
        schedule = next(s for s in schedules if 'operators' in s)
        result['beast'][str(seed)] = schedule
    return result


def span(values, digits=2):
    return f'{min(values):.{digits}f}–{max(values):.{digits}f}'


def tables(selected, all_runs):
    text = [
        '## Agreement and convergence', '',
        '| Case | Steps per chain | Largest absolute mean z | Largest clade difference | Largest continuous R-hat | Lowest continuous ESS | Criteria |',
        '| --- | ---: | ---: | ---: | ---: | ---: | --- |',
    ]
    for case in CASES:
        run = selected[case]
        z = max(abs(v['z']) for v in run['means'].values() if v['z'] is not None)
        clade = max((abs(v['difference']) for v in run['clades'].values() if v['tested']), default=0)
        diagnostics = [d[label] for d in run['convergence'].values() for label in run['means']]
        rhats = [d['rhat'] for d in diagnostics]
        ess = [e for d in diagnostics for e in d['ess']]
        text.append(f"| {case} | {run['steps']:,} | {z:.3f} | {clade:.5f} | {max(rhats):.5f} | {min(ess):.1f} | {'Pass' if run['passes'] else 'Unresolved'} |")
    text += ['', 'These are the last batches required by the protocol. Earlier batches remain',
             'archived below; batches of different lengths are not pooled. The clade maximum',
             'excludes the imposed T1–T19 group. Each case has four chains per engine.', '',
             '| Batch | Means agree | Clades agree | Tree means agree | Convergence gates | Clade precision |',
             '| --- | --- | --- | --- | --- | --- |']
    for run in all_runs:
        flags = ['yes' if run[k] else 'no' for k in ['all_means_agree', 'all_clades_agree', 'all_tree_means_agree', 'convergence_passes', 'clade_precision_passes']]
        text.append(f"| {run['case']}, {run['steps'] // 1000000}M | " + ' | '.join(flags) + ' |')
    text += ['', '## Trees', '',
             '| Case | Distinct topologies per chain, Rove | BEAST | Mean rooted RF, Rove | BEAST | Reference topology frequency, Rove | BEAST |',
             '| --- | ---: | ---: | ---: | ---: | ---: | ---: |']
    for case in CASES:
        run = selected[case]
        rf, ref = run['tree_means']['rooted_rf'], run['tree_means']['reference_topology']
        text.append(f"| {case} | {span(run['unique_topologies']['rove'],0)} | {span(run['unique_topologies']['beast'],0)} | {rf['rove']['mean']:.4f} | {rf['beast']['mean']:.4f} | {ref['rove']['mean']:.5f} | {ref['beast']['mean']:.5f} |")
    text += ['', 'A constant topology indicator has no measurable mixing ESS. The comparison',
             'files flag tree diagnostics with any constant chain as limited; they are',
             'not counted as convergence evidence. All chains obeyed the imposed ingroup',
             'constraint. Unconstrained reference clades and their Monte Carlo errors are',
             'listed separately in each comparison file.', '',
             '## Runtime and sampling efficiency', '',
             '| Case | Wall seconds per chain, Rove | BEAST | BEAST/Rove median wall time |',
             '| --- | ---: | ---: | ---: |']
    for case in CASES:
        run = selected[case]
        r, b = (run['wall_seconds'][e] for e in ['rove','beast'])
        ratio = statistics.median(b) / statistics.median(r)
        text.append(f'| {case} | {span(r)} | {span(b)} | {ratio:.2f} |')
    text += ['', 'The following ratios divide median ESS/second in Rove by median ESS/second',
             'in BEAST. Values above one favor Rove for that statistic. They include the',
             'cost of the different proposal schedules and tuning rules, not just pruning.', '',
             '| Case | Log-posterior density | Log likelihood | Gamma shape | Clock sigma | Tree height |',
             '| --- | ---: | ---: | ---: | ---: | ---: |']
    for case in CASES:
        run = selected[case]
        ratios = [statistics.median(run['ess_per_second']['rove'][k]) / statistics.median(run['ess_per_second']['beast'][k])
                  for k in ['posterior','likelihood','gamma_shape','clock_sigma','tree_height']]
        text.append('| ' + case + ' | ' + ' | '.join(f'{r:.2f}' for r in ratios) + ' |')
    text += ['', 'Per-chain ESS, wall times, ESS/second and their variation remain in the JSON',
             'files. A faster chain can have lower ESS/second for a parameter that it',
             'explores less efficiently.', '']
    return '\n'.join(text)


def known_tree(text):
    """The fully bifurcating, unweighted T-number Newick used by these fixtures."""
    tokens = iter(re.findall(r'T\d+|[(),;]', text))
    def node(token):
        if token != '(':
            return token
        left = node(next(tokens))
        if next(tokens) != ',':
            raise ValueError('Expected a binary tree')
        right = node(next(tokens))
        if next(tokens) != ')':
            raise ValueError('Expected a binary tree')
        return left, right
    result = node(next(tokens))
    if next(tokens) != ';':
        raise ValueError('Expected a Newick terminator')
    return result


def minimum_changes(tree, column):
    if isinstance(tree, str):
        return ({0, 1} if column[tree] == '?' else {int(column[tree])}), 0
    a, first = minimum_changes(tree[0], column)
    b, second = minimum_changes(tree[1], column)
    return (a & b, first + second) if a & b else (a | b, first + second + 1)


def character_checks():
    result = {}
    for case in CASES:
        directory = ROOT / 'fixtures/validation' / case
        matrix = read_matrix(directory / 'matrix.nex')
        tree = known_tree((directory / 'truth.newick').read_text())
        scores = [minimum_changes(tree, dict(zip(matrix, column)))[1] for column in zip(*matrix.values())]
        result[case] = {
            'minimum_changes_on_known_tree': sum(scores),
            'columns_requiring_multiple_changes': sum(n > 1 for n in scores),
            'columns_compatible_with_constant': sum(n == 0 for n in scores),
        }
    return result


def read_runs(directories):
    """Validate the complete batch list before writing any archive files."""
    runs, selected, fingerprints, seen = [], {}, set(), set()
    for directory in sorted(directories):
        record = json.loads((directory / 'comparison.json').read_text())
        metadata = json.loads((directory / 'metadata.json').read_text())
        key = record['case'], record['steps']
        if key in seen:
            raise ValueError(f'Duplicate case and run length: {key}')
        if key != (metadata['case'], metadata['steps']) or record['seeds'] != metadata['seeds']:
            raise ValueError(f'Comparison and metadata disagree: {directory}')
        if record['protocol_sha256'] != metadata['protocol_sha256']:
            raise ValueError(f'Comparison and metadata use different protocols: {directory}')
        seen.add(key)
        fingerprints.add(record['protocol_sha256'])
        runs.append((directory, record, metadata))
        case = record['case']
        if case not in selected or selected[case]['steps'] < record['steps']:
            selected[case] = record
    if len(fingerprints) != 1 or set(selected) != set(CASES):
        raise ValueError('Need all six cases under one frozen protocol')
    return runs, selected, fingerprints.pop()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--runs', type=Path, nargs='+', required=True, help='individual case run directories')
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    inputs, selected, protocol = read_runs(args.runs)
    args.output.mkdir(parents=True, exist_ok=False)
    runs, directories = [], {}
    estimates = ['case\tsteps\tstatistic\trove_mean\trove_mcse\tbeast_mean\tbeast_mcse\tz']
    for directory, record, metadata in inputs:
        case = record['case']
        directories[(case, record['steps'])] = directory
        destination = args.output / case / f"{record['steps'] // 1000000}m"
        destination.mkdir(parents=True, exist_ok=False)
        for name in ['comparison.json','summaries.json']:
            shutil.copyfile(directory / name, destination / name)
        write_json(destination / 'metadata.json', sanitize(metadata, ROOT))
        write_json(destination / 'operators.json', operator_counts(directory, metadata))
        runs.append(record)
    chains = ['case\tengine\tseed\tstatistic\tmean\tsd\tess\thpd_low\thpd_high']
    for case in CASES:
        run = selected[case]
        summaries = json.loads((directories[(case, run['steps'])] / 'summaries.json').read_text())
        for engine in ['rove', 'beast']:
            for seed, summary in zip(run['seeds'], summaries[engine]):
                for label, beast_label in STATISTICS.items():
                    source = label if engine == 'rove' else beast_label
                    divisor = 38 if engine == 'beast' and label == 'mean_branch_rate' else 1
                    stat = summary['statistics'][source]
                    row = [case, engine, seed, label]
                    for key in ['mean', 'sd', 'ess', 'hpd_low', 'hpd_high']:
                        value = stat[key]
                        row.append(value / divisor if value is not None and key != 'ess' else value)
                    chains.append('\t'.join(map(str, row)))
        for label, entry in run['means'].items():
            values = [case, run['steps'], label, entry['rove']['mean'], entry['rove']['mcse'], entry['beast']['mean'], entry['beast']['mcse'], entry['z']]
            estimates.append('\t'.join(map(str, values)))
    (args.output / 'chain-estimates.tsv').write_text('\n'.join(chains) + '\n')
    (args.output / 'estimates.tsv').write_text('\n'.join(estimates) + '\n')
    (args.output / 'tables.md').write_text(tables(selected, sorted(runs, key=lambda r:(CASES.index(r['case']),r['steps']))))
    write_json(args.output / 'character-checks.json', character_checks())
    write_json(args.output / 'archive.json', {
        'protocol_sha256': protocol,
        'analysis_python': sys.version,
        'report_script_sha256': hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
        'selected_steps': {case:selected[case]['steps'] for case in CASES},
        'all_selected_cases_pass': all(run['passes'] for run in selected.values()),
    })


if __name__ == '__main__':
    main()
