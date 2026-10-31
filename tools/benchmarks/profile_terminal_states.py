#!/usr/bin/env python3
"""Measure short copies of the first two terminal states of each budget study."""
import argparse
import json
import shutil
import subprocess
from pathlib import Path

from sampler_study import ROOT, capture, execute, sha, write

PROTOCOL = ROOT / 'benchmarks/profile/TERMINAL-STATES.md'


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--study', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    study, out = args.study.resolve(), args.output.resolve()
    metadata = json.loads((study / 'metadata.json').read_text())
    budgets = json.loads((study / 'budgets.json').read_text())
    if [r['case'] for r in budgets] != ['morphology90', 'zilla']:
        raise ValueError('Both frozen budget studies must finish before profiling terminal states.')
    # Checkpoint codecs are executable-local. This controlled comparison keeps
    # engine sources, dependencies and compiler fixed; only phase guards differ.
    if capture('git', 'diff', metadata['source_commit'], '--', 'src', 'Cargo.toml', 'Cargo.lock'):
        raise ValueError('Engine sources changed since the terminal states were written.')
    if capture('rustc', '-Vv') != metadata['rustc']:
        raise ValueError('Compiler changed since the original builds.')
    if capture('git', 'status', '--porcelain', '--untracked-files=no'):
        raise ValueError('Commit the supplement and tools before running it.')
    out.mkdir(parents=True, exist_ok=False)
    (out / 'bin').mkdir()
    commands = []
    for profiled in [False, True]:
        cmd = ['cargo', 'build', '--locked', '--release', '--example', 'profile_checkpoint']
        if profiled: cmd += ['--features', 'profile-sampler']
        commands.append(cmd)
        with (out / f'build-{int(profiled)}.log').open('w') as log:
            subprocess.run(cmd, cwd=ROOT, stdout=log, stderr=log, check=True)
        shutil.copy2(ROOT / 'target/release/examples/profile_checkpoint',
                     out / 'bin' / ('profile' if profiled else 'normal'))
    record = {'source_commit': capture('git', 'rev-parse', 'HEAD'),
        'parent_source_commit': metadata['source_commit'], 'protocol_sha256': sha(PROTOCOL),
        'build_commands': commands, 'binaries': {p.name: sha(p) for p in (out / 'bin').iterdir()},
        'parent_metadata_sha256': sha(study / 'metadata.json'), 'results': []}
    write(out / 'results.json', record)
    for case in ['morphology90', 'zilla']:
        spec = metadata['cases'][case]
        source = study / f'{case}-budget'
        manifest = json.loads((source / 'checkpoint.json').read_text())
        if manifest['executable_sha256'] != metadata['binaries']['rove']:
            raise ValueError('Source checkpoint does not belong to the frozen normal binary.')
        if manifest['input_sha256'] != metadata['inputs'][case]['sha256']:
            raise ValueError('Source checkpoint input differs from the protocol.')
        for index in [0, 1]:
            saved = manifest['chains'][index]
            state = source / f'chain-{index+1:02}' / saved['file']
            if saved['step'] != spec['steps'] or sha(state) != saved['sha256']:
                raise ValueError('Source state is damaged or not at the frozen cap.')
            job = {'input': metadata['inputs'][case]['path'], 'input_sha256': manifest['input_sha256'],
                'state': str(state), 'state_sha256': saved['sha256'], 'initial_step': saved['step'],
                'steps': 1_000_000 if case == 'morphology90' else 100_000,
                'conditioning': spec['conditioning']}
            job_path = out / f'{case}-{index+1}.json'
            write(job_path, job)
            pair = {'case': case, 'source_chain': index+1, 'source_seed': spec['seed']+index,
                'source_state_sha256': saved['sha256'], 'steps': job['steps']}
            for profiled in ([False, True] if index == 0 else [True, False]):
                mode = 'profile' if profiled else 'normal'
                directory = out / f'{case}-{index+1}-{mode}'
                cmd = [str(out / 'bin' / mode), str(job_path), str(directory)]
                timing = execute(cmd, out / f'{directory.name}.log')
                result = json.loads((directory / 'result.json').read_text())
                pair[mode] = {**timing, **result}
                if profiled:
                    pair['occupancy'] = json.loads((directory / 'profile.json').read_text())
                    if pair['occupancy']['samples'] < 2000:
                        raise ValueError('Too few terminal-state phase observations.')
            if pair['normal']['final_state_sha256'] != pair['profile']['final_state_sha256']:
                raise ValueError('Instrumentation changed the continuation state.')
            if sha(state) != saved['sha256']:
                raise ValueError('The original checkpoint changed during measurement.')
            pair['fractions'] = {k: n/pair['occupancy']['samples'] for k,n in pair['occupancy']['counts'].items()}
            pair['sampling_time_ratio'] = pair['profile']['sampling_seconds']/pair['normal']['sampling_seconds']
            record['results'].append(pair)
            write(out / 'results.json', record)
            print(f'{case} chain {index+1}: normal {pair["normal"]["sampling_seconds"]:.3f}s; ratio {pair["sampling_time_ratio"]:.3f}; final state identical', flush=True)


if __name__ == '__main__': main()
