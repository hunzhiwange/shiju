#!/usr/bin/env python3
"""Rebuild on edits; keep the current application alive until a build succeeds.

macOS/Linux only. Watch crates/, .cargo/, and root Cargo
manifests. Ctrl-C stops this watcher and the process groups it owns. Compilation
errors preserve the running version; startup errors after replacement are logged
and retried on the next edit. In-memory application state is not preserved.
"""
import argparse
import fcntl
import os
import signal
import shutil
import threading
import subprocess
import sys
import time

from pathlib import Path

from cargo_slot import group_alive, signal_group

ROOT = Path(__file__).resolve().parents[1]
QUIET_SECONDS = 30.0
MAX_WAIT_SECONDS = 60.0
MAX_UPDATE_WAIT_SECONDS = 1800.0

def start_watcher(changed):
    command = ['watchexec', '--quiet', '--postpone', '--debounce', '500ms',
               '--fs-events', 'create,remove,rename,modify',
               '--on-busy-update', 'queue', '--shell=none']
    for path in ('crates', '.cargo', 'Cargo.toml', 'Cargo.lock', 'rust-toolchain.toml', 'rust-toolchain'):
        if (ROOT / path).exists():
            command += ['--watch', path]
    # A short command emits notifications to our pipe. Watchexec owns all file
    # watching and event batching; it never owns or restarts the application.
    command += ['--', sys.executable, '-c', "print('SHIJU_SOURCE_CHANGED', flush=True)"]
    watcher = subprocess.Popen(command, cwd=ROOT, stdout=subprocess.PIPE,
                               text=True, start_new_session=True)

    def receive():
        try:
            for line in watcher.stdout:
                if line.strip() == 'SHIJU_SOURCE_CHANGED':
                    changed.set()
        finally:
            watcher.stdout.close()
            changed.set()  # Wake the supervisor if Watchexec exits.

    threading.Thread(target=receive, daemon=True).start()
    return watcher


def stop(process):
    if process is None:
        return
    signal_group(process.pid, signal.SIGINT)
    deadline = time.monotonic() + 10
    while group_alive(process.pid) and time.monotonic() < deadline:
        process.poll()
        time.sleep(0.05)
    signal_group(process.pid, signal.SIGKILL)
    process.wait()
    while group_alive(process.pid):
        time.sleep(0.05)


def build(mode, env, lock, changed, watcher, initial=False):
    # Both watch targets share this lock: never compile their heavy Rust targets
    # concurrently. Build and run inherit the same normal Cargo environment.
    while True:
        try:
            fcntl.flock(lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
            break
        except BlockingIOError:
            time.sleep(0.2)
    try:
        # First launch is immediate. Later builds debounce with a hard deadline,
        # so continuous edits cannot starve updates. Never cancel an active build.
        changed.clear()
        if not initial:
            deadline = time.monotonic() + MAX_WAIT_SECONDS
            while True:
                remaining = deadline - time.monotonic()
                if remaining <= 0 or not changed.wait(timeout=min(QUIET_SECONDS, remaining)):
                    break
                changed.clear()
                if watcher.poll() is not None:
                    raise RuntimeError('Watchexec 已退出')
        if watcher.poll() is not None:
            raise RuntimeError('Watchexec 已退出')
        reason = '首次立即编译' if initial else '静默期或最长等待期结束，开始编译'
        print(f'[watch] {reason} {mode}', flush=True)
        targets = [('vw-cli', 'shiju')] if mode == 'gateway' else [
            ('vw-webview', 'vw-webview'), ('vw-desktop', 'shi-ju')]
        for package, binary in targets:
            process = subprocess.Popen(['cargo', 'build', '-p', package, '--bin', binary],
                                       cwd=ROOT, env=env, start_new_session=True)
            try:
                if process.wait() != 0:
                    print('[watch] 编译失败，保留旧进程；修改后自动重试。', flush=True)
                    return False
            finally:
                stop(process)
        return True
    finally:
        fcntl.flock(lock, fcntl.LOCK_UN)


def request_restart(mode):
    state = ROOT / 'target/dev-watch'
    lock_path = state / f'{mode}.lock'
    if not lock_path.exists():
        raise RuntimeError(f'请先启动 make {mode}-watch')
    with lock_path.open('a') as instance:
        try:
            fcntl.flock(instance, fcntl.LOCK_EX | fcntl.LOCK_NB)
        except BlockingIOError:
            # A separate command never reads or intercepts application stdin.
            request = state / f'{mode}.restart'
            temporary = state / f'{mode}.{os.getpid()}.tmp'
            try:
                temporary.write_text(str(time.time_ns()))
                os.replace(temporary, request)
            finally:
                temporary.unlink(missing_ok=True)
            print(f'[watch] 已请求切换 {mode}；处理结果请查看监听终端。')
            return
        raise RuntimeError(f'请先启动 make {mode}-watch')


def watch(mode):
    state = ROOT / 'target/dev-watch'
    state.mkdir(parents=True, exist_ok=True)
    # Do not accidentally start a second watcher for the same application.
    with (state / f'{mode}.lock').open('a') as instance:
        try:
            fcntl.flock(instance, fcntl.LOCK_EX | fcntl.LOCK_NB)
        except BlockingIOError:
            raise RuntimeError(f'{mode} watcher 已在运行')
        request = state / f'{mode}.restart'
        request.unlink(missing_ok=True)
        with (state / 'build.lock').open('a') as lock:
            env = os.environ.copy()
            running = None
            changed = threading.Event()
            watcher = start_watcher(changed)
            changed.set()
            initial = True
            ready = False
            ready_since = None
            last_request = None
            try:
                while True:
                    if watcher.poll() is not None:
                        raise RuntimeError(f'Watchexec 已退出 ({watcher.returncode})')
                    if changed.wait(timeout=0.5):
                        first_build = initial
                        initial = False
                        ready = build(mode, env, lock, changed, watcher, initial=first_build)
                        if ready:
                            if ready_since is None:
                                ready_since = time.monotonic()
                            if running is not None:
                                print(f'[watch] 新版就绪：执行 make {mode}-restart 切换；'
                                      '最迟在首个待切换版本就绪 30 分钟后自动切换。', flush=True)
                    manual = False
                    try:
                        token = request.read_text()
                    except FileNotFoundError:
                        token = None
                    if token is not None and token != last_request:
                        last_request = token
                        manual = True
                        if not ready:
                            print('[watch] 当前没有编译成功的待切换版本，保留旧进程。', flush=True)
                    expired = ready_since is not None and (
                        time.monotonic() - ready_since >= MAX_UPDATE_WAIT_SECONDS)
                    if ready and (running is None or manual or expired):
                        if watcher.poll() is not None:
                            raise RuntimeError('Watchexec 已退出')
                        # Recheck current sources before replacing the running app:
                        # cargo run must not discover a known compile error afterwards.
                        if running is not None:
                            if not build(mode, env, lock, changed, watcher, initial=True):
                                ready = False
                                continue
                        stop(running)
                        running = None
                        if mode == 'gateway':
                            command = ['cargo', 'run', '-p', 'vw-cli', '--bin', 'shiju', '--', 'gateway']
                        else:
                            command = ['cargo', 'run', '-p', 'vw-desktop', '--bin', 'shi-ju']
                        running = subprocess.Popen(command, cwd=ROOT, env=env,
                                                   start_new_session=True)
                        ready = False
                        ready_since = None
                        print(f'[watch] 已启动 {" ".join(command)} (PID {running.pid})', flush=True)
                    if running is not None and running.poll() is not None:
                        print(f'[watch] 程序退出 ({running.returncode})；下次修改后重新编译启动。', flush=True)
                        stop(running)
                        running = None
            finally:
                # Finish cleanup even if Ctrl-C is pressed again.
                signal.signal(signal.SIGINT, signal.SIG_IGN)
                signal.signal(signal.SIGTERM, signal.SIG_IGN)
                stop(watcher)
                stop(running)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('mode', choices=['gateway', 'desktop'])
    parser.add_argument('--restart', action='store_true', help='请求运行中的 watcher 切换新版')
    args = parser.parse_args()
    if args.restart:
        try:
            request_restart(args.mode)
            return 0
        except (OSError, RuntimeError) as exc:
            print(f'[watch] {exc}', file=sys.stderr)
            return 1
    if shutil.which('watchexec') is None:
        parser.error('需要 Watchexec：macOS 执行 brew install watchexec；其他平台见 https://watchexec.github.io/')
    if os.name != 'posix':
        parser.error('目前仅支持 macOS/Linux')
    def interrupt(_signum, _frame):
        raise KeyboardInterrupt

    signal.signal(signal.SIGINT, interrupt)
    signal.signal(signal.SIGTERM, interrupt)
    try:
        watch(args.mode)
    except KeyboardInterrupt:
        return 130
    except (OSError, RuntimeError, ValueError, KeyError) as exc:
        print(f'[watch] {exc}', file=sys.stderr)
        return 1
    return 0


if __name__ == '__main__':
    sys.exit(main())
