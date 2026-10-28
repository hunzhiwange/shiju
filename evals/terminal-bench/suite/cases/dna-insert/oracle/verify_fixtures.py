"""Independently recompute gold contracts from synthetic public inputs."""
import hashlib
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]

def rc(seq):
    return ''.join({'A':'T','C':'G','G':'C','T':'A'}[c] for c in reversed(seq))

def audit():
    for n in range(1, 5):
        m = f'm{n}'
        data = json.loads((ROOT / 'fixtures' / m / 'input.json').read_text())
        gold = json.loads((ROOT / 'oracle' / m / 'gold/result.json').read_text())
        if n == 1:
            template, target = data['input'], data['output']
            added = len(target) - len(template)
            solutions = []
            for boundary in range(len(template)):
                for rotation in range(len(target)):
                    unrotated = target[-rotation:] + target[:-rotation] if rotation else target
                    if unrotated[:boundary] + unrotated[boundary+added:] == template:
                        solutions.append({'boundary':boundary, 'insert':unrotated[boundary:boundary+added], 'rotation':rotation})
            assert solutions == [gold], solutions
        elif n == 2:
            candidates = []
            for f in range(15, 21):
                for r in range(15, 21):
                    forward, reverse = data['right'][:f], rc(data['left'][-r:])
                    ft = sum(4 if c in 'GC' else 2 for c in forward)
                    rt = sum(4 if c in 'GC' else 2 for c in reverse)
                    if 58 <= ft <= 64 and 58 <= rt <= 64 and abs(ft-rt) <= 2:
                        candidates.append(((f+r,f,r), {'forward':data['insert']+forward, 'reverse':reverse,
                            'forward_anneal_length':f, 'reverse_anneal_length':r, 'forward_tm':ft, 'reverse_tm':rt}))
            assert min(candidates)[1] == gold
        elif n == 3:
            assert gold == {'valid':[p['reverse'] == rc(p['left']) and p['forward'] == p['right'] for p in data['pairs']]}
        else:
            bases = list(data['input'])
            bases[data['boundary']:data['boundary']] = list(data['insert'])
            for _ in range(data['rotation']):
                bases.append(bases.pop(0))
            sequence = ''.join(bases)
            assert gold == {'sequence':sequence,'length':len(bases),'sha256':hashlib.sha256(sequence.encode('ascii')).hexdigest()}
    print('Four independent synthetic oracle audits passed')

if __name__ == '__main__':
    audit()
