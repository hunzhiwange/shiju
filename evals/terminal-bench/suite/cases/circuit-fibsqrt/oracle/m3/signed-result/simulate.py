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
            gate = [op, int(left[3:]), int(right[3:])]
        else:
            gate = ['copy', int(rhs[3:])]
        result.append(gate)
    if not result or any(ref >= len(result) for g in result for ref in (g[1:] if g[0] != 'const' else [])):
        raise ValueError('invalid reference')
    return result

def simulate(source, value, width=8, steps=9):
    gates = parse(source)
    values = [0] * len(gates)
    for i in range(width):
        values[i] = (value >> i) & 1
    for _ in range(steps):
        for i, gate in enumerate(gates):
            op, *refs = gate
            if op == 'const': result = refs[0]
            elif op == 'copy': result = values[refs[0]]
            elif op == 'not': result = 1 ^ values[refs[0]]
            elif op == '&': result = values[refs[0]] & values[refs[1]]
            elif op == '|': result = values[refs[0]] | values[refs[1]]
            else: result = values[refs[0]] ^ values[refs[1]]
            values[i] = result
    return sum(bit << i for i, bit in enumerate(values[-width:])) - ((1 << width) if values[-1] else 0)
if __name__ == '__main__':
    import json, sys
    value = json.load(sys.stdin)
    print(simulate(value['gates'], value['n'], value['width'], value['steps']))
