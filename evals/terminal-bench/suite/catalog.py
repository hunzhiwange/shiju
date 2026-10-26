"""Verify every task byte against Git before emitting public metadata only."""
import argparse
import hashlib
from importlib.metadata import version
import json
from pathlib import Path
import subprocess
import tomllib

from harbor.models.task.task import strip_canary

REPOSITORY = 'https://github.com/harbor-framework/terminal-bench-2-1'


def validate_ids(ids):
    if len(ids) != len(set(ids)):
        raise ValueError('duplicate task IDs')


def git(source, *args):
    return subprocess.check_output(['git', '-C', str(source), *args])


def sha256(data):
    return hashlib.sha256(data).hexdigest()


def build_catalog(source, revision):
    if version("harbor") != "0.24.0":
        raise ValueError("catalog requires Harbor 0.24.0")
    source = Path(source)
    head = git(source, 'rev-parse', 'HEAD').decode().strip()
    if len(revision) != 40 or head != revision:
        raise ValueError('source HEAD differs from full frozen revision')
    entries = {}
    for record in git(source, 'ls-tree', '-rz', revision, '--', 'tasks').split(b'\0'):
        if not record:
            continue
        metadata, name = record.split(b'\t', 1)
        mode, kind, oid = metadata.decode().split()
        path = name.decode()
        if kind != 'blob' or mode not in ('100644', '100755'):
            raise ValueError('unsupported task entry: ' + path)
        entries[path] = (mode, oid)
    actual = {p.relative_to(source).as_posix() for p in (source / 'tasks').rglob('*')
              if p.is_file() or p.is_symlink()}
    if actual != set(entries):
        raise ValueError('task tree has missing or extra files')
    verified_files = {}
    for path, (mode, oid) in sorted(entries.items()):
        local = source / path
        if local.is_symlink():
            raise ValueError('task symlink: ' + path)
        if bool(local.stat().st_mode & 0o111) != (mode == "100755"):
            raise ValueError("file mode differs from frozen Git tree: " + path)
        data = local.read_bytes()
        blob = hashlib.sha1(b'blob ' + str(len(data)).encode() + b'\0' + data).hexdigest()
        if blob != oid:
            raise ValueError('file differs from frozen Git blob: ' + path)
        verified_files[path] = {'sha256': sha256(data), 'git_blob': oid, 'mode': mode,
                                'bytes': len(data)}
    task_ids = sorted({Path(p).parts[1] for p in entries if len(Path(p).parts) > 2})
    validate_ids(task_ids)
    if not task_ids:
        raise ValueError('empty task tree')
    root_files = {Path(p).name: metadata for p, metadata in verified_files.items()
                  if len(Path(p).parts) == 2}
    tasks = []
    for task_id in task_ids:
        prefix = f'tasks/{task_id}/'
        files = {path[len(prefix):]: metadata for path, metadata in verified_files.items()
                 if path.startswith(prefix)}
        for required in ('instruction.md', 'task.toml', 'tests/test.sh', 'solution/solve.sh'):
            if required not in files:
                raise ValueError(f'{task_id}: missing {required}')
        if not any(p.startswith('environment/') for p in files):
            raise ValueError(task_id + ': missing environment')
        task_dir = source / 'tasks' / task_id
        config = tomllib.loads((task_dir / 'task.toml').read_text())
        instruction = (task_dir / 'instruction.md').read_text()
        tasks.append({
            'id': task_id,
            'url': f'{REPOSITORY}/tree/{revision}/tasks/{task_id}',
            'tree_sha': git(source, 'rev-parse', f'{revision}:tasks/{task_id}').decode().strip(),
            'category': config['metadata']['category'],
            'agent': config['agent'], 'verifier': config['verifier'],
            'environment': config['environment'],
            'instruction_sha256': files['instruction.md']['sha256'],
            'effective_instruction_sha256': sha256(strip_canary(instruction).encode()),
            'files': files,
        })
    return {'schema_version': 1, 'repository': REPOSITORY, 'revision': revision,
            'tasks_tree_sha': git(source, 'rev-parse', f'{revision}:tasks').decode().strip(),
            'task_count': len(tasks), 'categories': sorted({t['category'] for t in tasks}),
            'harbor_version': '0.24.0', 'root_files': root_files, 'tasks': tasks}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--source', required=True, type=Path)
    parser.add_argument('--revision', required=True)
    parser.add_argument('--output', required=True, type=Path)
    args = parser.parse_args()
    catalog = build_catalog(args.source, args.revision)
    args.output.write_text(json.dumps(catalog, indent=2, sort_keys=True) + '\n')
    print(f"Verified {catalog['task_count']} tasks, {len(catalog['categories'])} categories")


if __name__ == '__main__':
    main()
