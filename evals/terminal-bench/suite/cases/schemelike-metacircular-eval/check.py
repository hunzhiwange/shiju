"""Execute candidate interpreters against independent synthetic expectations."""
import json
from pathlib import Path
import subprocess
import sys
import tempfile


def probes(kind):
    if kind == 'm1':
        symbol = lambda x: {'symbol': x}
        return [
            ("'(branch (leaf -19) #f) ; ignored\n(+ 31 8)", '',
             [[symbol('quote'), [symbol('branch'), [symbol('leaf'), -19], False]],
              [symbol('+'), 31, 8]], 0),
            ('; comment\n"a ; ( b" #t () -37', '', ['a ; ( b', True, [], -37], 0),
            ('(a (b)', '', 'ERROR:syntax\n', 1),
            (')', '', 'ERROR:syntax\n', 1),
            ('', '', [], 0)]
    if kind == 'm2':
        return [
            ('(define amount 23) (display (+ amount 18)) (newline)', '', '41\n', 0),
            ('(display (if 0 71 unknown)) (display (if #f missing 29))', '', '7129', 0),
            ('(define (f n) (begin (+ n 7) (* n 4))) (display (f 9))', '', '36', 0),
            ('(display (let ((a 17) (b 5)) (- a b)))', '', '12', 0)]
    if kind == 'm3':
        return [
            ('(define x 41) (define f (lambda (y) (+ x y))) (display (let ((x 90)) (f 2)))', '', '43', 0),
            ('(define x 8) (let ((y 4)) (set! x 27)) (display x)', '', '27', 0),
            ('(define x 31) (display (let ((x 2) (y x)) y)) (display x)', '', '3131', 0),
            ('(define (f x) 5) (f 1 2) (display "late")', '', 'ERROR:arity\n', 1),
            ('(set! absent 7) (display "late")', '', 'ERROR:undefined\n', 1),
            ('(3 4) (display "late")', '', 'ERROR:procedure\n', 1)]
    if kind == 'm4':
        return [
            ('(define a (read)) (display (+ a 16)) (newline) (display (read))', '27\n93\n', '43\n93', 0),
            ('(putchr (getchr)) (putchr (getchr)) (putchr (getchr))', 'Z!\ntrailer', 'Z!\n', 0),
            ('(+ 3 8) (define x 9) (display "done") (newline) (+ 6 8)', '', 'done\n', 0),
            ('; leading\n\n(display "alpha")\n; middle\n(display "beta")', '', 'alphabeta', 0)]
    raise ValueError('unknown kind')


def check(app, oracle):
    artifact = app / 'interpreter.py'
    if artifact.is_symlink() or not artifact.is_file() or artifact.stat().st_size > 100000:
        return False
    kind = json.loads((oracle / 'task.json').read_text())['kind']
    with tempfile.TemporaryDirectory(prefix='scheme-probes-', dir=app) as directory:
        for index, (source, payload, expected, code) in enumerate(probes(kind)):
            command = [sys.executable, '-I', str(artifact)]
            if kind == 'm1':
                command.append('--parse')
                data = source
            else:
                program = Path(directory) / f'probe-{index}.scm'
                program.write_text(source)
                data = str(program) + '\n' + payload
            result = subprocess.run(command, input=data, text=True, capture_output=True,
                                    cwd=app, timeout=5)
            if result.returncode != code or result.stderr:
                return False
            if kind == 'm1' and code == 0:
                if json.loads(result.stdout) != expected:
                    return False
            elif result.stdout != expected:
                return False
    return True


if __name__ == '__main__':
    try:
        passed = check(Path(sys.argv[1]).resolve(), Path(sys.argv[2]).resolve())
    except (OSError, ValueError, subprocess.TimeoutExpired):
        passed = False
    sys.exit(0 if passed else 1)
