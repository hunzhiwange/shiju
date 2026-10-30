"""Materialize the explicitly approved A+B+C verifier variant, never the original.

The upstream task is read-only. Only the helper's cleanup/exception handling and
the displayed task identity change; inputs, solution, assertions and limits do not.
This is a task-local compatibility package, not an official readiness receipt.
"""
import argparse
import hashlib
from pathlib import Path
import shutil
import subprocess

from suite.state import atomic_json, load, tree

PARENT = 'filter-js-from-html'
VARIANT = PARENT + '-compat-cleanup-load120'
PATCHES = ('cleanup.patch', 'infrastructure.patch', 'page-alarm.patch')
IMAGE = 'alexgshaw/filter-js-from-html@sha256:92acda0f124b988036a6f426ce0bc47fac19f5efe9fc5e6ea3ea52ccb075d0a4'
ROOT = Path(__file__).resolve().parent


def materialize(source, output):
    source, output = Path(source), Path(output)
    catalog = load(ROOT.parents[2] / 'catalog.json')
    official = next(t for t in catalog['tasks'] if t['id'] == PARENT)
    expected = {n: {'sha256': v['sha256'], 'executable': v['mode'] == '100755'}
                for n, v in official['files'].items()}
    if tree(source) != expected:
        raise ValueError('upstream task identity drift')
    shutil.copytree(source, output)
    for name in PATCHES:
        subprocess.run(['patch', '--batch', '--fuzz=0', '-p1', '-i', str(ROOT / name)],
                       cwd=output, check=True, capture_output=True)
    config = output / 'task.toml'
    text = config.read_text()
    for old, new in [(f'name = "terminal-bench/{PARENT}"', f'name = "terminal-bench/{VARIANT}"'),
                     ('docker_image = "alexgshaw/filter-js-from-html:20251031"', f'docker_image = "{IMAGE}"')]:
        if text.count(old) != 1:
            raise ValueError('unexpected upstream task configuration')
        text = text.replace(old, new)
    config.write_text(text)
    actual = tree(output)
    if {n: v for n, v in actual.items() if n not in ('task.toml', 'tests/test_outputs.py')} != {
            n: v for n, v in expected.items() if n not in ('task.toml', 'tests/test_outputs.py')}:
        raise ValueError('variant exceeded authorized file scope')
    if tree(source) != expected:
        raise ValueError('upstream changed during materialization')
    return {'id': VARIANT, 'counts_as_original_official': False,
            'authorization': 'User approved A+B and then C with the original total timeout on 2026-10-30',
            'image': IMAGE, 'official_files': expected, 'execution_files': actual,
            'patches': {n: hashlib.sha256((ROOT / n).read_bytes()).hexdigest()
                        for n in PATCHES}}


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--source', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    receipt = materialize(args.source.resolve(), args.output.resolve())
    atomic_json(args.output.resolve().with_suffix('.identity.json'), receipt)
