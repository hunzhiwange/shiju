"""Trusted fixed proposition, fresh Coq compilation and kernel verification."""
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import sys
import tempfile

WORDS = {'intros', 'induction', 'as', 'simpl', 'rewrite', 'reflexivity',
         'exact', 'apply', 'assumption', 'n', 'm', 'k', 'IH', 'S', 'O',
         'Nat.add_0_r', 'Nat.add_succ_r'}
GOALS = {'forall n : nat, n + 0 = n',
         'forall n m : nat, n + S m = S (n + m)',
         'forall n : nat, n + 1 = S n',
         'forall n m : nat, n + m = m + n'}


def permitted(body):
    # Closed lexical language: no vernacular, comments, plugins, quotation,
    # shell tactics, tactic definitions or candidate-provided dependencies.
    tokens = re.findall(r'Nat\.add_0_r|Nat\.add_succ_r|[A-Za-z_][A-Za-z_0-9]*|[0-9]+|[.;+\-\[\]|():=<>]|\s+', body)
    if ''.join(tokens) != body or not body.strip():
        return False
    return all(token in WORDS or token.isspace() or token.isdigit()
               or (len(token) == 1 and token in '.;+-[]|():=<>') for token in tokens)


def check(app, oracle):
    source = app / 'proof.txt'
    if source.is_symlink() or not source.is_file() or source.stat().st_size > 16384:
        return False
    body = source.read_text(encoding='utf-8')
    if not permitted(body):
        return False
    goal = json.loads((oracle / 'task.json').read_text())['goal']
    if goal not in GOALS:
        raise ValueError('unrecognized trusted target')
    coqc, coqchk = shutil.which('coqc'), shutil.which('coqchk')
    if not coqc or not coqchk:
        raise RuntimeError('coqc/coqchk unavailable: infrastructure failure')
    with tempfile.TemporaryDirectory(prefix='coq-kernel-') as directory:
        work = Path(directory)
        (work / 'Submission.v').write_text(
            'Require Import Coq.Arith.PeanoNat.\nTheorem target : '+goal+
            '.\nProof.\n'+body+'\nQed.\nPrint Assumptions target.\n')
        env = {'PATH': os.environ.get('PATH', os.defpath), 'HOME': str(work)}
        compiled = subprocess.run([coqc, '-q', 'Submission.v'], cwd=work,
                                  env=env, capture_output=True, timeout=30)
        if compiled.returncode:
            return False
        # Print Assumptions is supplemental to actual kernel checking, not a
        # candidate-controlled report. Never accept admitted global premises.
        if compiled.stdout.strip() != b'Closed under the global context':
            return False
        if not (work / 'Submission.vo').is_file():
            return False
        kernel = subprocess.run([coqchk, '-silent', 'Submission'], cwd=work,
                                env=env, capture_output=True, timeout=30)
        return kernel.returncode == 0


if __name__ == '__main__':
    try:
        passed = check(Path(sys.argv[1]).resolve(), Path(sys.argv[2]).resolve())
    except (UnicodeError, subprocess.TimeoutExpired):
        passed = False
    except Exception as error:
        print(f'checker infrastructure error: {error}', file=sys.stderr)
        sys.exit(2)
    sys.exit(0 if passed else 1)
