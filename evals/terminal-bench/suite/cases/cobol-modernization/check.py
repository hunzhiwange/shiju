"""Check synthetic artifacts and execute candidate programs on hidden inputs."""
import copy
from decimal import Decimal, ROUND_HALF_UP
import json
from pathlib import Path
import subprocess
import sys
import tempfile


def unique_pairs(pairs):
    result = {}
    for key, value in pairs:
        if key in result:
            raise ValueError('duplicate JSON key')
        result[key] = value
    return result


def decode(text):
    return json.loads(text, object_pairs_hook=unique_pairs,
                      parse_constant=lambda value: (_ for _ in ()).throw(ValueError(value)))


def equal(actual, expected):
    # Canonical encoding preserves integer/bool/float distinctions.
    return json.dumps(actual, sort_keys=True, allow_nan=False) == json.dumps(expected, sort_keys=True)


def invoke(script, work, value=None, args=()):
    result = subprocess.run([sys.executable, str(script), *map(str, args)],
                            input=json.dumps(value) if value is not None else '',
                            text=True, capture_output=True, cwd=work, timeout=3,
                            env={'PATH': '/usr/local/bin:/usr/bin:/bin', 'PYTHONDONTWRITEBYTECODE': '1'})
    if result.returncode != 0 or len(result.stdout) > 65536:
        raise ValueError('candidate process failed or produced excessive output')
    return decode(result.stdout)


def expected_update(value):
    x = copy.deepcopy(value)
    ids = {a['id'] for a in x['accounts']}
    owners = {b['id']: b['owner'] for b in x['items']}
    if (x['buyer'] in ids and x['seller'] in ids and x['item'] in owners
            and owners[x['item']] == x['seller']):
        for account in x['accounts']:
            credit = x['amount'] if account['id'] == x['seller'] else 0
            debit = x['amount'] if account['id'] == x['buyer'] else 0
            account['balance'] += credit - debit
        for item in x['items']:
            if item['id'] == x['item']:
                item['owner'] = x['buyer']
        x['journal'] += [':'.join(map(str, (x['item'], x['amount'], x['seller'], x['buyer'])))]
    return {key: x[key] for key in ('accounts', 'items', 'journal')}


def check(app, oracle):
    task = decode((oracle / 'task.json').read_text())
    kind = task['kind']
    if kind == 'm1':
        return equal(decode((app / 'result.json').read_text()), task['expected'])
    scripts = {'m2': 'calculate.py', 'm3': 'update.py', 'm4': 'compare.py'}
    script = app / scripts[kind]
    if not script.is_file() or script.is_symlink():
        return False
    source = script.read_bytes()
    cases = task['scenarios'] if kind == 'm4' else task['inputs']
    for value in cases:
        with tempfile.TemporaryDirectory(prefix='cobol-check-') as directory:
            work = Path(directory)
            candidate = work / 'candidate.py'
            candidate.write_bytes(source)
            if kind == 'm2':
                net = value['gross_cents'] - value['discount_cents']
                tax = int((Decimal(net) * Decimal(value['tax_basis_points']) / Decimal(10000))
                          .quantize(Decimal(1), rounding=ROUND_HALF_UP))
                expected = {'net_cents': net, 'tax_cents': tax, 'total_cents': net + tax}
                actual = invoke(candidate, work, value)
            elif kind == 'm3':
                expected = expected_update(value)
                actual = invoke(candidate, work, value)
            else:
                left, right = work / 'legacy', work / 'modern'
                for folder, files in ((left, value['left']), (right, value['right'])):
                    folder.mkdir()
                    for name, content in files.items():
                        (folder / name).write_bytes(content.encode('ascii'))
                    (folder / 'ignored.tmp').write_text(folder.name)
                before = {str(p.relative_to(work)): p.read_bytes()
                          for folder in (left, right) for p in folder.iterdir()}
                names = ('ACCOUNTS.DAT', 'BOOKS.DAT', 'TRANSACTIONS.DAT')
                differences = [n for n in names if n not in value['left'] or n not in value['right']
                               or value['left'][n] != value['right'][n]]
                expected = {'equal': not differences, 'differences': differences}
                actual = invoke(candidate, work, args=(left, right))
                after = {str(p.relative_to(work)): p.read_bytes()
                         for folder in (left, right) for p in folder.iterdir()}
                if after != before:
                    return False
            if not equal(actual, expected):
                return False
    return True


if __name__ == '__main__':
    try:
        passed = check(Path(sys.argv[1]), Path(sys.argv[2]))
    except (ValueError, OSError, KeyError, TypeError, subprocess.TimeoutExpired):
        passed = False
    sys.exit(0 if passed else 1)
