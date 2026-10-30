"""Independent real-pdflatex checker; exit 2 means infrastructure failure."""
import json
from pathlib import Path
import re
import shutil
import subprocess
import sys
import tempfile


class InfrastructureError(RuntimeError):
    pass


def allowed_change(synonyms, before, after):
    families = [set(line.split(', ')) for line in synonyms.splitlines() if line]
    pattern = r'\\[A-Za-z]+|[A-Za-z]+|[^A-Za-z\\]+|\\'
    old, new = re.findall(pattern, before), re.findall(pattern, after)
    return len(old) == len(new) and all(
        a == b or (a.isascii() and a.isalpha() and b.isascii() and b.isalpha()
                   and any(a in family and b in family for family in families))
        for a, b in zip(old, new))


def compile_document(item):
    engine = shutil.which('pdflatex')
    if engine is None:
        raise InfrastructureError('real pdflatex is required; no synthetic log fallback')
    with tempfile.TemporaryDirectory(prefix='overfull-verifier-') as directory:
        work = Path(directory)
        (work/'main.tex').write_text(item['main'])
        (work/'input.tex').write_text(item['input'])
        process = subprocess.run([engine, '-no-shell-escape', '-interaction=nonstopmode', '-halt-on-error', 'main.tex'], cwd=work, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, timeout=15)
        log = (work/'main.log').read_text(errors='replace') if (work/'main.log').is_file() else ''
        pdf = work/'main.pdf'
        return {'compiled': process.returncode == 0, 'overfull': len(re.findall(r'Overfull \\hbox', log)), 'pdf': pdf.is_file() and pdf.read_bytes().startswith(b'%PDF-')}


def run_candidate(app, payload):
    if not (app/'answer.py').is_file() or (app/'answer.py').is_symlink():
        return None
    with tempfile.TemporaryDirectory(prefix='overfull-candidate-') as directory:
        try:
            result = subprocess.run([sys.executable, str((app/'answer.py').resolve())], cwd=directory, input=json.dumps(payload), capture_output=True, text=True, timeout=25)
            if result.returncode != 0:
                return None
            return json.loads(result.stdout)
        except (subprocess.TimeoutExpired, json.JSONDecodeError):
            return None


def report_matches(result, expected):
    return (isinstance(result, dict) and set(result) == set(expected)
            and type(result['compiled']) is bool and type(result['pdf']) is bool
            and type(result['overfull']) is int and result == expected)


def check(app, oracle):
    task = json.loads((oracle/'task.json').read_text())
    # Fail before scoring nop when infrastructure is absent.
    if not shutil.which('pdflatex'):
        raise InfrastructureError('real pdflatex missing')
    main = task['main']
    documents = [{'main': main, 'input': r'\noindent\mbox{'+task['long']+r'}\par'},
                 {'main': main, 'input': r'\noindent\mbox{tiny}\par'},
                 {'main': main, 'input': r'\UndefinedSyntheticCommand'}]
    if task['id'] in ('m1', 'm4'):
        expected = [compile_document(doc) for doc in documents]
        if not (expected[0]['compiled'] and expected[0]['overfull'] > 0 and expected[0]['pdf'] and expected[1] == {'compiled': True, 'overfull': 0, 'pdf': True} and not expected[2]['compiled'] and not expected[2]['pdf']):
            raise InfrastructureError('synthetic TeX probes did not establish required baseline')
        if task['id'] == 'm1':
            return all(report_matches(run_candidate(app, doc), value) for doc, value in zip(documents, expected))
        # Two orders detect stale success/log/artifact reuse.
        for order in ((0, 1, 2), (2, 0, 1)):
            result = run_candidate(app, {'documents': [documents[i] for i in order]})
            if not isinstance(result, dict) or set(result) != {'results'} or not isinstance(result['results'], list) or len(result['results']) != 3:
                return False
            if not all(report_matches(item, expected[index]) for item, index in zip(result['results'], order)):
                return False
        return True
    long = task['long']
    probes = [(r'\noindent\mbox{'+long+r'}\par'+'\n', long+', tiny\n'),
              (r'\noindent\mbox{'+long+r'}\par'+'\n% fixed:  spacing!\n', long+', small, tiny\nnoindent, xx\n'),
              (r'\noindent\mbox{'+long+r'}\par'+'\n'+r'\noindent\mbox{'+long+r'}\par', long+', tiny\n')]
    if task['id'] == 'm3':
        probes.append((r'\noindent\mbox{'+long+r'}\par'+'\n% Preserve---CAPS, digits 123 and  two spaces.\n', long+', tiny\nmbox, z\n'))
    for before, synonyms in probes:
        baseline = compile_document({'main': main, 'input': before})
        if not baseline['compiled'] or baseline['overfull'] < 1:
            raise InfrastructureError('source must actually reproduce a warning')
        result = run_candidate(app, {'input': before, 'synonyms': synonyms})
        if not isinstance(result, dict) or set(result) != {'input'} or not isinstance(result['input'], str) or not allowed_change(synonyms, before, result['input']):
            return False
        compiled = compile_document({'main': main, 'input': result['input']})
        if compiled != {'compiled': True, 'overfull': 0, 'pdf': True}:
            return False
    return True


if __name__ == '__main__':
    try:
        passed = check(Path(sys.argv[1]), Path(sys.argv[2]))
    except (InfrastructureError, subprocess.TimeoutExpired, OSError) as error:
        print(str(error), file=sys.stderr)
        sys.exit(2)
    sys.exit(0 if passed else 1)
