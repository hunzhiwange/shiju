"""Independent real SQLite, artifact and gcov checks; no agent reports."""
import gzip
import hashlib
import json
import os
from pathlib import Path
import shutil
import sqlite3
import subprocess
import sys
import tempfile


def run(args, cwd, env=None):
    return subprocess.run(args, cwd=cwd, env=env, capture_output=True,
                          text=True, timeout=100)


def coverage(work, data_root, notes_root, source_name):
    """Require fresh counters that the matching real gcov can parse."""
    counters = list(data_root.rglob('*.gcda'))
    if not counters:
        return False
    matched = False
    for data in counters:
        if data.is_symlink() or data.stat().st_size < 16:
            return False
        note = notes_root / data.with_suffix('.gcno').name
        if not note.is_file():
            return False
        destination = data.with_suffix('.gcno')
        if note.resolve() != destination.resolve():
            shutil.copyfile(note, destination)
        output = work / 'gcov-output'
        output.mkdir(exist_ok=True)
        for old in output.glob('*.gcov.json.gz'):
            old.unlink()
        result = run(['gcov', '--json-format', str(destination)], output)
        if result.returncode:
            return False
        for report in output.glob('*.gcov.json.gz'):
            with gzip.open(report, 'rt') as stream:
                document = json.load(stream)
            for entry in document['files']:
                if Path(entry['file']).name == source_name:
                    matched |= any(line['count'] > 0 for line in entry['lines'])
    return matched


def query(binary, work, env=None):
    for values in [(4, 9, -2), (17, 0, 31, 8)]:
        sql = ('CREATE TABLE samples(v INTEGER);INSERT INTO samples VALUES'
               + ','.join(f'({v})' for v in values)
               + ';SELECT count(*),sum(v) FROM samples;')
        result = run([str(binary), ':memory:', sql], work, env)
        if result.returncode or result.stdout != f'{len(values)}|{sum(values)}\n':
            return False
    return True


def dependencies_unchanged(task):
    return all(Path(name).is_file() and not Path(name).is_symlink()
               and hashlib.sha256(Path(name).read_bytes()).hexdigest() == sha
               for name, sha in task['dependencies'].items())


def check(app, oracle):
    task = json.loads((oracle / 'task.json').read_text())
    for name, sha in task['immutable'].items():
        path = app / name
        if path.is_symlink() or hashlib.sha256(path.read_bytes()).hexdigest() != sha:
            return False
    if not dependencies_unchanged(task):
        return False
    child = task['child']
    script = app / {'m1': 'prepare.sh', 'm2': 'build.sh', 'm3': 'run.sh', 'm4': 'install.sh'}[child]
    if not script.is_file():
        return False
    if child == 'm1':
        if run(['sh', str(script)], app).returncode or not dependencies_unchanged(task):
            return False
        for name in ['sqlite3.c', 'sqlite3.h', 'shell.c']:
            path = app / 'source' / name
            if path.is_symlink() or path.read_bytes() != (Path('/opt/sqlite') / name).read_bytes():
                return False
        result = run([str(app / 'probe')], app)
        return (result.returncode == 0 and result.stdout == '3.50.4\n'
                and coverage(app, app, app, 'probe.c'))
    if child == 'm2':
        shutil.rmtree(app / 'build', ignore_errors=True)
        if run(['sh', str(script)], app).returncode or not dependencies_unchanged(task):
            return False
        build = app / 'build'
        binary = build / 'sqlite3'
        return query(binary, app) and coverage(app, build, build, 'sqlite3.c')
    if child == 'm3':
        for index, values in enumerate([(2, 7, -5), (31, 4, 0, 8)]):
            shutil.rmtree(app / 'coverage', ignore_errors=True)
            db, sql = app / f'input-{index}.db', app / f'input-{index}.sql'
            sql.write_text('CREATE TABLE samples(v INTEGER);INSERT INTO samples VALUES'
                           + ','.join(f'({v})' for v in values)
                           + ';SELECT count(*),sum(v) FROM samples;\n')
            result = run(['sh', str(script), str(db), str(sql)], app)
            if (result.returncode or result.stdout != f'{len(values)}|{sum(values)}\n'
                    or not dependencies_unchanged(task)):
                return False
            if not db.is_file():
                return False
            with sqlite3.connect(db) as connection:
                if connection.execute('SELECT v FROM samples ORDER BY rowid').fetchall() != [(v,) for v in values]:
                    return False
            if not coverage(app, app / 'coverage', Path('/opt/sqlite'), 'sqlite3.c'):
                return False
        sql = app / 'invalid.sql'
        sql.write_text('SELECT absent_column FROM absent_table;\n')
        return run(['sh', str(script), str(app / 'invalid.db'), str(sql)], app).returncode != 0
    if child == 'm4':
        if run(['sh', str(script)], app).returncode or not dependencies_unchanged(task):
            return False
        env = dict(os.environ, PATH=str(app / 'bin') + os.pathsep + os.environ['PATH'],
                   GCOV_PREFIX=str(app / 'coverage'), GCOV_PREFIX_STRIP='2')
        binary = shutil.which('sqlite3', path=env['PATH'])
        if binary is None or Path(binary).resolve() != Path('/opt/sqlite/sqlite3'):
            return False
        shutil.rmtree(app / 'coverage', ignore_errors=True)
        with tempfile.TemporaryDirectory(dir=app) as directory:
            if not query('sqlite3', Path(directory), env):
                return False
        return coverage(app, app / 'coverage', Path('/opt/sqlite'), 'sqlite3.c')
    raise ValueError('unknown child')


if __name__ == '__main__':
    try:
        accepted = check(Path(sys.argv[1]), Path(sys.argv[2]))
    except (OSError, ValueError, KeyError, sqlite3.Error, subprocess.TimeoutExpired):
        accepted = False
    sys.exit(0 if accepted else 1)
