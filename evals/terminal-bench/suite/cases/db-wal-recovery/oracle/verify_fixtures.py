"""Independent fixture audit, never copied into an agent environment."""
import json
from pathlib import Path
import sqlite3
import tempfile

ROOT = Path(__file__).resolve().parents[1]


def audit_wal(kind):
    name = 'structure.wal' if kind == 'm1' else 'boundary.wal'
    raw = (ROOT / 'fixtures' / kind / name).read_bytes()
    word = lambda data: int.from_bytes(data, 'big')
    magic, version, size, sequence = [word(raw[n:n + 4]) for n in range(0, 16, 4)]
    salts = [word(raw[n:n + 4]) for n in (16, 20)]
    def checksum(data, state):
        a, b = state
        words = [int.from_bytes(data[n:n + 4], 'little') for n in range(0, len(data), 4)]
        for n in range(0, len(words), 2):
            a = (a + words[n] + b) % (2 ** 32)
            b = (b + words[n + 1] + a) % (2 ** 32)
        return a, b
    state = checksum(raw[:24], (0, 0))
    header_valid = state == tuple(word(raw[n:n + 4]) for n in (24, 28))
    frames = []
    versions = {}
    committed = {}
    last = count = db_pages = 0
    invalid = None
    for offset in range(32, len(raw), 24 + size):
        number = (offset - 32) // (24 + size) + 1
        frame = raw[offset:offset + 24 + size]
        if len(frame) != 24 + size:
            invalid = number
            break
        pg, db = word(frame[:4]), word(frame[4:8])
        frames.append({'page_number': pg, 'database_pages': db, 'offset': offset})
        candidate = checksum(frame[:8] + frame[24:], state)
        if frame[8:16] != raw[16:24] or candidate != tuple(word(frame[n:n + 4]) for n in (16, 20)):
            invalid = number
            break
        state = candidate
        count += 1
        versions[pg] = number
        if db:
            last, db_pages, committed = number, db, versions.copy()
    actual = ({'magic': f'{magic:08x}', 'version': version, 'page_size': size,
               'checkpoint_sequence': sequence, 'salts': salts, 'frames': frames}
              if kind == 'm1' else
              {'header_valid': header_valid, 'valid_frames': count, 'first_invalid_frame': invalid,
               'last_commit_frame': last, 'database_pages': db_pages,
               'committed_pages': [{'page_number': k, 'frame': v} for k, v in sorted(committed.items())]})
    expected = json.loads((ROOT / 'oracle' / kind / 'expected.json').read_text())['expected']
    assert actual == expected, (actual, expected)


def audit_database(kind, key):
    with tempfile.TemporaryDirectory() as directory:
        database = Path(directory) / 'audit.db'
        database.write_bytes((ROOT / 'fixtures' / kind / 'base.db').read_bytes())
        encrypted = (ROOT / 'fixtures' / kind / 'changes.xor').read_bytes()
        database.with_name('audit.db-wal').write_bytes(bytes(b ^ key for b in encrypted))
        with sqlite3.connect(database) as con:
            assert con.execute('PRAGMA integrity_check').fetchone() == ('ok',)
            actual = [dict(zip(('id', 'name', 'value'), row)) for row in
                      con.execute('SELECT id,name,value FROM readings ORDER BY id')]
        expected = json.loads((ROOT / 'oracle' / kind / 'expected.json').read_text())['expected']
        assert actual == expected


if __name__ == '__main__':
    audit_wal('m1')
    audit_wal('m3')
    audit_database('m2', 0x29)
    audit_database('m4', 0x6d)
    print('Four independent fixture audits passed')
