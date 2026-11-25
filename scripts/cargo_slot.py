#!/usr/bin/env python3
"""Hold one Cargo slot across a command and all of its foreground children."""
import json
import os
from pathlib import Path
import signal
import re
import subprocess
import sys
import time
import uuid
from contextlib import contextmanager

ROOT = Path(__file__).resolve().parents[1]


@contextmanager
def slot(root=ROOT, number=None):
    if number is not None and number not in range(1, 21):
        raise RuntimeError('Cargo slot number must be between 1 and 20')
    root = root.resolve()
    locks = root / 'target/rust-slot-locks'
    if (root / 'target').is_symlink() or locks.is_symlink():
        raise RuntimeError('Refusing Cargo slots through a symlink')
    locks.mkdir(parents=True, exist_ok=True)
    inherited = os.environ.get('SHIJU_CARGO_SLOT_TOKEN')
    if os.environ.get('CARGO_TARGET_DIR'):
        target = Path(os.environ.get('CARGO_TARGET_DIR', ''))
        if (not target.is_absolute() or target.parent != root / 'target'
                or target.name not in {f'rust-slot-{n}' for n in range(1, 21)}
                or target.is_symlink()):
            raise RuntimeError('Invalid inherited Cargo slot path')
        lock = locks / f'slot-{target.name.removeprefix("rust-slot-")}.lock'
        if lock.is_symlink():
            raise RuntimeError('Refusing Cargo slot lock through a symlink')
        if inherited:
            receipt = json.loads((lock / 'owner.json').read_text())
            if receipt['token'] != inherited:
                raise RuntimeError('Inherited Cargo slot owner does not match')
            os.kill(receipt['pid'], 0)
        else:
            # Existing task plans record ownership as either JSON or plain text.
            owner = lock / 'owner.json'
            if owner.is_file():
                pid = json.loads(owner.read_text())['pid']
            else:
                match = re.search(r'\bpid[=:]\s*(\d+)', (lock / 'owner').read_text(), re.I)
                if not match:
                    raise RuntimeError('Inherited Cargo slot has no owner PID')
                pid = int(match[1])
            ancestor = os.getpid()
            while ancestor > 1 and ancestor != pid:
                ancestor = int(subprocess.check_output(
                    ['ps', '-o', 'ppid=', '-p', str(ancestor)], text=True).strip())
            if pid <= 1 or ancestor != pid:
                raise RuntimeError('Inherited Cargo slot is not owned by this process or an ancestor')
        env = os.environ.copy()
        env['CARGO_BUILD_BUILD_DIR'] = str(target)
        env['SHIJU_CARGO_SLOT_ACTIVE'] = '1'
        yield env
        return
    candidates = (number,) if number is not None else range(1, 21)
    next_notice = 0
    while True:
        attempted = candidates
        for number in attempted:
            lock = locks / f'slot-{number}.lock'
            try:
                lock.mkdir()
            except FileExistsError:
                if lock.is_symlink():
                    raise RuntimeError(f'Refusing Cargo slot lock through a symlink: {lock}')
                if len(candidates) == 1:
                    try:
                        pid = json.loads((lock / 'owner.json').read_text())['pid']
                        if type(pid) is int and pid > 1:
                            os.kill(pid, 0)
                    except ProcessLookupError:
                        # An orphaned child may still use the old cache.
                        print(f'Cargo slot {number} owner PID {pid} exited; using a free slot.',
                              file=sys.stderr, flush=True)
                        candidates = tuple(n for n in range(1, 21) if n != number)
                    except (OSError, ValueError, KeyError, TypeError):
                        pass
                continue
            break
        else:
            if attempted != candidates:
                continue
            now = time.monotonic()
            if now >= next_notice:
                location = lock if len(candidates) == 1 else locks
                print(f'Waiting for Cargo slot: {location} (Ctrl-C to cancel)',
                      file=sys.stderr, flush=True)
                next_notice = now + 15
            time.sleep(1)
            continue
        break
    try:
        target = root / f'target/rust-slot-{number}'
        if target.is_symlink():
            raise RuntimeError('Refusing Cargo slot target through a symlink')
        token = uuid.uuid4().hex
        (lock / 'owner.json').write_text(json.dumps(dict(
            plan='开发缓存统一', task='command', session=token, token=token,
            pid=os.getpid())))
        env = os.environ.copy()
        env.update(CARGO_TARGET_DIR=str(target), CARGO_BUILD_BUILD_DIR=str(target),
                   SHIJU_CARGO_SLOT_TOKEN=token, SHIJU_CARGO_SLOT_ACTIVE='1')
        print(f'Cargo slot {number}: {target}', file=sys.stderr, flush=True)
        yield env
    finally:
        (lock / 'owner.json').unlink(missing_ok=True)
        lock.rmdir()


def group_alive(pid):
    states = subprocess.check_output(['ps', '-ax', '-o', 'pgid=,stat='], text=True)
    return any(int(fields[0]) == pid and not fields[1].startswith('Z')
               for line in states.splitlines() if len(fields := line.split()) == 2)


def signal_group(pid, signum):
    try:
        os.killpg(pid, signum)
    except ProcessLookupError:
        pass
    except PermissionError:
        # During macOS exit/reparenting the group can briefly reject signals
        # before ps stops reporting its live members. Wait for that transition;
        # a persistently live group still raises the original permission error.
        for _ in range(20):
            if not group_alive(pid):
                return
            time.sleep(0.01)
        raise


@contextmanager
def stop_on_signal(stop):
    previous = {}
    try:
        for signum in (signal.SIGINT, signal.SIGTERM):
            previous[signum] = signal.signal(signum, stop)
        yield
    finally:
        for signum, handler in previous.items():
            signal.signal(signum, handler)


@contextmanager
def foreground_process(command, cwd, env, *, shutdown_timeout=10):
    child = None
    interrupted = False
    cleaning_up = False

    def stop(_signum, _frame):
        nonlocal interrupted
        # If a signal arrives during Popen, wait until we have the group ID.
        interrupted = True
        if child is not None and not cleaning_up:
            raise KeyboardInterrupt

    with stop_on_signal(stop):
        try:
            child = subprocess.Popen(command, cwd=cwd, env=env, start_new_session=True)
            if interrupted:
                raise KeyboardInterrupt
            yield child
        finally:
            cleaning_up = True
            if child is not None:
                signal_group(child.pid, signal.SIGTERM)
                try:
                    child.wait(timeout=shutdown_timeout)
                except subprocess.TimeoutExpired:
                    signal_group(child.pid, signal.SIGKILL)
                    child.wait()
                # Descendants can outlive the leader. Wait until the group
                # has no live process that can keep using the slot.
                signal_group(child.pid, signal.SIGKILL)
                while group_alive(child.pid):
                    time.sleep(0.05)
            cleaning_up = False
            if interrupted:
                raise KeyboardInterrupt


def run_foreground(command, cwd, env, *, shutdown_timeout=10):
    with foreground_process(command, cwd, env, shutdown_timeout=shutdown_timeout) as child:
        return child.wait()


def run(command, root=ROOT, number=None):
    if not command:
        raise RuntimeError('Usage: python3 scripts/cargo_slot.py COMMAND [ARGS...]')
    if os.name != 'posix':
        raise RuntimeError('Cargo slot runner currently supports macOS/Linux')

    def stop(_signum, _frame):
        raise KeyboardInterrupt

    # Also handle interruption while waiting to claim the slot.
    with stop_on_signal(stop):
        with slot(root, number=number) as env:
            return run_foreground(command, root, env)


if __name__ == '__main__':
    try:
        sys.exit(run(sys.argv[1:]))
    except KeyboardInterrupt:
        sys.exit(130)
    except (RuntimeError, OSError, ValueError, KeyError) as exc:
        print(f'Cargo slot failed: {exc}', file=sys.stderr)
        sys.exit(1)
