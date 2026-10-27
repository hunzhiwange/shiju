"""Independent gate evaluation and subprocess-based submission checks."""
import json
from pathlib import Path
import re
import subprocess
import sys


def unique_pairs(pairs):
    result = {}
    for key, value in pairs:
        if key in result:
            raise ValueError('duplicate JSON key')
        result[key] = value
    return result


def decode(text):
    return json.loads(text, object_pairs_hook=unique_pairs)


def parse(text):
    gates = []
    for index, line in enumerate(text.splitlines()):
        lhs, rhs = line.split(' = ')
        if lhs != f'out{index}':
            raise ValueError('nonconsecutive gate')
        if rhs in ('0', '1'):
            gate = ['const', int(rhs)]
        elif re.fullmatch(r'~out[0-9]+', rhs):
            gate = ['not', int(rhs[4:])]
        elif re.fullmatch(r'out[0-9]+', rhs):
            gate = ['copy', int(rhs[3:])]
        else:
            match = re.fullmatch(r'out([0-9]+) ([&|^]) out([0-9]+)', rhs)
            if not match:
                raise ValueError('bad binary gate')
            gate = [match[2], int(match[1]), int(match[3])]
        gates.append(gate)
    if not gates or any(ref >= len(gates) for gate in gates if gate[0] != 'const' for ref in gate[1:]):
        raise ValueError('reference bounds')
    return gates


def evaluate(gates, n, width, steps):
    state = [int(bool(n & (1 << i))) if i < width else 0 for i in range(len(gates))]
    for _ in range(steps):
        for index, gate in enumerate(gates):
            op, a = gate[:2]
            if op == 'const': value = a
            elif op == 'copy': value = state[a]
            elif op == 'not': value = int(not state[a])
            elif op == '&': value = int(bool(state[a]) and bool(state[gate[2]]))
            elif op == '|': value = int(bool(state[a]) or bool(state[gate[2]]))
            else: value = int(state[a] != state[gate[2]])
            state[index] = value
    return int(''.join(map(str, state[-width:][::-1])), 2)


def execute(app, program, value):
    result = subprocess.run([sys.executable, str(app / program)], input=value,
                            text=True, capture_output=True, cwd=app, timeout=3)
    return result


def check(app, task):
    output = app / task['output']
    if not output.is_file() or output.is_symlink() or output.stat().st_size > 100000:
        return False
    child = task['child']
    if child == 'm1':
        inputs = [
            'out0 = out0\nout1 = 1\nout2 = 0\nout3 = ~out1\nout4 = out0 & out1\nout5 = out1 | out2\nout6 = out4 ^ out5\nout7 = out6\n',
            'out0 = out1\nout1 = out1\n',
            'out0 = 0\nout1 = out0 ^ out0\n']
        for text in inputs:
            result = execute(app, 'parse.py', text)
            if result.returncode or json.dumps(decode(result.stdout)) != json.dumps(parse(text)):
                return False
        invalid = ['', 'out1 = 0\n', 'out0 = 2\n', 'out0 = out1\n',
                   'out0 = out0 + out0\n', 'out0 = ~out-1\n']
        return all(execute(app, 'parse.py', text).returncode != 0 for text in invalid)
    if child == 'm2':
        gates = parse(output.read_text())
        if not 16 <= len(gates) < 256 or gates[:8] != [['copy', i] for i in range(8)]:
            return False
        return all(evaluate(gates, n, 8, 9) == (n + 1) % 256 for n in range(256))
    if child == 'm3':
        # Self-copy inputs, both constant polarities, wraparound, zero-valued
        # internal storage and reverse dependencies requiring several sweeps.
        for width in (1, 3, 8):
            inputs = [f'out{i} = out{i}' for i in range(width)]
            identity = inputs + [f'out{width+i} = out{i}' for i in range(width)]
            inverted = inputs + [f'out{width+i} = ~out{i}' for i in range(width)]
            delayed = inputs + [f'out{width+i} = out{width+i+1}' for i in range(width)] + [f'out{2*width} = 1']
            feedback = inputs + [f'out{width+i} = ~out{width+i}' for i in range(width)]
            binary = inputs + [f'out{width+i} = out{i} {op} out{(i+1)%width}' for i, op in enumerate(('&', '|', '^') * 3) if i < width]
            for lines in (identity, inverted, delayed, feedback, binary):
                text = '\n'.join(lines) + '\n'
                for n, steps in ((0, 1), (1, 2), ((1 << width) - 1, 3), ((1 << width) + 2, 9)):
                    query = {'gates': text, 'n': n, 'width': width, 'steps': steps}
                    result = execute(app, 'simulate.py', json.dumps(query))
                    if result.returncode or result.stdout.strip() != str(evaluate(parse(text), n, width, steps)):
                        return False
        return True
    if child == 'm4':
        records = decode(output.read_text())
        inputs = decode((app / 'inputs.json').read_text())
        if type(records) is not list or len(records) != len(inputs):
            return False
        for n, record in zip(inputs, records):
            if type(record) is not dict or set(record) != {'n', 'actual', 'expected', 'equal'}:
                return False
            if any(type(record[key]) is not int for key in ('n', 'actual', 'expected')) or type(record['equal']) is not bool:
                return False
            result = subprocess.run([sys.executable, str(app / 'simulator.py'), str(n)],
                                    capture_output=True, text=True, cwd=app, timeout=3)
            if result.returncode or record != {'n': n, 'actual': int(result.stdout), 'expected': n ^ 165, 'equal': int(result.stdout) == (n ^ 165)}:
                return False
        return True
    raise ValueError('unknown child')


if __name__ == '__main__':
    app, oracle = map(Path, sys.argv[1:])
    task = decode((oracle / 'task.json').read_text())
    try:
        passed = check(app, task)
    except (ValueError, TypeError, KeyError, OSError, subprocess.TimeoutExpired):
        passed = False
    sys.exit(0 if passed else 1)
