"""Black-box execution on new directories; expectations are verifier-only data."""
import copy
import csv
import json
from pathlib import Path
import subprocess
import sys
import tempfile

import pyarrow as pa
import pyarrow.parquet as pq


def strict(value):
    """JSON equality must distinguish integer IDs from strings and booleans."""
    if isinstance(value, dict):
        return ('object', tuple(sorted((k, strict(v)) for k, v in value.items())))
    if isinstance(value, list):
        return ('array', tuple(strict(v) for v in value))
    return (type(value).__name__, value)


def equivalent(actual, expected):
    assert strict(actual) == strict(expected), 'incorrect complete output'


def report(value):
    assert set(value) == {'total_conflicts', 'conflicts'}
    assert type(value['total_conflicts']) is int
    assert value['total_conflicts'] == len(value['conflicts'])
    return {**value, 'conflicts': sorted(value['conflicts'], key=lambda c: (c['user_id'], c['field']))}


def shifted(value, offset):
    if isinstance(value, list):
        return [shifted(v, offset) for v in value]
    if isinstance(value, dict):
        return {k: (str(int(v) + offset).zfill(3) if isinstance(v, str) else v + offset)
                if k in ('id', 'user_id', 'userId') else shifted(v, offset)
                for k, v in value.items()}
    return value


def check(app, oracle):
    child = (oracle / 'child.txt').read_text().strip()
    program = app / 'merge.py'
    assert program.is_file(), 'missing merge.py'
    scenario = json.loads((oracle / 'scenarios.json').read_text())
    for offset in (0, 611):
        data = shifted(copy.deepcopy(scenario), offset)
        with tempfile.TemporaryDirectory(prefix='merger-probe-') as tmp:
            root = Path(tmp)
            if child in ('m1', 'm4'):
                a, b, c = data['raw']
                if offset:
                    a.reverse(); b.reverse(); c.reverse()
                (root / 'source_a.json').write_text(json.dumps(a, ensure_ascii=False))
                with (root / 'source_b.csv').open('w', newline='') as f:
                    writer = csv.DictWriter(f, fieldnames=list(b[0]))
                    writer.writeheader(); writer.writerows(b)
                pq.write_table(pa.Table.from_pylist(c), root / 'source_c.parquet')
            else:
                rows = data['canonical']
                if child == 'm2':
                    rows = copy.deepcopy(rows)
                    rows[2]['user_id'] = str(rows[2]['user_id']).zfill(3)
                    rows.append(copy.deepcopy(rows[2]))
                (root / 'records.json').write_text(json.dumps(rows[::-1], ensure_ascii=False))
            result = subprocess.run([sys.executable, str(program), str(root)],
                                    cwd=root, capture_output=True, timeout=10)
            assert result.returncode == 0, 'candidate failed'
            if child == 'm1':
                actual = json.loads((root / 'normalized.json').read_text())
                key = lambda row: (row['source'], row['user_id'])
                equivalent(sorted(actual, key=key), sorted(data['canonical'], key=key))
            elif child == 'm2':
                equivalent(json.loads((root / 'groups.json').read_text()), data['groups'])
            else:
                expected = data['merged']
                if child == 'm3':
                    actual = json.loads((root / 'merged.json').read_text())
                    assert set(actual) == {'rows', 'report'}
                else:
                    table = pq.read_table(root / 'merged_users.parquet')
                    expected_schema = pa.schema([('user_id', pa.int64())] +
                                                [(f, pa.string()) for f in ['name', 'email', 'created_date', 'status']])
                    assert table.schema.equals(expected_schema, check_metadata=False), 'wrong parquet schema'
                    actual = {'rows': table.to_pylist(), 'report': json.loads((root / 'conflicts.json').read_text())}
                equivalent(actual['rows'], expected['rows'])
                equivalent(report(actual['report']), report(expected['report']))


if __name__ == '__main__':
    try:
        check(Path(sys.argv[1]), Path(sys.argv[2]))
    except (AssertionError, ValueError, KeyError, TypeError, OSError, subprocess.TimeoutExpired, pa.ArrowException) as exc:
        print(type(exc).__name__ + ': ' + str(exc), file=sys.stderr)
        sys.exit(1)
