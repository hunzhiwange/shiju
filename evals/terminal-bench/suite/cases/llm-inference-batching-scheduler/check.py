"""Independent structural and arithmetic checks on synthetic plan artifacts."""
import json
from pathlib import Path
import sys


def integer(value):
    return type(value) is int


def plan_shapes(plan, requests):
    if not isinstance(plan, list) or len(plan) != len(requests):
        raise ValueError('coverage')
    index = {r['request_id']: r for r in requests}
    seen, batches, shapes = set(), {}, set()
    for record in plan:
        if not isinstance(record, dict) or set(record) != {'request_id', 'batch_id', 'shape'}:
            raise ValueError('record schema')
        rid, bid, shape = record['request_id'], record['batch_id'], record['shape']
        if not isinstance(rid, str) or rid not in index or rid in seen:
            raise ValueError('request identity')
        seen.add(rid)
        if not isinstance(bid, str) or not bid or not isinstance(shape, dict):
            raise ValueError('batch schema')
        if set(shape) != {'seq_align', 'heads_align', 'hidden_align'} or not all(integer(v) for v in shape.values()):
            raise ValueError('shape schema')
        s = shape['seq_align']
        if s <= 0 or s % 64 or s < ((index[rid]['prompt_len'] + 63) // 64) * 64:
            raise ValueError('sequence feasibility')
        if shape['heads_align'] != 32 or shape['hidden_align'] != 4096:
            raise ValueError('model dimensions')
        if bid in batches and batches[bid] != shape:
            raise ValueError('batch consistency')
        batches[bid] = shape
        shapes.add(s)
    return shapes


def cost(plan, requests):
    index = {r['request_id']: r for r in requests}
    groups = {}
    for record in plan:
        groups.setdefault(record['batch_id'], []).append(record)
    total = 100 * sum({r['shape']['seq_align'] for r in plan})
    for records in groups.values():
        s = records[0]['shape']['seq_align']
        g = max(index[r['request_id']]['gen_len'] for r in records)
        total += len(records) * s * (s + g) + 1000
    return total


def check(app, oracle):
    task = json.loads((oracle / 'task.json').read_text())
    data, child = task['input'], task['child']
    if json.loads((app / 'input.json').read_text()) != data:
        return False
    result = json.loads((app / 'result.json').read_text())
    if child == 'm1':
        wanted = [{'request_id': r['request_id'], 'seq_align': ((r['prompt_len'] + 63) // 64) * 64}
                  for r in data['requests']]
        return (result == wanted and isinstance(result, list)
                and all(integer(r.get('seq_align')) for r in result))
    if child == 'm2':
        plan_shapes(result, data['requests'])
        reqs = {r['request_id']: r for r in data['requests']}
        return (len({r['batch_id'] for r in result}) == 2
                and all(r['shape']['seq_align'] == ((reqs[r['request_id']]['prompt_len'] + 63) // 64) * 64 for r in result))
    if child == 'm3':
        if not isinstance(result, list) or len(result) != 2:
            return False
        shapes = set()
        for plan, requests in zip(result, data['buckets']):
            shapes |= plan_shapes(plan, requests)
        return len(shapes) <= 2
    if child == 'm4':
        if not isinstance(result, dict) or set(result) != {'candidate_index', 'cost', 'plan'}:
            return False
        n = result['candidate_index']
        if not integer(n) or not 0 <= n < len(data['candidates']) or not integer(result['cost']):
            return False
        plan_shapes(result['plan'], data['requests'])
        selected = data['candidates'][n]
        if sorted(result['plan'], key=lambda r: r['request_id']) != sorted(selected, key=lambda r: r['request_id']):
            return False
        expected = cost(selected, data['requests'])
        return result['cost'] == expected == min(cost(p, data['requests']) for p in data['candidates'])
    raise RuntimeError('unknown child')


if __name__ == '__main__':
    app, oracle = [Path(p).resolve() for p in sys.argv[1:]]
    try:
        ok = check(app, oracle)
    except (OSError, ValueError, TypeError, KeyError):
        ok = False
    sys.exit(0 if ok else 1)
