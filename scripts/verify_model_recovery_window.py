#!/usr/bin/env python3
"""Build and run the explicit macOS recovery window test in an owned Cargo slot."""
import argparse
import json
import os
from pathlib import Path
import signal
import subprocess
import sys
import tempfile
import time

ROOT = Path(__file__).resolve().parents[1]


def test_executable(messages, name, kind):
    """Never select a regular application binary from mixed Cargo output."""
    matches = {
        Path(item["executable"])
        for item in messages
        if item.get("reason") == "compiler-artifact"
        and item.get("executable")
        and item["target"]["name"] == name
        and item["target"]["kind"] == [kind]
        and item.get("profile", {}).get("test") is True
    }
    if len(matches) != 1:
        raise RuntimeError(f"expected exactly one test artifact for {name}, found {len(matches)}")
    return matches.pop()


def run_owned(command, environment, log, *, timeout=600):
    with log.open("w") as output:
        child = subprocess.Popen(
            command, cwd=ROOT, env=environment, stdout=output,
            stderr=subprocess.STDOUT, start_new_session=True,
        )
        try:
            return child.wait(timeout=timeout)
        finally:
            # Also reap descendants if a native panic aborts before Rust Drop runs.
            try:
                os.killpg(child.pid, signal.SIGTERM)
            except ProcessLookupError:
                pass
            try:
                child.wait(timeout=5)
            except subprocess.TimeoutExpired:
                os.killpg(child.pid, signal.SIGKILL)
                child.wait()
            deadline = time.monotonic() + 5
            while True:
                try:
                    os.killpg(child.pid, 0)
                except ProcessLookupError:
                    break
                if time.monotonic() > deadline:
                    os.killpg(child.pid, signal.SIGKILL)
                    break
                time.sleep(0.05)


def build(environment, output, package, target, name, kind):
    log = output / f"build-{package}.log"
    command = ["cargo", "test", "-p", package, *target, "--no-run", "--message-format=json"]
    print("RUN", " ".join(command), flush=True)
    code = run_owned(command, environment, log)
    if code:
        raise RuntimeError(f"build exited {code}; see {log}")
    messages = []
    for line in log.read_text().splitlines():
        try:
            messages.append(json.loads(line))
        except ValueError:
            continue
    executable = test_executable(messages, name, kind).resolve(strict=True)
    expected = Path(environment["CARGO_TARGET_DIR"]) / "debug" / "deps"
    if executable.parent != expected:
        raise RuntimeError(f"test artifact is outside owned slot: {executable}")
    return executable


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", type=Path, help="new evidence directory (default: temporary directory)")
    args = parser.parse_args()
    if sys.platform != "darwin":
        parser.error("this native window validation currently requires macOS")
    output = args.output.resolve() if args.output else Path(tempfile.mkdtemp(prefix="vw-recovery-window-"))
    if args.output:
        output.mkdir(parents=True, exist_ok=False)
    locks = ROOT / "target" / "rust-slot-locks"
    locks.mkdir(parents=True, exist_ok=True)
    lock = None
    while lock is None:
        for slot in range(1, 11):
            candidate = locks / f"slot-{slot}.lock"
            try:
                candidate.mkdir()
            except FileExistsError:
                continue
            lock = candidate
            break
        if lock is None:
            time.sleep(2)
    try:
        (lock / "owner").write_text(f"plan=model-recovery task=4 session=window-validation PID={os.getpid()}\n")
        environment = os.environ.copy()
        target = ROOT / "target" / f"rust-slot-{slot}"
        environment["CARGO_TARGET_DIR"] = str(target)
        environment["CARGO_BUILD_BUILD_DIR"] = str(target)
        evidence = {"slot": slot, "CARGO_TARGET_DIR": str(target), "output": str(output)}
        (output / "slot.json").write_text(json.dumps(evidence, indent=2))
        print(json.dumps(evidence), flush=True)
        gateway = build(environment, output, "vw-agent", ["--lib"], "vw_agent", "lib")
        window = build(environment, output, "vw-desktop", ["--test", "recovery-window"], "recovery-window", "test")
        code = run_owned([str(window), "--gateway-test-binary", str(gateway), str(output)], environment, output / "window.log", timeout=180)
        print(f"window exit={code}; evidence={output}", flush=True)
        if code:
            raise RuntimeError(f"window validation failed; see {output / 'window.log'}")
        results = json.loads((output / "window-results.json").read_text())
        if len(results) != 8:
            raise RuntimeError("expected all four scenarios in both themes")
        print("PASS: 8 native window scenarios; model HTTP counts and screenshots recorded", flush=True)
    finally:
        (lock / "owner").unlink(missing_ok=True)
        lock.rmdir()


if __name__ == "__main__":
    def interrupted(_signal, _frame):
        raise KeyboardInterrupt
    signal.signal(signal.SIGTERM, interrupted)
    main()
