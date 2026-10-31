#!/usr/bin/env python3
"""Run the frozen MC3 study; all raw outputs remain in an ignored local directory."""
import argparse
from concurrent.futures import ThreadPoolExecutor
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import time
import xml.etree.ElementTree as ET

from run import hardware, power_state, load_average
from sampler_study import sha, write, capture, validate_run

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / 'tools/beast'))
from compare_inference import (STATISTICS, CONSTRAINT, normalize, pooled, difference,
                               convergence, clade_summary)
from compare_runs import read_trace

PROTOCOL = ROOT / 'benchmarks/coupled/PROTOCOL.md'
SMALL = ['reference-mk', 'reference-mkv', 'control', 'homoplasy', 'missing', 'reduced']
LARGE = {
    'morphology90': dict(steps=20_000_000, interval=1000, every=2_000_000, pilot=1_000_000, conditioning='none'),
    'zilla': dict(steps=5_000_000, interval=500, every=1_000_000, pilot=250_000, conditioning='variable'),
}
ARMS = ['cold', 'coupled-three', 'coupled-four']
JAVA_HOME = Path('/Library/Java/JavaVirtualMachines/temurin-17.jdk/Contents/Home')


def ladder(delta):
    return [1 / (1 + i * delta) for i in range(4)]


def choose_ladder(candidates):
    """The preregistered rule; it never looks at production diagnostics."""
    eligible = []
    for delta, runs in candidates.items():
        if all(sum(r['coupling']['cold_hot_cold_round_trips_per_state']) >= 1
               and all(s['attempted'] >= 50 and s['accepted'] / s['attempted'] >= .1
                       for s in r['coupling']['swaps']) for r in runs):
            eligible.append(float(delta))
    if eligible:
        return max(eligible)
    return max(map(float, candidates), key=lambda d: (
        min(sum(r['coupling']['cold_hot_cold_round_trips_per_state']) for r in candidates[str(d)]), -d))


def jvm(out):
    packages = out / 'packages'
    jars = [ROOT / '.local/beast-reference-20261025/tools/launcher.jar', *sorted(packages.glob('*/lib/*.jar'))]
    base = [str(JAVA_HOME / 'bin/java'), '-Xms256m', '-Xmx2g', '-Xss8m',
            '-Djava.awt.headless=true', '-Duser.language=en', '-Djava.only=true',
            f'-Dbeast.user.package.dir={packages}']
    return base, os.pathsep.join(map(str, jars)), jars


def prepare(out, zilla):
    if capture('git', 'status', '--porcelain', '--untracked-files=normal'):
        raise ValueError('Commit implementation, protocol and runner before preparation.')
    if os.environ.get('RUSTFLAGS') or os.environ.get('CARGO_ENCODED_RUSTFLAGS'):
        raise ValueError('Use the default CPU target, without RUSTFLAGS.')
    (out / 'bin').mkdir()
    cmd = ['cargo', 'build', '--locked', '--release', '--bin', 'rove', '--example', 'convergence_budget']
    with (out / 'build.log').open('w') as log:
        subprocess.run(cmd, cwd=ROOT, stdout=log, stderr=log, check=True)
    shutil.copy2(ROOT / 'target/release/rove', out / 'bin/rove')
    shutil.copy2(ROOT / 'target/release/examples/convergence_budget', out / 'bin/convergence_budget')
    base, cp, jars = jvm(out)
    classes = out / 'classes'
    classes.mkdir()
    source = ROOT / 'tools/beast/ReferenceDiagnostics.java'
    subprocess.run([str(JAVA_HOME / 'bin/javac'), '--release', '17', '-cp', cp,
                    '-d', str(classes), str(source)], check=True)
    subprocess.run(base + ['-cp', f'{classes}{os.pathsep}{cp}', 'rove.reference.ReferenceDiagnostics', 'self-test'], check=True)
    inputs = {'morphology90': ROOT / 'fixtures/benchmarks/morphology_90x250.nex', 'zilla': zilla}
    inputs.update({case: ROOT / 'fixtures/validation' / case / 'matrix.nex' for case in SMALL})
    if sha(zilla) != 'fd3ed2bb5b35e9814cae9b033f1c78fc473a0a01b846fde2eddf495992f1482a':
        raise ValueError('Private input differs from protocol.')
    versions = {ET.parse(p).getroot().get('name'): ET.parse(p).getroot().get('version')
                for p in (out / 'packages').glob('*/version.xml')}
    expected = {'BEAST.base':'2.7.8', 'BEAST.app':'2.7.8', 'BEASTLabs':'2.0.3',
                'MM':'1.2.1', 'ORC':'1.2.1', 'CoupledMCMC':'1.2.2'}
    if versions != expected:
        raise ValueError(f'Unexpected reference packages: {versions}')
    record = {'source_commit': capture('git', 'rev-parse', 'HEAD'), 'protocol_sha256':sha(PROTOCOL),
              'binaries':{p.name:sha(p) for p in (out/'bin').iterdir()},
              'inputs':{k:{'path':str(p), 'sha256':sha(p)} for k,p in inputs.items()},
              'packages':versions, 'jars':{str(p):sha(p) for p in jars},
              'archive_sha256':sha(out/'downloads/CoupledMCMC.v1.2.2.zip'),
              'reference_source_sha256':sha(source), 'hardware':hardware(), 'rustc':capture('rustc','-Vv'),
              'java_version':subprocess.run(base[:1]+['-version'],capture_output=True,text=True,check=True).stderr,
              'build_command':cmd, 'build_settings':{'opt_level':3,'lto':'fat','codegen_units':1,'target_cpu':'default','features':[]},
              'created_unix_seconds':time.time()}
    write(out/'metadata.json', record)


def verify(out):
    record = json.loads((out/'metadata.json').read_text())
    if capture('git','status','--porcelain','--untracked-files=normal'):
        raise ValueError('Production requires a clean checkout.')
    if capture('git','rev-parse','HEAD') != record['source_commit'] or sha(PROTOCOL) != record['protocol_sha256']:
        raise ValueError('Source or protocol changed.')
    for name,digest in record['binaries'].items():
        if sha(out/'bin'/name) != digest: raise ValueError('Binary changed.')
    for v in record['inputs'].values():
        if sha(Path(v['path'])) != v['sha256']: raise ValueError('Input changed.')
    for path,digest in record['jars'].items():
        if sha(Path(path)) != digest: raise ValueError('Reference JAR changed.')
    return record


def command(out, metadata, case, directory, seed, steps, interval, delta=None):
    data = metadata['inputs'][case]
    conditioning = LARGE[case]['conditioning'] if case in LARGE else ('variable' if case == 'reference-mkv' else 'none')
    cmd = [str(out/'bin/rove'), 'sample' if delta is None else 'coupled', data['path'],
           '--output',str(directory),'--seed',str(seed),'--steps',str(steps),'--sample-every',str(interval),
           '--model','reference','--conditioning',conditioning,'--gaps','missing',
           '--partitioning','observed','--backend','neon','--checkpoint-every','100000',
           '--expect-input-sha256',data['sha256'],'--print-every',str(interval),'--events-json']
    if case in SMALL:
        cmd += ['--monophyly', ','.join(f'T{i}' for i in range(1,20))]
    if delta is not None:
        cmd += ['--likelihood-powers', ','.join(map(repr,ladder(delta))), '--swap-every','1000']
    return cmd


def run_group(directory, jobs, prefix_every=0):
    """Each job is (command, cwd). Readers drain pipes and timestamp prefix events."""
    directory.mkdir(parents=True, exist_ok=False)
    before = {'unix_seconds':time.time(),'power':power_state(),'load':load_average()}
    write(directory/'schedule.json', {'jobs':[(cmd,str(cwd)) for cmd,cwd in jobs], 'before':before})
    start = time.perf_counter()

    def member(index, job):
        cmd,cwd = job
        cwd.mkdir(parents=True, exist_ok=True)
        events = {}
        began = time.perf_counter()-start
        with (directory/f'chain-{index+1:02}.console.log').open('w') as log:
            child = subprocess.Popen(cmd,cwd=cwd,stdout=subprocess.PIPE,stderr=subprocess.STDOUT,text=True)
            try:
                for line in child.stdout:
                    log.write(line)
                    if prefix_every and line.startswith('{'):
                        event=json.loads(line)
                        step=event.get('step',0)
                        if step and step % prefix_every == 0:
                            events.setdefault(str(step),time.perf_counter()-start)
                code=child.wait()
            finally:
                child.stdout.close()
                if child.poll() is None:
                    child.terminate()
                    child.wait()
        return {'command':cmd,'returncode':code,'start_group_seconds':began,
                'end_group_seconds':time.perf_counter()-start,'prefix_group_seconds':events}

    with ThreadPoolExecutor(max_workers=len(jobs)) as pool:
        runs=list(pool.map(lambda pair:member(*pair),enumerate(jobs)))
    record={'before':before,'wall_seconds':time.perf_counter()-start,'runs':runs,
            'after':{'unix_seconds':time.time(),'power':power_state(),'load':load_average()}}
    write(directory/'timing.json',record)
    if any(r['returncode'] for r in runs):
        raise ValueError(f'Failed group: {directory}; preserve the outputs and inspect console logs.')
    print(f'{directory.name}: {record["wall_seconds"]:.2f}s',flush=True)
    return record


def beast_xml(case, output, steps):
    model=ET.parse(ROOT/'fixtures/validation'/case/'beast.xml')
    model.getroot().find('run').attrib.update(spec='coupledMCMC.CoupledMCMC',chainLength=str(steps),
        chains='4',deltaTemperature='0.1',optimise='false',neighbourSwapping='true',
        resampleEvery='1000',heatLikelihoodOnly='true',logHeatedChains='false')
    for logger in model.getroot().findall('.//logger'):
        logger.set('logEvery',str(100000 if logger.get('id') == 'screenlog' else 1000))
    model.write(output,encoding='UTF-8',xml_declaration=True)


def validate_small(group, arm, steps, xml_directory):
    artifacts=[]
    for j in range(4):
        directory=group/f'chain-{j+1:02}'
        trace=directory/('analysis.log' if arm=='beast' else 'trace.log')
        values=read_trace(trace,0)
        expected=list(range(0,steps+1,1000))
        if values['Sample'] != expected:
            raise ValueError('Small-fixture trace is incomplete.')
        states=[]
        with (directory/'trees.nexus').open() as trees:
            for line in trees:
                if line.lstrip().lower().startswith('tree '):
                    states.append(int(line.split()[1].removeprefix('STATE_')))
        if states != expected:
            raise ValueError('Small-fixture tree states differ from the trace.')
        if arm!='beast':
            record=json.loads((directory/'run.json').read_text())
            if record['status']!='completed' or record['completed_steps']!=steps:
                raise ValueError('Small-fixture sampler did not finish.')
        artifacts.append({f.name:sha(f) for f in directory.iterdir() if f.is_file()})
    write(group/'artifacts.json',{'chains':artifacts,
        'xml':{p.name:sha(p) for p in xml_directory.glob('*.xml')} if arm=='beast' else {}})


def small(out, metadata):
    base,cp,_=jvm(out)
    arms=['cold','coupled','beast']
    for k,case in enumerate(SMALL):
        steps=40_000_000 if case == 'reference-mkv' else 10_000_000
        for arm in arms[k%3:]+arms[:k%3]:
            group=out/'small'/case/arm
            jobs=[]
            for j in range(4):
                seed=710000+100*k+10*j
                directory=group/f'chain-{j+1:02}'
                if arm == 'beast':
                    # Prepare XMLs outside the timed group directory.
                    configs=out/'xml'/case
                    configs.mkdir(parents=True,exist_ok=True)
                    xml=configs/f'{seed}.xml'
                    beast_xml(case,xml,steps)
                    cmd=base+['-cp',cp,'beastfx.app.beast.BeastMCMC','-batch','-seed',str(seed),
                              '-threads','4',str(xml)]
                    # Explicit filenames avoid BEAST deriving them from XML location.
                    model=ET.parse(xml)
                    model.getroot().find('.//logger[@id="tracelog"]').set('fileName','analysis.log')
                    model.write(xml,encoding='UTF-8',xml_declaration=True)
                    jobs.append((cmd,directory))
                else:
                    jobs.append((command(out,metadata,case,directory,seed,steps,1000,
                                         None if arm == 'cold' else .1),ROOT))
            run_group(group,jobs)
            validate_small(group,arm,steps,out/'xml'/case)


def pilot(out, metadata):
    results={}
    for k,(case,spec) in enumerate(LARGE.items()):
        candidates={}
        for delta in [.01,.05,.2]:
            group=out/'pilot'/case/str(delta)
            jobs=[(command(out,metadata,case,group/f'chain-{j+1:02}',711000+100*k+10*j,
                           spec['pilot'],spec['interval'],delta),ROOT) for j in range(2)]
            run_group(group,jobs)
            candidates[str(delta)]=[json.loads((group/f'chain-{j+1:02}/run.json').read_text()) for j in range(2)]
        results[case]={'delta':choose_ladder(candidates),'candidates':candidates}
        write(out/'pilot.json',results)
        print(f'{case}: selected delta {results[case]["delta"]}',flush=True)


def large(out, metadata):
    selected=json.loads((out/'pilot.json').read_text())
    for k,(case,spec) in enumerate(LARGE.items()):
        for arm in (ARMS if k == 0 else list(reversed(ARMS))):
            a=ARMS.index(arm)
            n=3 if arm == 'coupled-three' else 4
            group=out/'large'/case/arm
            jobs=[(command(out,metadata,case,group/f'chain-{j+1:02}',712000+1000*k+100*a+10*j,
                           spec['steps'],spec['interval'],None if a == 0 else selected[case]['delta']),ROOT)
                  for j in range(n)]
            run_group(group,jobs,spec['every'])
            for j in range(n):
                validate_run(group/f'chain-{j+1:02}',metadata,case,spec['steps'],spec['interval'],712000+1000*k+100*a+10*j)


def fixture_comparison(case, groups, first, second):
    stats, traces={},{}
    for arm in [first,second]:
        stats[arm],traces[arm]=[],[]
        for run in groups[arm]:
            s,t=normalize(run['summary'],run['trace'],'beast' if arm=='beast' else 'rove')
            stats[arm].append(s); traces[arm].append(t)
    means={}; checks={arm:{} for arm in [first,second]}
    for label in STATISTICS:
        p={arm:pooled([s[label] for s in stats[arm]]) for arm in [first,second]}
        means[label]={**p,**difference(p[first],p[second])}
        for arm in [first,second]:
            checks[arm][label]=convergence([s[label] for s in stats[arm]],[t[label] for t in traces[arm]])
            # Rust also reports whether its finite lag window was exhausted.
            name = STATISTICS[label] if arm == 'beast' else label
            lag=[next(c for c in r['rust']['columns'] if c['name']==name)['lag_limit_reached'] for r in groups[arm]]
            checks[arm][label]['lag_limit_reached']=lag
            checks[arm][label]['passes'] &= not any(lag)
    summaries={a:[r['summary'] for r in groups[a]] for a in [first,second]}
    clades={}
    for key in set().union(*(s['observed_clade_frequencies'] for ss in summaries.values() for s in ss)):
        p={arm:clade_summary(summaries[arm],key) for arm in [first,second]}
        if key == CONSTRAINT:
            if any(f!=1 for a in p.values() for f in a['chain_frequencies']):
                raise ValueError('Ingroup constraint violated.')
        elif max(v['mean'] for v in p.values()) >= .01:
            clades[key]={**p,'agrees':abs(p[first]['mean']-p[second]['mean'])<=.05,
                         'precision_passes':max(v['mcse'] for v in p.values())<=.0125}
    tree={arm:{label:convergence([s['tree_statistics'][label] for s in summaries[arm]],
                               [s['tree_traces'][label] for s in summaries[arm]],tree=True)
               for label in ['rooted_rf','reference_topology']} for arm in [first,second]}
    passed=(all(v['agrees'] for v in means.values()) and all(c['passes'] for cs in checks.values() for c in cs.values())
            and all(c['agrees'] and c['precision_passes'] for c in clades.values()))
    return {'case':case,'first':first,'second':second,'passes':passed,'means':means,'convergence':checks,
            'clades':clades,'tree_diagnostics':tree,
            'unique_topologies':{a:[s['unique_topologies'] for s in ss] for a,ss in summaries.items()}}


def analyze_small(out):
    base,cp,_=jvm(out)
    diagnostics=base+['-cp',f'{out / "classes"}{os.pathsep}{cp}','rove.reference.ReferenceDiagnostics']
    result={}
    for case in SMALL:
        groups={}
        for arm in ['cold','coupled','beast']:
            groups[arm]=[]
            for j in range(4):
                directory=out/'small'/case/arm/f'chain-{j+1:02}'
                trace=directory/('analysis.log' if arm == 'beast' else 'trace.log')
                summary=directory/'summary.json'
                with (directory/'summary.console.log').open('w') as console:
                    subprocess.run(diagnostics+['inference-summary',str(trace),str(directory/'trees.nexus'),
                        str(ROOT/'fixtures/validation'/case/'truth.newick'),'10',str(summary)],
                        stdout=console,stderr=console,check=True)
                run={'summary':json.loads(summary.read_text()),'trace':read_trace(trace,10)}
                run['rust']=json.loads(subprocess.check_output([str(out/'bin/rove'),'diagnose',str(trace),'--burnin-percent','10']))
                if run['summary']['retained_samples'] != len(run['trace']['Sample']):
                    raise ValueError('Trace/tree retention differs.')
                groups[arm].append(run)
        result[case]={a:fixture_comparison(case,groups,'coupled',a) for a in ['cold','beast']}
        write(out/'small-results.json',result)


def analyze_large(out):
    results={}
    for case,spec in LARGE.items():
        results[case]={}
        for arm in ARMS:
            group=out/'large'/case/arm
            timing=json.loads((group/'timing.json').read_text())
            n=3 if arm=='coupled-three' else 4
            checks=[]; streak=0; first=None
            for step in range(spec['every'],spec['steps']+1,spec['every']):
                check=json.loads(subprocess.check_output([str(out/'bin/convergence_budget'),str(group),str(step),str(n)]))
                check['group_wall_seconds']=max(r['prefix_group_seconds'][str(step)] for r in timing['runs'])
                streak=streak+1 if check['all_pass'] else 0
                if streak==2 and first is None: first={'step':step,'seconds':check['group_wall_seconds']}
                checks.append(check)
                write(group/f'prefix-{step}.json',check)
            records=[json.loads((group/f'chain-{j+1:02}/run.json').read_text()) for j in range(n)]
            results[case][arm]={'wall_seconds':timing['wall_seconds'],'first_sustained_pass':first,
                'checks':checks,'moves':[r['moves'] for r in records],
                'coupling':[r.get('coupling') for r in records],
                'output_sha256':[{f:sha(group/f'chain-{j+1:02}'/f) for f in ['run.json','trace.log','trees.nexus']} for j in range(n)]}
            write(out/'large-results.json',results)


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('phase',choices=['prepare','small','pilot','large','analyze-small','analyze-large'])
    parser.add_argument('--output',type=Path,required=True)
    parser.add_argument('--zilla',type=Path)
    args=parser.parse_args(); out=args.output.resolve()
    if args.phase=='prepare':
        if args.zilla is None: parser.error('--zilla is required for preparation')
        prepare(out,args.zilla.resolve()); return
    metadata=verify(out)
    if args.phase=='small': small(out,metadata)
    elif args.phase=='pilot': pilot(out,metadata)
    elif args.phase=='large': large(out,metadata)
    elif args.phase=='analyze-small': analyze_small(out)
    elif args.phase=='analyze-large': analyze_large(out)


if __name__=='__main__':
    main()
