"""Explicit ready-subset coverage and immutable exclusion evidence."""
import copy
import re
from pathlib import Path

from suite.state import digest, load


def selected_parents(catalog, scope=None):
    parents = [t['id'] for t in catalog['tasks']]
    if catalog.get('task_count') != 89 or len(parents) != 89 or len(set(parents)) != 89:
        raise ValueError('selection requires the complete 89-parent catalog')
    if scope is None:
        return parents
    if (not isinstance(scope, dict) or set(scope) != {'kind', 'selected', 'excluded'}
            or scope['kind'] != 'ready_subset' or not isinstance(scope['selected'], list)
            or not scope['selected'] or not all(isinstance(p, str) for p in scope['selected'])
            or len(set(scope['selected'])) != len(scope['selected'])
            or not isinstance(scope['excluded'], list)):
        raise ValueError('invalid explicit ready-subset selection')
    selected = set(scope['selected'])
    excluded = set()
    for row in scope['excluded']:
        if (not isinstance(row, dict) or set(row) != {'parent_id', 'status', 'reason', 'evidence'}
                or not isinstance(row['parent_id'], str) or row['parent_id'] in excluded
                or row['status'] not in ('blocked', 'platform_rejected', 'stale', 'not_ready', 'calibration_failed')
                or not isinstance(row['reason'], str) or not row['reason'].strip()
                or not isinstance(row['evidence'], list) or not row['evidence']):
            raise ValueError('invalid excluded parent status or evidence')
        excluded.add(row['parent_id'])
        paths = set()
        for proof in row['evidence']:
            if not isinstance(proof, dict) or set(proof) != {'path', 'sha256'}:
                raise ValueError('invalid exclusion evidence descriptor')
            if (not isinstance(proof['path'], str) or not isinstance(proof['sha256'], str)
                    or not re.fullmatch(r'[a-f0-9]{64}', proof['sha256'])):
                raise ValueError('invalid exclusion evidence path or hash')
            path = Path(proof['path'])
            if (not path.is_absolute() or path.is_symlink() or not path.is_file()
                    or str(path) in paths or digest(path) != proof['sha256']):
                raise ValueError('missing or changed exclusion evidence')
            paths.add(str(path))
    if selected & excluded or selected | excluded != set(parents):
        raise ValueError('selected/excluded must partition all 89 parents')
    return [p for p in parents if p in selected]


def merge_execution_configs(paths):
    """Merge explicit rows without rewriting roots, recovery paths or task settings."""
    if not paths:
        raise ValueError('execution configuration required')
    merged, rows = None, {}
    for path in paths:
        value = load(path)
        if not isinstance(value, dict) or not {'historical_root', 'source_root', 'cases_root', 'tasks'} <= value.keys():
            raise ValueError('expected execution configuration, not historical runtime report')
        roots = {key: value[key] for key in ('historical_root', 'source_root', 'cases_root')}
        if any(not Path(root).is_absolute() for root in roots.values()) or not isinstance(value['tasks'], list):
            raise ValueError('invalid execution configuration')
        if merged is None:
            merged = {**roots, 'tasks': []}
        elif roots != {key: merged[key] for key in roots}:
            raise ValueError('execution configuration roots differ')
        for row in value['tasks']:
            if not isinstance(row, dict) or not isinstance(row.get('taskId'), str):
                raise ValueError('invalid execution task row')
            name = row['taskId']
            if name in rows and rows[name] != row:
                raise ValueError('conflicting duplicate execution task ID')
            if name not in rows:
                rows[name] = copy.deepcopy(row)
                merged['tasks'].append(rows[name])
    return merged


def require_selected_task(manifest, task_id):
    matches = [t for t in manifest['tasks'] if t['taskId'] == task_id]
    if len(matches) != 1:
        raise ValueError('task is not selected in this frozen experiment')
    task = matches[0]
    scope = manifest.get('scope')
    if scope is not None and task.get('parent_id') not in scope['selected']:
        raise ValueError('task is not selected in this frozen experiment')
    return task
