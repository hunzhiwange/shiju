"""Run real synthetic allocator programs; reject immutable-input drift."""
import hashlib
import json
from pathlib import Path
import resource
import subprocess
import sys
import tempfile


def unique(pairs):
    result = {}
    for key, value in pairs:
        if key in result:
            raise ValueError('duplicate key')
        result[key] = value
    return result


def read_json(path):
    if path.is_symlink() or not path.is_file() or path.stat().st_size > 32768:
        raise ValueError('invalid artifact')
    return json.loads(path.read_text(), object_pairs_hook=unique)


def no_core():
    resource.setrlimit(resource.RLIMIT_CORE, (0, 0))


def execute(command, cwd):
    return subprocess.run(command, cwd=cwd, capture_output=True, timeout=15,
                          preexec_fn=no_core)


def check(candidate, oracle):
    task = read_json(oracle / 'task.json')
    try:
        for name, expected in task['input_sha256'].items():
            path = candidate / name
            if path.is_symlink() or hashlib.sha256(path.read_bytes()).hexdigest() != expected:
                return False
        kind = task['kind']
        if kind == 2:
            value = read_json(candidate / 'diagnosis.json')
            return json.dumps(value, sort_keys=True) == json.dumps(task['expected'], sort_keys=True)
        user = candidate / 'user.cpp'
        if user.is_symlink() or not user.is_file() or user.stat().st_size > 32768:
            return False
        with tempfile.TemporaryDirectory(prefix='.heap-check-', dir=candidate) as tmp:
            runs = []
            guard_flags = []
            if kind >= 3:
                # Link-time probes reject direct early termination even when a
                # candidate prints plausible finalizer output and has no leaks.
                guard = Path(tmp) / 'exit_guard.cpp'
                guard.write_text('#include <cstdlib>\n' + ''.join(
                    'extern "C" [[noreturn]] void __wrap_' + name +
                    '(int) { std::abort(); }\n'
                    for name in ('exit', '_exit', '_Exit', 'quick_exit')))
                guard_flags = [str(guard), '-Wl,--wrap=exit,--wrap=_exit,--wrap=_Exit,--wrap=quick_exit']
            for mode, flags in [('debug', ['-O0']), ('release', ['-O2', '-DNDEBUG'])]:
                binary = str(Path(tmp) / mode)
                result = execute(['g++', '-std=c++17', '-o', binary,
                                  str(candidate / 'main.cpp'), str(user), *guard_flags, *flags], tmp)
                if result.returncode:
                    return False
                run = execute([binary], tmp)
                runs.append(run)
                if kind >= 3:
                    if run.returncode or run.stdout.splitlines() != [b'work-complete', b'finalizer', b'cleanup-complete']:
                        return False
                    leak = execute(['valgrind', '--quiet', '--leak-check=full',
                                    '--errors-for-leak-kinds=definite', '--error-exitcode=19', binary], tmp)
                    if leak.returncode:
                        return False
            if kind == 1:
                actual = {'debug_exit': runs[0].returncode, 'release_signal': -runs[1].returncode}
                submitted = read_json(candidate / 'repro.json')
                return (actual == task['expected'] and
                        json.dumps(submitted, sort_keys=True) == json.dumps(actual, sort_keys=True))
            for name, expected in task['input_sha256'].items():
                path = candidate / name
                if path.is_symlink() or hashlib.sha256(path.read_bytes()).hexdigest() != expected:
                    return False
            return True
    except (OSError, ValueError, UnicodeError, subprocess.TimeoutExpired):
        return False


if __name__ == '__main__':
    sys.exit(0 if check(Path(sys.argv[1]).resolve(), Path(sys.argv[2]).resolve()) else 1)
