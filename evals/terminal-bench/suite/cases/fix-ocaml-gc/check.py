"""Independent executable probes for synthetic compressed heaps; no parent imports."""
import json
from pathlib import Path
import random
import subprocess
import sys


def probes(child):
    rng = random.Random(4081)
    if child == 'm1':
        for headers in [[], [0], [-1], [3, -7, 0, -1], [-1000000, 999999]] + [
                [rng.randint(-1000, 1000) for _ in range(20)] for _ in range(10)]:
            yield {'headers': headers}, [dict(free=h < 0, words=abs(h) + (h >= 0)) for h in headers]
    elif child == 'm2':
        yield {'heap': []}, []
        for _ in range(25):
            heap, expected = [], []
            for _ in range(rng.randint(1, 10)):
                words = rng.randint(1, 15)
                free = bool(rng.randrange(2))
                expected.append(dict(offset=len(heap), words=words, free=free))
                heap += [-words if free else words - 1] + [rng.choice([-99999, 99999, 0])] * (words - 1)
            yield {'heap': heap}, expected
        for heap in [[-2], [1], [0, -3, 99], [-1, 5, -1], [3, 0, 0]]:
            yield {'heap': heap}, {'error': 'bounds'}
    elif child == 'm3':
        yield {'capacity': 0, 'blocks': [], 'cursor': 0}, {'valid': True}
        yield {'capacity': 0, 'blocks': [], 'cursor': -1}, {'valid': False}
        blocks = [dict(offset=0, words=3, free=False), dict(offset=3, words=5, free=True),
                  dict(offset=8, words=2, free=False)]
        for cursor in [-1, 0, 1, 3, 7, 8, 9, 10, 11]:
            yield dict(capacity=10, blocks=blocks, cursor=cursor), {'valid': cursor in (0, 3, 8, 10)}
        for bad in [[], blocks[:-1], list(reversed(blocks)),
                    [dict(offset=0, words=10, free=1)],
                    [dict(offset=0, words=4, free=True), dict(offset=4, words=6, free=True)],
                    [dict(offset=0, words=0, free=False), dict(offset=0, words=10, free=False)],
                    [dict(offset=0, words=-1, free=False)],
                    [dict(offset=0, words=4, free=False), dict(offset=5, words=5, free=True)],
                    [dict(offset=0, words=7, free=False), dict(offset=6, words=4, free=True)],
                    [dict(offset=0, words=11, free=False)]]:
            yield dict(capacity=10, blocks=bad, cursor=0), {'valid': False}
        for words in range(1, 17):
            b = [dict(offset=0, words=words, free=bool(words % 2))]
            yield dict(capacity=words, blocks=b, cursor=words), {'valid': True}
            yield dict(capacity=words + 1, blocks=b, cursor=0), {'valid': False}
    else:
        scripts = [(8, []), (4, [['alloc', 'a', 3], ['alloc', 'b', 0]]),
                   (5, [['alloc', 'a', 3], ['alloc', 'b', 0]]),
                   (12, [['alloc', 'a', 2], ['alloc', 'b', 2], ['alloc', 'c', 2],
                         ['free', 'a'], ['free', 'c'], ['free', 'b'], ['alloc', 'd', 11]]),
                   (12, [['alloc', 'a', 2], ['alloc', 'b', 2], ['alloc', 'c', 2],
                         ['free', 'a'], ['free', 'c'], ['alloc', 'd', 1], ['alloc', 'e', 20]])]
        for seed in range(15):
            local = random.Random(seed)
            ops, active = [], []
            # Free only IDs whose success is known from the independent simulator.
            for index in range(25):
                if active and local.randrange(3) == 0:
                    name = local.choice(active); active.remove(name); ops.append(['free', name])
                else:
                    name = str(index); ops.append(['alloc', name, local.randrange(7)])
                    if allocation_reference(24, ops)['results'][-1] is not None:
                        active.append(name)
            scripts.append((24, ops))
        for capacity, ops in scripts:
            yield dict(capacity=capacity, ops=ops), allocation_reference(capacity, ops)


def allocation_reference(capacity, operations):
    """Word ownership oracle, independent of candidate interval-list algorithms."""
    cells = [None] * capacity
    answers = []
    for op in operations:
        if op[0] == 'free':
            cells = [None if owner == op[1] else owner for owner in cells]
            answers.append(None)
        else:
            needed = op[2] + 1
            start = next((i for i in range(capacity - needed + 1)
                          if all(c is None for c in cells[i:i + needed])), None)
            if start is not None:
                cells[start:start + needed] = [op[1]] * needed
            answers.append(start)
    blocks, offset = [], 0
    while offset < capacity:
        end = offset + 1
        while end < capacity and cells[end] == cells[offset]:
            end += 1
        blocks.append(dict(offset=offset, words=end - offset, free=cells[offset] is None))
        offset = end
    return dict(results=answers, blocks=blocks)


def main():
    app, oracle = map(Path, sys.argv[1:])
    child = json.loads((oracle / 'task.json').read_text())['child']
    program = app / 'solution.py'
    if not program.is_file() or program.is_symlink():
        return 1
    for request, expected in probes(child):
        try:
            result = subprocess.run([sys.executable, str(program)], input=json.dumps(request).encode(),
                                    capture_output=True, timeout=3, cwd=app)
            if result.returncode or len(result.stdout) > 100000:
                return 1
            actual = json.loads(result.stdout)
        except (subprocess.TimeoutExpired, ValueError, UnicodeError):
            return 1
        # JSON serialization distinguishes booleans from numbers and rejects extra fields.
        if json.dumps(actual, sort_keys=True) != json.dumps(expected, sort_keys=True):
            return 1
    return 0


if __name__ == '__main__':
    sys.exit(main())
