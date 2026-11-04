#!/usr/bin/env python3
"""Timestamp and summarize serial cargo-dist 0.30.1 builds; never publish."""
import argparse
from collections import Counter
from contextlib import contextmanager
from datetime import datetime, timezone
import json
import os
from pathlib import Path
import platform
import re
import signal
import subprocess
import sys
import time

ROOT = Path(__file__).resolve().parents[1]
PACKAGES = ('vw-acp', 'vw-cli', 'vw-desktop', 'vw-webview')
BINS = ('acp', 'shiju', 'vibe-agent', 'shi-ju', 'vw-webview')
TARGETS = ('x86_64-apple-darwin', 'aarch64-pc-windows-msvc',
           'aarch64-apple-darwin', 'x86_64-pc-windows-msvc',
           'x86_64-unknown-linux-gnu', 'aarch64-unknown-linux-gnu')
ANSI = re.compile(r'\x1b\[[0-9;]*m')
START = re.compile(r'^building (\S+) target.*using cargo profile (\S+) --package=([^\s)]+)\)')


def summarize(log, exit_code):
    builds, errors, totals = [], [], {}
    active = None
    counts = Counter()
    previous = None
    for number, raw in enumerate(log.splitlines(), 1):
        raw = ANSI.sub('', raw)
        try:
            stamp, line = raw.split(' ', 1)
            now = datetime.fromisoformat(stamp.replace('Z', '+00:00'))
            if now.tzinfo is None:
                raise ValueError('timestamp needs timezone')
        except ValueError:
            errors.append(f'line {number}: missing or invalid timestamp')
            continue
        if previous is not None and now < previous:
            errors.append(f'line {number}: timestamps moved backwards')
        previous = now
        match = START.match(line)
        if match:
            if active is not None:
                errors.append('build missing Finished record')
            target, profile, package = match.groups()
            counts[package] += 1
            active = dict(package=package, target=target, profile=profile,
                          attempt=counts[package], start=stamp, seconds=None,
                          post_compile_seconds=None, command=(
                              f'cargo build --profile {profile} --message-format=json-render-diagnostics '
                              f'--target {target} --all-features --package {package}'))
            builds.append(active)
            begin, last_compile = now, None
        elif line.lstrip().startswith('Compiling ') and active is not None:
            last_compile = now
        elif re.match(r'\s*Finished [`\']', line):
            if active is None:
                errors.append(f'line {number}: unmatched Finished record')
                continue
            duration = (now - begin).total_seconds()
            if duration < 0:
                errors.append(f'line {number}: negative duration')
            else:
                active['seconds'] = duration
                active['post_compile_seconds'] = (
                    (now - last_compile).total_seconds() if last_compile else None)
                totals[active['package']] = totals.get(active['package'], 0) + duration
            active = None
        elif re.match(r'\s*error(?:\[|:)', line):
            errors.append(f'line {number}: compiler/build error')
    if active is not None:
        errors.append('build missing Finished record')
    if not builds:
        errors.append('no package builds found')
    return dict(success=not errors and exit_code == 0, exit_code=exit_code,
                builds=builds, package_seconds=totals, errors=errors,
                tail_definition='Elapsed after last Compiling log until Finished; not pure linking time')


def write_json(path, value):
    path.write_text(json.dumps(value, indent=2, ensure_ascii=False) + '\n', encoding='utf-8')


@contextmanager
def slot():
    # A workflow holds this lock across cache restore, build, packaging and save.
    external = os.environ.get('RELEASE_CARGO_SLOT')
    if external is not None:
        if external not in {str(n) for n in range(1, 11)}:
            raise RuntimeError('Invalid workflow slot number')
        target = ROOT / f'target/rust-slot-{external}'
        owner = ROOT / f'target/rust-slot-locks/slot-{external}.lock/owner'
        session = os.environ.get('GITHUB_RUN_ID')
        if (not session or not owner.is_file()
                or f'session={session}' not in owner.read_text().split()
                or Path(os.environ.get('CARGO_TARGET_DIR', '')).resolve() != target.resolve()):
            raise RuntimeError('Invalid workflow slot owner or target directory')
        yield int(external), target
        return
    locks = ROOT / 'target/rust-slot-locks'
    locks.mkdir(parents=True, exist_ok=True)
    while True:
        for number in range(1, 11):
            lock = locks / f'slot-{number}.lock'
            try:
                lock.mkdir()
            except FileExistsError:
                continue
            break
        else:
            time.sleep(5)
            continue
        break
    try:
        write_json(lock / 'owner.json', dict(plan='发布构建提速', task=1,
                   session=os.environ.get('GITHUB_RUN_ID', 'local-benchmark'), pid=os.getpid()))
        target = ROOT / f'target/rust-slot-{number}'
        os.environ['CARGO_TARGET_DIR'] = str(target)
        yield number, target
    finally:
        (lock / 'owner.json').unlink(missing_ok=True)
        lock.rmdir()


def stop_process(child):
    """Wait for the whole build process tree before releasing its Cargo slot."""
    if os.name == 'nt':
        subprocess.run(['taskkill', '/PID', str(child.pid), '/T', '/F'],
                       stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        child.wait()
        return
    try:
        os.killpg(child.pid, signal.SIGTERM)
    except ProcessLookupError:
        pass
    child.wait()
    deadline = time.monotonic() + 10
    while True:
        try:
            os.killpg(child.pid, 0)
            if time.monotonic() >= deadline:
                os.killpg(child.pid, signal.SIGKILL)
        except ProcessLookupError:
            break
        time.sleep(0.1)


def capture(command):
    with subprocess.Popen(command, cwd=ROOT, stdout=subprocess.PIPE, stderr=subprocess.PIPE,
                          text=True, encoding='utf-8', errors='replace',
                          start_new_session=os.name != 'nt') as child:
        try:
            stdout, stderr = child.communicate()
        except BaseException:
            stop_process(child)
            raise
        return dict(command=command, exit_code=child.returncode, output=stdout, stderr=stderr)


def sizes(directory):
    return {str(p.relative_to(ROOT)): p.stat().st_size for p in directory.rglob('*')
            if p.is_file()} if directory.exists() else {}


def run_dist(target, output):
    command = ['dist', 'build', '--print=linkage', '--output-format=json',
               '--artifacts=local', '--target', target]
    write_json(output / 'command.json', command)
    # stdout remains valid dist JSON; stderr holds Cargo and dist progress.
    with (output / 'dist-manifest.json').open('w', encoding='utf-8') as manifest, \
            (output / 'build.log').open('w', encoding='utf-8') as log:
        with subprocess.Popen(command, cwd=ROOT, stdout=manifest, stderr=subprocess.PIPE,
                              text=True, encoding='utf-8', errors='replace', bufsize=1,
                              start_new_session=os.name != 'nt') as child:
            try:
                for line in child.stderr:
                    stamp = datetime.now(timezone.utc).isoformat(timespec='microseconds')
                    record = f'{stamp} {line}'
                    log.write(record)
                    log.flush()
                    print(record, end='', flush=True)
                return child.wait()
            except BaseException:
                stop_process(child)
                raise


def benchmark(target, output):
    output.mkdir(parents=True, exist_ok=False)
    metadata = dict(target=target, cache_mode=os.environ.get('RELEASE_CACHE_MODE', 'existing-slot-no-restore'),
                    cache_key=os.environ.get('RELEASE_CACHE_KEY'),
                    cache_matched_key=os.environ.get('RELEASE_CACHE_MATCHED_KEY'),
                    cache_hit=os.environ.get('RELEASE_CACHE_HIT'),
                    os=platform.platform(), architecture=platform.machine(),
                    logical_cpus=os.cpu_count(), runner_image=os.environ.get('ImageOS'),
                    runner_image_version=os.environ.get('ImageVersion'))
    with slot() as (number, target_dir):
        metadata.update(slot=number, CARGO_TARGET_DIR=str(target_dir),
                        configured_build_dir=str(ROOT / 'target'),
                        initial_slot_files=len(sizes(target_dir)))
        write_json(output / 'environment.json', metadata)
        try:
            for name, command in {
                'sha': ['git', 'rev-parse', 'HEAD'], 'rustc': ['rustc', '-Vv'],
                'cargo': ['cargo', '-V'], 'dist': ['dist', '--version'],
                'dist_build_help': ['dist', 'build', '--help'],
                'cargo_metadata': ['cargo', 'metadata', '--no-deps', '--format-version=1'],
                'dist_plan': ['dist', 'plan', '--output-format=json'],
            }.items():
                result = capture(command)
                write_json(output / f'{name}.json', result)
                if result['exit_code']:
                    raise RuntimeError(f'{name} failed: exit {result["exit_code"]}')
                if name == 'dist' and result['output'].strip() != 'cargo-dist 0.30.1':
                    raise RuntimeError('benchmark requires cargo-dist 0.30.1')
            if platform.system() == 'Darwin':
                metadata['hardware'] = capture(['sysctl', 'hw.model', 'machdep.cpu.brand_string', 'hw.memsize'])
            elif platform.system() == 'Windows':
                metadata['hardware'] = capture(['powershell', '-NoProfile', '-Command',
                    'Get-CimInstance Win32_Processor | Select-Object Name,NumberOfCores; '
                    'Get-CimInstance Win32_ComputerSystem | Select-Object TotalPhysicalMemory'])
            else:
                metadata['hardware'] = capture(['uname', '-a'])
            if metadata['hardware']['exit_code']:
                raise RuntimeError('hardware inventory failed')
            metadata['cargo_config'] = (ROOT / '.cargo/config.toml').read_text()
            cargo_metadata = json.loads(json.loads((output / 'cargo_metadata.json').read_text())['output'])
            metadata['cargo_reported_directories'] = {
                k: v for k, v in cargo_metadata.items() if k.endswith('directory')}
            if Path(cargo_metadata['target_directory']).resolve() != target_dir:
                raise RuntimeError('Cargo target directory bypasses the shared slot')
            build_dir = cargo_metadata.get('build_directory')
            if build_dir and Path(build_dir).resolve() != ROOT / 'target':
                raise RuntimeError('Cargo build directory differs from the repository configuration')
            plan = json.loads(json.loads((output / 'dist_plan.json').read_text())['output'])
            metadata['dist_planned_bins'] = sorted({
                asset['name'] for artifact in plan['artifacts'].values()
                if target in artifact.get('target_triples', [])
                for asset in artifact.get('assets', []) if asset.get('kind') == 'executable'})
            if set(metadata['dist_planned_bins']) != set(BINS):
                raise RuntimeError('dist plan binary selection differs from benchmark expectations')
            metadata['bins'] = {p['name']: [t['name'] for t in p['targets'] if 'bin' in t['kind']]
                                for p in cargo_metadata['packages'] if p['name'] in PACKAGES}
            started = time.monotonic()
            code = run_dist(target, output)
            metadata.update(exit_code=code, elapsed_seconds=time.monotonic() - started)
            report = summarize((output / 'build.log').read_text(encoding='utf-8'), code)
            missing = set(PACKAGES) - set(report['package_seconds'])
            if missing:
                report['errors'].append(f'missing packages: {sorted(missing)}')
                report['success'] = False
            suffix = '.exe' if 'windows' in target else ''
            metadata['binary_bytes'] = {}
            for binary in BINS:
                path = target_dir / target / 'dist' / (binary + suffix)
                if path.is_file():
                    metadata['binary_bytes'][binary] = path.stat().st_size
                else:
                    report['errors'].append(f'missing binary: {binary}')
                    report['success'] = False
            write_json(output / 'summary.json', report)
            return code if code else (0 if report['success'] else 1)
        except BaseException as error:
            metadata['error'] = str(error)
            metadata.setdefault('exit_code', 1)
            raise
        finally:
            if not (output / 'summary.json').exists():
                report = summarize((output / 'build.log').read_text(encoding='utf-8')
                                   if (output / 'build.log').exists() else '',
                                   metadata.get('exit_code', 1))
                report['success'] = False
                report['errors'].append(metadata.get('error', 'benchmark interrupted'))
                write_json(output / 'summary.json', report)
            metadata['artifact_bytes'] = {
                **sizes(ROOT / 'target/distrib'), **sizes(target_dir / 'distrib')}
            metadata['intermediate_directories'] = {
                str(p): p.exists() for p in (ROOT / 'target', target_dir,
                                              ROOT / 'target' / target / 'dist')}
            write_json(output / 'environment.json', metadata)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('log', nargs='?', type=Path)
    parser.add_argument('--exit-code', type=int, default=0)
    parser.add_argument('--run', choices=TARGETS)
    parser.add_argument('--output', type=Path)
    args = parser.parse_args()
    if args.run:
        if not args.output:
            parser.error('--run requires a new --output directory')
        return benchmark(args.run, args.output.resolve())
    if not args.log:
        parser.error('provide a timestamped log or --run')
    report = summarize(args.log.read_text(encoding='utf-8'), args.exit_code)
    print(json.dumps(report, indent=2))
    return 0 if report['success'] else 1


if __name__ == '__main__':
    def interrupted(signum, frame):
        raise KeyboardInterrupt(f'interrupted by signal {signum}')
    signal.signal(signal.SIGTERM, interrupted)
    sys.exit(main())
