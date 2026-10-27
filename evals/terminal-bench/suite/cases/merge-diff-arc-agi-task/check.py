"""Execute submissions and independently verify Git identity, ancestry and behavior."""
import copy
import importlib.util
import json
import os
from pathlib import Path
import random
import subprocess
import sys


def git(repo, *args):
    result = subprocess.run(['git', '-C', str(repo), *args], capture_output=True,
                            env={**os.environ, 'GIT_CONFIG_GLOBAL': '/dev/null',
                                 'GIT_CONFIG_NOSYSTEM': '1', 'GIT_TERMINAL_PROMPT': '0'},
                            timeout=10)
    assert result.returncode == 0, result.stderr.decode(errors='replace')
    return result.stdout.decode().strip()


def check_map(path, rotate):
    assert path.is_file(), 'algo.py missing'
    spec = importlib.util.spec_from_file_location('submitted_algo', path)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    assert callable(getattr(module, 'map', None)), 'map function missing'
    rng = random.Random(731)
    for height, width in [(1, 1), (1, 5), (6, 1), (2, 3), (4, 7), (3, 3)]:
        grid = [[rng.randrange(-9, 10) for _ in range(width)] for _ in range(height)]
        original = copy.deepcopy(grid)
        if rotate:
            expected = [[original[height - 1 - row][col] for row in range(height)]
                        for col in range(width)]
        else:
            expected = [[3 * value + 2 for value in row] for row in original]
        actual = module.map(grid)
        assert type(actual) is list and all(type(row) is list for row in actual), 'grid lists required'
        assert actual == expected, 'incorrect mapping'
        assert grid == original, 'input mutated'
        assert actual is not grid and all(row is not old for row in actual for old in grid), 'output aliases input'


def main():
    candidate, oracle = [Path(p).resolve() for p in sys.argv[1:]]
    expected = json.loads((oracle / 'expected.json').read_text())
    child = expected['child']
    if child == 'm4':
        check_map(candidate / 'algo.py', True)
        return
    assert child in ('m1', 'm2', 'm3')
    assert (candidate / 'setup.sh').is_file(), 'setup.sh missing'
    result = subprocess.run(['sh', str(candidate / 'setup.sh'), str(candidate)],
                            cwd=candidate, capture_output=True, timeout=20,
                            env={**os.environ, 'GIT_CONFIG_GLOBAL': '/dev/null',
                                 'GIT_CONFIG_NOSYSTEM': '1', 'GIT_TERMINAL_PROMPT': '0'})
    assert result.returncode == 0, result.stderr.decode(errors='replace')
    repo = candidate / 'repo'
    assert git(repo, 'rev-parse', '--is-inside-work-tree') == 'true'
    assert git(repo, 'symbolic-ref', '--short', 'HEAD') == 'branch1', 'wrong checkout'
    assert git(repo, 'rev-parse', 'branch2') == expected['right'], 'wrong second bundle tip'
    if child == 'm1':
        assert git(repo, 'rev-parse', 'branch1') == expected['left'], 'wrong first bundle tip'
    else:
        for tip in (expected['left'], expected['right']):
            git(repo, 'merge-base', '--is-ancestor', tip, 'HEAD')
        assert len(git(repo, 'rev-list', '--parents', '-n', '1', 'HEAD').split()) == 3, 'merge commit required'
        assert (repo / 'left.txt').read_text() == 'retain left\n'
        assert (repo / 'right.txt').read_text() == 'retain right\n'
    assert git(repo, 'status', '--porcelain') == '', 'dirty or unresolved worktree'
    if child == 'm3':
        # Importing Python can create bytecode; check cleanliness before execution.
        sys.dont_write_bytecode = True
        check_map(repo / 'algo.py', False)


if __name__ == '__main__':
    try:
        main()
    except (AssertionError, OSError, ValueError, AttributeError, TypeError,
            ImportError, SyntaxError, subprocess.TimeoutExpired) as error:
        print(str(error), file=sys.stderr)
        sys.exit(1)
