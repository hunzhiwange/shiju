"""Synthetic CSV bytes and real Vim execution; hidden expectations stay outside /app."""
import json
from pathlib import Path
import re
import shutil
import subprocess
import sys
import tempfile


def decode_keys(value):
    """Deliberately small, explicit Vim double-string dialect."""
    result = ''
    while value:
        if value.startswith(r'\<CR>'):
            result += '\r'; value = value[5:]
        elif value.startswith(r'\<Esc>'):
            result += '\x1b'; value = value[6:]
        elif value.startswith('\\\\'):
            result += '\\'; value = value[2:]
        elif value[0] == '\\':
            raise ValueError('unsupported string escape')
        else:
            result += value[0]; value = value[1:]
    return result


def safe_keys(keys):
    while keys:
        if keys.startswith(':s/'):
            # Unescaped slash separates pattern/replacement; expressions forbidden.
            match = re.match(r':s/((?:\\.|[^/\r])*)/((?:\\.|[^/\r])*)/([ge]*)\r', keys)
            if not match or '\\=' in match[2] or any(c in match[0] for c in '|\n"'):
                return False
            keys = keys[match.end():]
        else:
            token = next((t for t in ('gU$', 'A;OK\x1b', '0', '^', '$', 'j') if keys.startswith(t)), None)
            if token is None:
                return False
            keys = keys[len(token):]
    return True


def valid_script(text):
    lines = [line for line in text.splitlines() if line.strip()]
    if len(lines) != 7 or lines[3:] != [':%normal! @a', ':%normal! @b', ':%normal! @c', ':wq']:
        return False
    macros = []
    for register, line in zip('abc', lines[:3]):
        match = re.fullmatch(r"call setreg\('" + register + r"', \"([^\"]*)\"\)", line)
        if not match:
            return False
        try:
            keys = decode_keys(match[1])
        except ValueError:
            return False
        if not keys or not safe_keys(keys):
            return False
        macros.append(keys)
    return len(set(macros)) == 3 and sum(map(len, macros)) < 200


def expected(raw):
    return ''.join(';'.join(field.strip().upper() for field in row.split(',')[::-1]) + ';OK\n'
                   for row in raw.splitlines()).encode()


def check(app, oracle):
    task = json.loads((oracle / 'task.json').read_text())
    if task['child'] == 'm1':
        output = app / 'output.csv'
        return (output.is_file() and not output.is_symlink()
                and output.read_bytes() == task['expected'].encode()
                and (app / 'input.csv').read_bytes() == task['input'].encode())
    if task['child'] not in ('m2', 'm3', 'm4'):
        raise ValueError('unknown child')
    vim = shutil.which('vim')
    if vim is None:
        raise RuntimeError('Vim dependency missing')
    script = app / 'apply_macros.vim'
    if not script.is_file() or script.is_symlink():
        return False
    text = script.read_text()
    if not valid_script(text):
        return False
    # Multiple pristine files expose cursor/last-line bugs and fixed-output cheats.
    probes = [task['input'], ' a , b , c \n', ' first word , second9 , tail7 \n x, Y, zz \n']
    with tempfile.TemporaryDirectory(prefix='csv-vim-') as directory:
        work = Path(directory)
        candidate = work / 'apply.vim'; candidate.write_text(text)
        for raw in probes:
            data = work / 'input.csv'; data.write_text(raw)
            try:
                result = subprocess.run([vim, '-Nu', 'NONE', '-i', 'NONE', '-n', '-Es', str(data), '-S', str(candidate)],
                                        cwd=work, capture_output=True, timeout=45)
            except subprocess.TimeoutExpired:
                return False
            if result.returncode or data.read_bytes() != expected(raw):
                return False
    return True


if __name__ == '__main__':
    sys.exit(0 if check(*map(Path, sys.argv[1:])) else 1)
