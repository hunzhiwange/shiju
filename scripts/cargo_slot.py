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
def slot(root=ROOT):
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
    while True:
        for number in range(1, 21):
            lock = locks / f'slot-{number}.lock'
            try:
                lock.mkdir()
            except FileExistsError:
                continue
            break
        else:
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


def run(command, root=ROOT):
    if not command:
        raise RuntimeError('Usage: python3 scripts/cargo_slot.py COMMAND [ARGS...]')
    if os.name != 'posix':
        raise RuntimeError('Cargo slot runner currently supports macOS/Linux')
    # Install handlers before claiming the lock, including the wait-for-slot path.
    previous = {}

    def stop(_signum, _frame):
        raise KeyboardInterrupt

    try:
        for signum in (signal.SIGINT, signal.SIGTERM):
            previous[signum] = signal.signal(signum, stop)
        with slot(root) as env:
            child = subprocess.Popen(command, cwd=root, env=env, start_new_session=True)
            try:
                return child.wait()
            finally:
                # Finish cleanup even if the caller interrupts a second time.
                for signum in previous:
                    signal.signal(signum, signal.SIG_IGN)
                signal_group(child.pid, signal.SIGTERM)
                try:
                    child.wait(timeout=10)
                except subprocess.TimeoutExpired:
                    signal_group(child.pid, signal.SIGKILL)
                    child.wait()
                # A descendant can outlive the leader briefly. Zombies cannot
                # write artifacts; wait until no live group member remains.
                signal_group(child.pid, signal.SIGKILL)
                while group_alive(child.pid):
                    time.sleep(0.05)
    finally:
        for signum, handler in previous.items():
            signal.signal(signum, handler)


if __name__ == '__main__':
    try:
        sys.exit(run(sys.argv[1:]))
    except KeyboardInterrupt:
        sys.exit(130)
    except (RuntimeError, OSError, ValueError, KeyError) as exc:
        print(f'Cargo slot failed: {exc}', file=sys.stderr)
        sys.exit(1)
