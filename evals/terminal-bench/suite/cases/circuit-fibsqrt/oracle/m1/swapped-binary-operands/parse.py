import re

def parse(source):
    result = []
    for index, line in enumerate(source.splitlines()):
        match = re.fullmatch(r'out(\d+) = (0|1|~out\d+|out\d+(?: [\&|^] out\d+)?)', line)
        if not match or int(match[1]) != index:
            raise ValueError('invalid gate')
        rhs = match[2]
        if rhs in ('0', '1'):
            gate = ['const', int(rhs)]
        elif rhs.startswith('~'):
            gate = ['not', int(rhs[4:])]
        elif ' ' in rhs:
            left, op, right = rhs.split()
            gate = [op, int(right[3:]), int(left[3:])]
        else:
            gate = ['copy', int(rhs[3:])]
        result.append(gate)
    if not result or any(ref >= len(result) for g in result for ref in (g[1:] if g[0] != 'const' else [])):
        raise ValueError('invalid reference')
    return result
if __name__ == '__main__':
    import json, sys
    try: print(json.dumps(parse(sys.stdin.read())))
    except ValueError: sys.exit(1)
