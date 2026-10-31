#!/usr/bin/env python3
"""Run the frozen sampler budget and phase-occupancy studies; keep raw data local."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import time

from run import hardware, power_state, load_average

ROOT = Path(__file__).resolve().parents[2]
PROTOCOLS = [ROOT / 'benchmarks/convergence/PROTOCOL.md', ROOT / 'benchmarks/profile/PROTOCOL.md']
CASES = {
    'morphology90': dict(conditioning='none', steps=20_000_000, every=2_000_000, interval=1000, profile_steps=2_000_000, seed=2026103101),
    'zilla': dict(conditioning='variable', steps=5_000_000, every=1_000_000, interval=500, profile_steps=300_000, seed=2026103111),
}


def sha(path):
    h = hashlib.sha256()
    with path.open('rb') as f:
        for block in iter(lambda: f.read(1024 * 1024), b''): h.update(block)
    return h.hexdigest()


def write(path, value):
    path.write_text(json.dumps(value, indent=2, allow_nan=False) + '\n')


def capture(*args):
    return subprocess.check_output(args, cwd=ROOT, text=True).strip()


def fingerprints():
    return {str(p.relative_to(ROOT)): sha(p) for p in PROTOCOLS}


def prepare(args):
    out = args.output.resolve()
    out.mkdir(parents=True, exist_ok=False)
    (out / 'bin').mkdir()
    commands = [
        ['cargo', 'build', '--locked', '--release', '--bin', 'rove', '--example', 'convergence_budget'],
        ['cargo', 'build', '--locked', '--release', '--bin', 'rove', '--features', 'profile-sampler'],
    ]
    for i, cmd in enumerate(commands):
        with (out / f'build-{i}.log').open('w') as log:
            subprocess.run(cmd, cwd=ROOT, stdout=log, stderr=log, check=True)
        shutil.copy2(ROOT / 'target/release/rove', out / 'bin' / ('rove' if i == 0 else 'rove-profile'))
        if i == 0: shutil.copy2(ROOT / 'target/release/examples/convergence_budget', out / 'bin/convergence_budget')
    matrices = {'morphology90': ROOT / 'fixtures/benchmarks/morphology_90x250.nex', 'zilla': args.zilla.resolve()}
    if sha(matrices['zilla']) != 'fd3ed2bb5b35e9814cae9b033f1c78fc473a0a01b846fde2eddf495992f1482a':
        raise ValueError('Private input does not match the frozen protocol.')
    record = {
        'protocols': fingerprints(), 'source_commit': capture('git', 'rev-parse', 'HEAD'),
        'build_commands': commands, 'build_settings': {'opt_level': 3, 'lto': 'fat', 'codegen_units': 1,
            'target_cpu': 'Rust target default', 'rustflags': os.environ.get('RUSTFLAGS', ''),
            'release_debug': os.environ.get('CARGO_PROFILE_RELEASE_DEBUG', 'default (off)')},
        'binaries': {p.name: sha(p) for p in (out / 'bin').iterdir()},
        'rustc': capture('rustc', '-Vv'), 'hardware': hardware(),
        'inputs': {k: {'path': str(p), 'sha256': sha(p)} for k, p in matrices.items()},
        'cases': CASES, 'created_unix_seconds': time.time(),
    }
    write(out / 'metadata.json', record)
    print('Prepared binaries and fingerprints.', flush=True)


def verify(out):
    record = json.loads((out / 'metadata.json').read_text())
    if capture('git', 'status', '--porcelain', '--untracked-files=no'):
        raise ValueError('Commit tracked changes before production measurements.')
    if record['source_commit'] != capture('git', 'rev-parse', 'HEAD') or record['protocols'] != fingerprints():
        raise ValueError('Source or protocol changed since preparation.')
    for name, digest in record['binaries'].items():
        if sha(out / 'bin' / name) != digest: raise ValueError('Binary fingerprint changed.')
    for data in record['inputs'].values():
        if sha(Path(data['path'])) != data['sha256']: raise ValueError('Input fingerprint changed.')
    return record


def command(out, metadata, case, model, seed, steps, interval, output, instrumented=False):
    data = metadata['inputs'][case]
    return [str(out / 'bin' / ('rove-profile' if instrumented else 'rove')), 'sample', data['path'],
        '--output', str(output), '--model', model, '--conditioning', CASES[case]['conditioning'],
        '--gaps', 'missing', '--partitioning', 'observed', '--backend', 'neon',
        '--seed', str(seed), '--steps', str(steps), '--sample-every', str(interval),
        '--checkpoint-every', '100000', '--expect-input-sha256', data['sha256'], '--events-json']


def execute(cmd, console):
    before = {'power': power_state(), 'load': load_average(), 'unix_seconds': time.time()}
    with console.open('w') as stream:
        start = time.perf_counter()
        subprocess.run(cmd, cwd=ROOT, stdout=stream, stderr=subprocess.STDOUT, check=True)
        elapsed = time.perf_counter() - start
    return {'command': cmd, 'whole_process_seconds': elapsed, 'before': before,
            'after': {'power': power_state(), 'load': load_average(), 'unix_seconds': time.time()}}


def sample_digest(path, trees=False):
    h = hashlib.sha256()
    with path.open('rb') as stream:
        for line in stream:
            if (trees and line.lstrip().lower().startswith(b'tree ')) or (not trees and not line.startswith(b'#')):
                h.update(line)
    return h.hexdigest()


def validate_run(directory, metadata, case, steps, interval, seed):
    data = json.loads((directory / 'run.json').read_text())
    if data['status'] != 'completed' or data['completed_steps'] != steps:
        raise ValueError('Incomplete run.')
    if data['input']['sha256'] != metadata['inputs'][case]['sha256']:
        raise ValueError('Wrong matrix.')
    if data['settings']['seed'] != seed or data['settings']['sample_every'] != interval:
        raise ValueError('Wrong seed or logging interval.')
    expected = 'none (Mk)' if CASES[case]['conditioning'] == 'none' else 'variable characters (Mkv)'
    if (data['model']['conditioning'] != expected or data['model']['monophyly'] != []
            or data['model']['sample_from_prior'] or data['model']['partitioning'] != 'observed'
            or not data['settings']['tuning'] or data['input']['gaps'] != 'missing'):
        raise ValueError('Target or tuning differs from the protocol.')
    with (directory / 'trace.log').open() as stream:
        rows = (line for line in stream if line.strip() and not line.startswith('#'))
        next(rows)
        states = [int(line.split()[0]) for line in rows]
    if states != list(range(0, steps + 1, interval)): raise ValueError('Truncated trace.')
    with (directory / 'trees.nexus').open() as stream:
        last = ''
        count = 0
        for line in stream:
            if line.lstrip().lower().startswith('tree '): count += 1
            if line.strip(): last = line.strip()
    if count != len(states) or last.lower() != 'end;': raise ValueError('Truncated tree file.')
    return data


def profile(out, metadata):
    results = []
    cases = [(case, model) for case in CASES for model in ['strict', 'reference']]
    for rep in range(2):
        for case, model in (cases if rep == 0 else list(reversed(cases))):
            spec = CASES[case]
            seed = 2026103181 + rep
            pair = {}
            for profiled in ([False, True] if rep == 0 else [True, False]):
                label = f'{case}-{model}-{rep+1}-' + ('profile' if profiled else 'normal')
                directory = out / label
                cmd = command(out, metadata, case, model, seed, spec['profile_steps'], 1000, directory, profiled)
                timed = execute(cmd, out / f'{label}.log')
                record = validate_run(directory, metadata, case, spec['profile_steps'], 1000, seed)
                pair['profile' if profiled else 'normal'] = {**timed, 'run_wall_seconds': record['wall_seconds'],
                    'trace_samples_sha256': sample_digest(directory / 'trace.log'),
                    'tree_samples_sha256': sample_digest(directory / 'trees.nexus', True),
                    'run_sha256': sha(directory / 'run.json'),
                    'moves': record['moves']}
                if profiled:
                    pair['occupancy'] = json.loads((directory / 'profile.json').read_text())
                    if pair['occupancy']['samples'] < 2000: raise ValueError('Too few phase samples.')
                    if not any(m['initial_tuning'] != m['final_tuning'] for m in record['moves']):
                        raise ValueError('No evidence that the run passed its tuning delay.')
                print(f'{label}: {timed["whole_process_seconds"]:.2f}s', flush=True)
            if any(pair['normal'][k] != pair['profile'][k] for k in ['trace_samples_sha256', 'tree_samples_sha256']):
                raise ValueError('Instrumentation changed the chain trajectory.')
            total = pair['occupancy']['samples']
            shares = {k: n / total for k, n in pair['occupancy']['counts'].items()}
            kernel = shares['node_partials'] + shares['rescaling']
            pair.update(case=case, model=model, repetition=rep + 1, seed=seed,
                fractions=shares, kernel_fraction=kernel, kernel_only_ceiling=1 / (1 - kernel),
                profile_over_normal_wall_ratio=pair['profile']['whole_process_seconds'] / pair['normal']['whole_process_seconds'],
                steps_per_second=spec['profile_steps'] / pair['normal']['whole_process_seconds'])
            results.append(pair)
            write(out / 'profiles.json', results)


def budget(out, metadata):
    results = []
    for case, spec in CASES.items():
        directory = out / f'{case}-budget'
        cmd = command(out, metadata, case, 'reference', spec['seed'], spec['steps'], spec['interval'], directory)
        cmd += ['--chains', '4', '--print-every', str(spec['every'])]
        timed = execute(cmd, out / f'{case}-budget.log')
        records = [validate_run(directory / f'chain-{i+1:02}', metadata, case, spec['steps'], spec['interval'], spec['seed']+i)
                   for i in range(4)]
        checks = []
        streak = 0
        first = None
        for step in range(spec['every'], spec['steps'] + 1, spec['every']):
            check = json.loads(capture(str(out / 'bin/convergence_budget'), str(directory), str(step)))
            streak = streak + 1 if check['all_pass'] else 0
            if streak == 2 and first is None: first = step
            check['consecutive_passes'] = streak
            checks.append(check)
            print(f'{case} {step}: scalar={check["scalar_pass"]} clades={check["clade_pass"]} all={check["all_pass"]}', flush=True)
        entry = {**timed, 'case': case, 'first_sustained_pass_step': first, 'checks': checks,
            'chain_wall_seconds': [r['wall_seconds'] for r in records], 'moves': [r['moves'] for r in records],
            'files_sha256': {str(p.relative_to(directory)): sha(p) for p in directory.glob('chain-*/*')
                if p.name in ('trace.log', 'trees.nexus', 'run.json')}}
        results.append(entry)
        write(out / 'budgets.json', results)
        # The 500-taxon case starts only after the complete 90-taxon assessment.


def publish(out, destination, phase):
    metadata = json.loads((out / 'metadata.json').read_text())
    data = json.loads((out / ('profiles.json' if phase == 'profile' else 'budgets.json')).read_text())
    # Only allow-listed aggregates are read above; never copy comparison.json,
    # raw run records, alignment exports, traces, trees or clade membership maps.
    text = json.dumps({'metadata': metadata, 'results': data}, indent=2, allow_nan=False)
    for info in metadata['inputs'].values():
        text = text.replace(info['path'], '$PUBLIC_MATRIX' if 'fixtures/benchmarks/' in info['path'] else '$PRIVATE_MATRIX')
    text = text.replace(str(out), '$RUN').replace(str(ROOT), '$REPO')
    destination.mkdir(parents=True, exist_ok=True)
    (destination / 'results.json').write_text(text + '\n')


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('phase', choices=['prepare', 'profile', 'budget', 'publish-profile', 'publish-budget'])
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--zilla', type=Path)
    parser.add_argument('--report', type=Path)
    args = parser.parse_args()
    if args.phase == 'prepare':
        if args.zilla is None: parser.error('prepare needs --zilla')
        prepare(args)
    elif args.phase.startswith('publish-'):
        if args.report is None: parser.error('publish needs --report')
        publish(args.output.resolve(), args.report, args.phase.split('-')[1])
    else:
        metadata = verify(args.output.resolve())
        (profile if args.phase == 'profile' else budget)(args.output.resolve(), metadata)


if __name__ == '__main__': main()
