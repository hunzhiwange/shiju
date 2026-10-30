"""Black-box synthetic classification controls, independent of candidate tests."""
import json
import math
from pathlib import Path
import struct
import subprocess
import sys
import tempfile


def close(actual, expected):
    if isinstance(expected, dict):
        return isinstance(actual, dict) and actual.keys() == expected.keys() and all(close(actual[k], v) for k, v in expected.items())
    if isinstance(expected, list):
        return isinstance(actual, list) and len(actual) == len(expected) and all(close(a, b) for a, b in zip(actual, expected))
    return type(actual) in (int, float) and math.isfinite(actual) and math.isclose(actual, expected, rel_tol=1e-7, abs_tol=1e-7)


def check(app, oracle):
    child = json.loads((oracle / 'task.json').read_text())['child']
    program = app / 'answer.py'
    if not program.is_file():
        return False
    with tempfile.TemporaryDirectory() as directory:
        work = Path(directory)
        def run(args, expected=None, error=False):
            try:
                result = subprocess.run([sys.executable, str(program), *map(str, args)], cwd=work, capture_output=True, timeout=8)
            except subprocess.TimeoutExpired:
                return False
            if error:
                return result.returncode != 0
            if result.returncode:
                return False
            try:
                return close(json.loads(result.stdout), expected)
            except (ValueError, TypeError):
                return False
        data = work / 'input.json'
        if child == 'm1':
            binary = work / 'records.bin'
            record = bytes([3]) + bytes([11])*1024 + bytes([27])*1024 + bytes([91])*1024
            binary.write_bytes(record + bytes([9]) + bytes(3072) + record)
            expected = {'shape':[3,3,32,32], 'histogram':[0,0,0,2,0,0,0,0,0,1], 'first_rgb':[11,27,91], 'classifier_outputs':10}
            if not run([binary,10], expected) or not run([binary,9], error=True):
                return False
            for payload in (b'', record[:-1], bytes([10])+record[1:]):
                binary.write_bytes(payload)
                if not run([binary,10], error=True):
                    return False
            binary.write_bytes(bytes([0])+bytes([7])*3072)
            return run([binary,10], {'shape':[1,3,32,32], 'histogram':[1,0,0,0,0,0,0,0,0,0], 'first_rgb':[7,7,7], 'classifier_outputs':10})
        if child == 'm2':
            # Equal logits give exactly uniform p; nonzero inputs exercise all gradients.
            for weights,x,label,lr in [([[0,0],[0,0]],[2,-1],1,0.2), ([[1000,0],[1000,0]],[1,3],1,0.1), ([[1,2],[1,2],[1,2]],[-2,4],2,0.3)]:
                c=len(weights);p=1/c
                data.write_text(json.dumps({'weights':weights,'x':x,'label':label,'lr':lr}))
                expected={'probabilities':[p]*c, 'loss':math.log(c), 'weights':[[a-lr*(p-int(i==label))*b for a,b in zip(row,x)] for i,row in enumerate(weights)]}
                if not run([data],expected):
                    return False
            # Unequal logits independently derived from exp(0), exp(log(3)).
            data.write_text(json.dumps({'weights':[[0],[math.log(3)]],'x':[1],'label':0,'lr':0.4}))
            return run([data], {'probabilities':[0.25,0.75], 'loss':math.log(4), 'weights':[[0.3],[math.log(3)-0.3]]})
        if child == 'm3':
            # Precomputed LCG sequence, independently fixed control values.
            controls=[(1,3,0,[0,0,2],0.625,2165703038),(0,3,0,[1,2,1],0.5,3519870697),(1,0,0,[],0,1),(1,1,4,[0],1.5,1015568748),(99,0,-2.25,[],-2.25,99)]
            for seed,steps,initial,indices,weight,state in controls:
                data.write_text(json.dumps({'seed':seed,'initial':initial,'targets':[-1,0,2],'steps':steps,'lr':0.5}))
                expected={'iterations':steps,'indices':indices,'weight':weight,'state':state}
                if not run([data],expected) or not run([data],expected):
                    return False
            return True
        if child == 'm4':
            output=work/'weights.bin'
            for weights,bias,x in [([[1,2],[-1,3]],[0.5,-0.5],[2,1]), ([[1],[1],[0]],[2,2,-3],[1]), ([[0.1,-0.2,0.3]],[0.4],[1,2,-1])]:
                data.write_text(json.dumps({'weights':weights,'bias':bias,'x':x}))
                c,d=len(weights),len(weights[0]);flat=[v for row in weights for v in row]+bias
                blob=b'VWCF'+struct.pack('<II',c,d)+struct.pack('<'+'f'*len(flat),*flat)
                rounded=struct.unpack('<'+'f'*len(flat),blob[12:])
                logits=[sum(rounded[i*d+j]*x[j] for j in range(d))+rounded[c*d+i] for i in range(c)]
                output.write_bytes(b'stale'*100)
                if not run([data,output], {'logits':logits,'prediction':max(range(c),key=lambda i:logits[i])}) or not output.is_file() or output.read_bytes()!=blob:
                    return False
            return True
        raise ValueError('unknown child')


if __name__ == '__main__':
    sys.exit(0 if check(Path(sys.argv[1]).resolve(), Path(sys.argv[2]).resolve()) else 1)
