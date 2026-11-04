"""Collect immutable evidence references; missing evidence and waivers never pass."""
import argparse
from collections import defaultdict
import hashlib
import json
import os
from pathlib import Path
import re
import sys
import tempfile

from check_evidence import (array, indexed_items, load, object_value,
                            require, tasks, text, unique_object, validate)


def summarize(baseline_path, evidence_dir, output_path):
    baseline_path = Path(baseline_path).resolve()
    evidence_dir = Path(evidence_dir).resolve()
    output_path = Path(output_path).resolve()
    require(output_path != baseline_path, 'output cannot overwrite baseline')
    require(output_path != evidence_dir and evidence_dir not in output_path.parents,
            'output must be outside source evidence directory')
    baseline = load(baseline_path)
    items = indexed_items(baseline, 'baseline')
    require(items, 'baseline: no capabilities')
    sources = object_value(baseline.get('sources'), 'baseline.sources')
    require(sources, 'baseline.sources: cannot be empty')
    for key, source in sources.items():
        sha = object_value(source, key).get('sha256')
        require(isinstance(sha, str) and re.fullmatch(r'[0-9a-f]{64}', sha),
                f'{key}: invalid sha256')
    for identifier, item in items.items():
        tasks(item.get('tasks'), f'{identifier}.tasks')
        references = array(item.get('sources'), f'{identifier}.sources')
        require(references, f'{identifier}: missing sources')
        for ref in references:
            ref = object_value(ref, identifier)
            key = ref.get('source')
            require(isinstance(key, str) and key in sources, f'{identifier}: unknown source')
            text(ref.get('symbol'), identifier)
            require(ref.get('sha256') == sources[key]['sha256'],
                    f'{identifier}: source hash mismatch')
        require(type(item.get('in_scope', item.get('basis') != 'excluded_execution')) is bool,
                f'{identifier}: invalid in_scope')
    require(evidence_dir.is_dir(), 'evidence directory does not exist')
    records = defaultdict(list)
    artifacts = []
    for path in sorted(evidence_dir.glob('t*.json')):
        require(not path.is_symlink(), f'{path.name}: symlink evidence refused')
        raw = path.read_bytes()
        # Task records have several historical schemas; reject duplicate JSON keys
        # but retain task-level records verbatim, rather than inventing item claims.
        doc = object_value(json.loads(raw, object_pairs_hook=unique_object), path.name)
        artifact = {'path': str(path.relative_to(Path.cwd())) if path.is_relative_to(Path.cwd()) else str(path), 'sha256': hashlib.sha256(raw).hexdigest(),
                    'document': doc}
        artifacts.append(artifact)
        for index, entry in enumerate(array(doc.get('items', []), path.name)):
            entry = object_value(entry, path.name)
            identifier = text(entry.get('id'), path.name)
            records[identifier].append((artifact, index, entry))
    result = []
    for identifier, item in items.items():
        included = item.get('in_scope', item.get('basis') != 'excluded_execution')
        related = [a for a in artifacts if a['document'].get('task') in item['tasks']]
        row = {'id': identifier, 'tasks': item['tasks'], 'sources': item['sources'],
               'conclusion': 'unverified' if related or records[identifier] else 'missing',
               'task_records': [{'path': a['path'], 'sha256': a['sha256']} for a in related],
               'candidates': []}
        if not included:
            row['conclusion'] = 'excluded'
        for artifact, index, entry in records[identifier]:
            ref = {'path': artifact['path'], 'sha256': artifact['sha256'], 'item_index': index}
            try:
                validate({**baseline, 'items': [item]}, {'schema_version': 1, 'items': [entry]})
            except (ValueError, TypeError) as error:
                ref['rejection'] = str(error)
            else:
                # Keep exact tested fields only when the existing strict checker accepts.
                if included and row['conclusion'] != 'verified':
                    row.update({key: entry[key] for key in ('tasks', 'tests', 'red', 'green', 'platforms', 'external_prerequisites')})
                    row['sources'] = item['sources']
                    row['conclusion'] = 'verified'
            row['candidates'].append(ref)
        if included and row['conclusion'] != 'verified':
            row['blocking_reasons'] = ([ref['rejection'] for ref in row['candidates']
                                        if 'rejection' in ref] or
                                       ['No item-level evidence accepted by strict checker'])
        result.append(row)
    counts = {state: sum(row['conclusion'] == state for row in result)
              for state in ('verified', 'unverified', 'missing', 'excluded')}
    return {'schema_version': 1, 'baseline': {'path': str(baseline_path.relative_to(Path.cwd())) if baseline_path.is_relative_to(Path.cwd()) else str(baseline_path),
            'sha256': hashlib.sha256(baseline_path.read_bytes()).hexdigest()},
            'items': result, 'artifacts': [{'path': a['path'], 'sha256': a['sha256'],
                'task': a['document'].get('task'), 'status': a['document'].get('status'),
                'scope_adjustment': a['document'].get('scope_adjustment')} for a in artifacts],
            'counts': counts,
            'unmapped_ids': sorted(set(records) - set(items)),
            'full_compatibility_claimed': counts['verified'] > 0 and all(
                r['conclusion'] in ('verified', 'excluded') for r in result),
            'waivers_are_passing_evidence': False}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--baseline', required=True)
    parser.add_argument('--evidence-dir', required=True)
    parser.add_argument('--output', required=True)
    args = parser.parse_args()
    try:
        result = summarize(args.baseline, args.evidence_dir, args.output)
        output = Path(args.output)
        # All reads and validation finish before the output is atomically replaced.
        with tempfile.NamedTemporaryFile(mode='w', encoding='utf-8', dir=output.parent,
                                         delete=False) as temporary:
            name = temporary.name
            try:
                json.dump(result, temporary, ensure_ascii=False, indent=2)
                temporary.write('\n')
                temporary.flush()
                os.fsync(temporary.fileno())
            except BaseException:
                Path(name).unlink(missing_ok=True)
                raise
        try:
            os.replace(name, output)
        finally:
            Path(name).unlink(missing_ok=True)
    except (OSError, ValueError, TypeError) as error:
        print(f'Evidence summary rejected: {error}', file=sys.stderr)
        return 1
    print(json.dumps(result['counts']))
    return 0


if __name__ == '__main__':
    raise SystemExit(main())
