"""Black-box synthetic sequence diagnostics; no upstream material or network."""
import os
import sys

# Harbor starts in /app: candidate select.py must not shadow stdlib select.
sys.path[:] = [entry for entry in sys.path
               if entry and os.path.realpath(entry) != os.path.realpath(os.getcwd())]

import json
from pathlib import Path
import random
import re
import subprocess


def probes(kind, seed):
    rng = random.Random(seed)
    if kind == 'm1':
        for i in range(6):
            records = {'alpha': 'MAVMKD', 'beta': 'GEAMVK', 'gamma': 'MMKFA'}
            records = {k+str(i): v for k, v in records.items()}
            fasta = '\n\n'.join('>'+k+' synthetic description\n'+v[:3].lower()+'\n'+v[3:]
                                for k, v in records.items())
            ex, em = rng.randrange(450, 540), rng.randrange(580, 660)
            yield {'fasta': fasta, 'filter': {'excitation_nm': ex, 'emission_nm': em}}, {
                'sequences': {k: v.removeprefix('M') for k, v in records.items()},
                'donor_excitation_nm': ex, 'acceptor_emission_nm': em}
    elif kind == 'm2':
        for i in range(8):
            ex, em = 470+i, 601+i
            ligand = 'ligand-'+str(i)
            rows = []
            def row(name, x, y, lig=None):
                return dict(id=name+str(i), sequence='MAVKEL', excitation_nm=x,
                            emission_nm=y, ligand=lig)
            # Forbidden exact matches precede the allowed matches, and wrong-axis
            # decoys precede the donor. Rotate allowed positions independently.
            forbidden = [row('forbidden-d', ex, em), row('forbidden-b', None, None, ligand)]
            allowed = [row('cross', ex+20, ex), row('near', ex+1, em+1),
                       row('donor', ex, em+30), row('acceptor', ex+90, em),
                       row('binder', None, None, ligand)]
            rng.shuffle(allowed)
            rows = forbidden + allowed
            yield {'allowed': [r['id'] for r in allowed], 'proteins': rows,
                   'target': dict(excitation_nm=ex, emission_nm=em, ligand=ligand)}, {
                       'donor': 'donor'+str(i), 'acceptor': 'acceptor'+str(i),
                       'binder': 'binder'+str(i)}
    elif kind == 'm3':
        valid = ['GSGSG', 'S'*20, 'G'*5, 'SG'*7]
        cases = [('ATGC'*30, valid, True),
                 ('GC'*7+'G'+'A'*35, valid, True),
                 ('GC'*17+'G'+'A'*15, valid, True),
                 ('ATGC'*13+'A'*60, valid, False),
                 ('ATGC'*13+'G'*60, valid, False),
                 ('ATGC'*30, ['G'*4]+valid[1:], False),
                 ('ATGC'*30, ['G'*21]+valid[1:], False),
                 ('ATGC'*30, ['GGASG']+valid[1:], False),
                 ('ATGC'*30, valid[:3], False),
                 ('ATGC'*30, valid+['G'*5], False),
                 ('ATGC'*12, valid, False),
                 ('ATGC'*20+'N', valid, False),
                 ('atgc'*30, valid, False)]
        for dna, linkers, expected in cases:
            yield dict(dna=dna, linkers=linkers), expected
    else:
        raise ValueError('unknown program diagnostic')


def program(app, kind, seed):
    entry = app / {'m1': 'parse.py', 'm2': 'select.py', 'm3': 'validate.py'}[kind]
    if entry.is_symlink() or not entry.is_file() or entry.stat().st_size > 65536:
        return False
    for data, expected in probes(kind, seed):
        result = subprocess.run([sys.executable, '-I', str(entry)], cwd=app,
                                input=json.dumps(data).encode(), capture_output=True,
                                timeout=5, env={'PATH': os.defpath, 'PYTHONDONTWRITEBYTECODE': '1'})
        if result.returncode:
            return False
        try:
            actual = json.loads(result.stdout)
        except (ValueError, UnicodeError):
            return False
        if type(actual) is not type(expected) or actual != expected:
            return False
    return True


# Standard genetic code, in T/C/A/G codon ordering. Independent translation
# accepts every synonymous coding, rather than comparing a gold DNA string.
BASES = 'TCAG'
AMINO = 'FFLLSSSSYY**CC*WLLLLPPPPHHQQRRRRIIIMTTTTNNKKSSRRVVVVAAAADDEEGGGG'
CODE = {a+b+c: AMINO[i*16+j*4+k]
        for i, a in enumerate(BASES) for j, b in enumerate(BASES)
        for k, c in enumerate(BASES)}


def delivery(app):
    artifact = app / 'gblock.txt'
    if artifact.is_symlink() or not artifact.is_file() or artifact.stat().st_size > 3001:
        return False
    raw = artifact.read_bytes()
    if not re.fullmatch(b'[ACGT]+\n?', raw):
        return False
    dna = raw.rstrip(b'\n').decode('ascii')
    if not 50 <= len(dna) <= 3000 or len(dna) % 3:
        return False
    aa = ''.join(CODE[dna[i:i+3]] for i in range(0, len(dna), 3))
    # Checker-owned synthetic domain contract; not read from mutable app input.
    domains = ['DKEAM', 'VAKEL', 'FKDAE', 'EAKVD', 'KAFDE']
    pattern = '[GS]{5,20}'.join(map(re.escape, domains))
    if not re.fullmatch(pattern, aa):
        return False
    gc = sum(c in 'GC' for c in dna[:50])
    if not 15 <= gc <= 35:
        return False
    for i in range(50, len(dna)):
        gc += (dna[i] in 'GC') - (dna[i-50] in 'GC')
        if not 15 <= gc <= 35:
            return False
    return True


def check(app, oracle):
    task = json.loads((oracle / 'task.json').read_text())
    if task['kind'] == 'm4':
        return delivery(app)
    return program(app, task['kind'], task['seed'])


if __name__ == '__main__':
    try:
        passed = check(Path(sys.argv[1]).resolve(), Path(sys.argv[2]).resolve())
    except (OSError, subprocess.TimeoutExpired):
        passed = False
    sys.exit(0 if passed else 1)
