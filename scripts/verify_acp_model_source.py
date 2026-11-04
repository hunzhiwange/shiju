#!/usr/bin/env python3
"""Run a fixed production ACP acceptance entry against an owned loopback fixture."""
import argparse
import json
import os
from pathlib import Path
import re
import shutil
import signal
import subprocess
import tempfile

from acp_model_source_fixture import ModelFixture, successful

ROOT = Path(__file__).resolve().parents[1]
TESTS = {"session-model": "acp_model_source_real_cli_session_model",
         "cli-local": "acp_model_source_real_cli_cli_local"}


class Unavailable(Exception):
    pass


class VersionMismatch(Unavailable):
    pass


def cargo_slot(env):
    raw = env.get("CARGO_TARGET_DIR", "")
    path = Path(raw)
    if not path.is_absolute() or path.parent != ROOT / "target" or not re.fullmatch(r"rust-slot-(?:[1-9]|10)", path.name):
        raise Unavailable("caller must hold a shared Cargo slot and set absolute CARGO_TARGET_DIR")
    lock = ROOT / "target/rust-slot-locks" / ("slot-" + path.name.removeprefix("rust-slot-") + ".lock")
    if not lock.is_dir() or not any(lock.iterdir()):
        raise Unavailable("Cargo slot has no owner record")
    records = "\n".join(record.read_text(errors="replace") for record in lock.iterdir() if record.is_file())
    match = re.search(r"\bPID[\"'=: ]+(\d+)\b", records, re.IGNORECASE)
    if not match:
        raise Unavailable("Cargo slot owner record must identify its holder PID")
    try:
        os.kill(int(match.group(1)), 0)
    except ProcessLookupError as error:
        raise Unavailable("Cargo slot holder is no longer running") from error
    return str(path)


def locate(entry, env):
    # Never use npm/npx discovery: it can install or silently change a version.
    candidates = entry.get("local_command_candidates", [])
    for candidate in candidates:
        if not isinstance(candidate, str) or candidate in ("npx", "npm", "uvx"):
            continue
        executable = shutil.which(candidate, path=env.get("PATH"))
        if executable:
            break
    else:
        raise Unavailable("real CLI is not installed")
    probe = entry.get("installed", {}).get("command", [])
    if not probe or Path(probe[0]).name != Path(candidate).name or probe[1:] != ["--version"]:
        raise Unavailable("manifest has no safe fixed version probe")
    release = entry.get("distribution", {}).get("release")
    if not isinstance(release, str) or not release:
        raise Unavailable("manifest has no pinned adapter release")
    result = subprocess.run([executable, "--version"], env=env, capture_output=True, timeout=20)
    version = (result.stdout + result.stderr).decode(errors="replace")
    if result.returncode or not re.search(r"(?<![\w.])" + re.escape(release) + r"(?![\w.])", version):
        raise VersionMismatch("installed CLI version differs from manifest")
    launch = entry.get("acp_launch", [])
    if not launch or Path(launch[0]).name != Path(candidate).name or any("latest" in x for x in launch):
        raise Unavailable("manifest launch does not identify the installed pinned CLI")
    underlying = entry.get("underlying", {})
    if underlying.get("kind") != "same_distribution":
        # Independent binaries/adapters need a separately pinned safe probe.
        command = underlying.get("command")
        expected = underlying.get("release")
        if not isinstance(command, list) or len(command) != 2 or command[1] != "--version" or not expected:
            raise Unavailable("underlying CLI version probe is unverified")
        binary = shutil.which(command[0], path=env.get("PATH"))
        if not binary:
            raise Unavailable("underlying CLI is not installed")
        result = subprocess.run([binary, "--version"], env=env, capture_output=True, timeout=20)
        if result.returncode or not re.search(r"(?<![\w.])" + re.escape(expected) + r"(?![\w.])", (result.stdout + result.stderr).decode(errors="replace")):
            raise VersionMismatch("underlying CLI version differs from manifest")
    return {"command": str(Path(executable).absolute()), "args": launch[1:], "env": {}}


def isolated_environment(entry, directory, fixture, mode):
    """Recipes must be supplied by each real driver task after upstream inspection.

    Only private directory/file templates are accepted, never arbitrary commands,
    ambient credentials, HOME substitution, or production URLs.
    """
    recipe = entry.get("offline_acceptance", {}).get(mode)
    if not isinstance(recipe, dict):
        raise Unavailable("driver has no reviewed offline native configuration recipe")
    env_key = recipe.get("config_root_env", "")
    if not re.fullmatch(r"[A-Z][A-Z0-9_]+", env_key) or env_key in ("HOME", "USERPROFILE", "PATH"):
        raise Unavailable("driver has no dedicated native configuration root")
    values = {"endpoint": fixture.url + "/v1", "key": fixture.key,
              "model": fixture.models[0], "root": str(directory)}
    if mode == "session-model":
        values.update(key="offline-native-account-must-not-be-used", model="forbidden-native-model")
    files = recipe.get("files", {})
    if not isinstance(files, dict) or not files:
        raise Unavailable("driver has no native offline configuration files")
    for name, template in files.items():
        path = Path(name)
        if path.is_absolute() or ".." in path.parts or not isinstance(template, str):
            raise Unavailable("unsafe native configuration file")
        content = template
        for key, value in values.items():
            content = content.replace("{{" + key + "}}", value)
        # This tool never accepts externally reachable upstreams in its recipes.
        urls = re.findall(r"https?://[^\s\"']+", content)
        if any(not url.startswith(fixture.url + "/") for url in urls):
            raise Unavailable("native configuration contains a non-fixture URL")
        target = directory / path
        target.parent.mkdir(parents=True, exist_ok=True, mode=0o700)
        with target.open("x") as file:
            os.chmod(target, 0o600)
            file.write(content)
    return {env_key: str(directory)}


def execute_cargo(command, *, env, timeout=300):
    """Own the process group, including agents left behind by a failed Rust test."""
    if os.name != "posix":
        raise Unavailable("offline native ACL and child-process containment are unverified on this platform")
    def interrupted(_signum, _frame):
        raise InterruptedError("offline acceptance interrupted")

    process = subprocess.Popen(command, cwd=ROOT, env=env, stdout=subprocess.PIPE,
                               stderr=subprocess.PIPE, start_new_session=True)
    previous = signal.signal(signal.SIGTERM, interrupted)
    try:
        stdout, stderr = process.communicate(timeout=timeout)
        return subprocess.CompletedProcess(command, process.returncode, stdout, stderr)
    finally:
        signal.signal(signal.SIGTERM, signal.SIG_IGN)
        # Even normal cargo exit can leave a CLI alive after a panicking test.
        try:
            os.killpg(process.pid, signal.SIGTERM)
        except ProcessLookupError:
            pass
        try:
            process.wait(timeout=3)
        except subprocess.TimeoutExpired:
            pass
        try:
            os.killpg(process.pid, signal.SIGKILL)
        except ProcessLookupError:
            pass
        process.wait()
        for stream in (process.stdout, process.stderr):
            if stream is not None:
                stream.close()
        signal.signal(signal.SIGTERM, previous)


def run(args):
    report = {"agent": args.agent, "mode": args.mode, "evidence_kind": "real-cli",
              "status": "unavailable", "passed": False}
    try:
        slot = cargo_slot(os.environ)
        manifest = json.loads(args.manifest.read_text())
        entries = [e for e in manifest["entries"] if e["key"] == args.agent]
        if len(entries) != 1:
            raise Unavailable("manifest must identify exactly one agent")
        entry = entries[0]
        with tempfile.TemporaryDirectory(prefix="acp-model-acceptance-") as temp, ModelFixture() as fixture:
            root = Path(temp)
            os.chmod(root, 0o700)
            native = root / "native"
            native.mkdir(mode=0o700)
            # Drop account, proxy, model and config variables before even probing CLI versions.
            env = {k: os.environ[k] for k in ("PATH", "HOME", "USERPROFILE", "HOMEDRIVE", "HOMEPATH", "SystemRoot", "TMPDIR", "RUST_MIN_STACK") if k in os.environ}
            env.update(CARGO_TARGET_DIR=slot, CARGO_BUILD_BUILD_DIR=slot,
                       VIBEWINDOW_CONFIG_DIR=str(root / "config"))
            native_env = isolated_environment(entry, native, fixture, args.mode)
            env.update(native_env)
            config = locate(entry, env)
            config["env"] = native_env if args.mode == "cli-local" else {}
            (root / "challenge.txt").write_text(fixture.challenge)
            runtime = root / "entry.json"
            runtime.write_text(json.dumps({"agent": args.agent, "mode": args.mode,
                                          "config": config, "isolation_env": native_env, "root": str(root),
                                          "fixture": fixture.url, "key": fixture.key,
                                          "control": fixture.control, "challenge": fixture.challenge}))
            os.chmod(runtime, 0o600)
            flow_path = root / "flow.json"
            env.update(VW_ACP_ACCEPTANCE_MANIFEST=str(runtime), VW_ACP_ACCEPTANCE_FLOW=str(flow_path))
            test = TESTS[args.mode]
            command = ["cargo", "test", "-p", "vw-agent", "--lib", "--all-features", test,
                       "--", "--ignored", "--nocapture", "--test-threads=1"]
            # Logs can contain supplier content. Never persist or echo raw stdout/stderr.
            result = execute_cargo(command, env=env)
            text = result.stdout.decode(errors="replace")
            exactly_one = bool(re.search(r"test result: ok\. 1 passed; 0 failed; 0 ignored;", text))
            evidence = fixture.snapshot()
            raw_flow = json.loads(flow_path.read_text()) if flow_path.exists() else {}
            if not isinstance(raw_flow, dict):
                raise ValueError("invalid flow result")
            flow = {key: raw_flow.get(key) is True for key in
                    ("new", "second_turn", "cancel", "restart", "switch", "isolated", "handshake")}
            flow["mode"] = args.mode
            passed = result.returncode == 0 and exactly_one and successful(evidence, flow)
            report.update(status="passed" if passed else ("handshake-only" if flow.get("handshake") and not evidence["requests"] else "failed"), passed=passed,
                          cargo_exit_code=result.returncode, effective_tests=1 if re.search(r"running 1 test\b", text) else 0,
                          requests=evidence, flow=flow, command=command, cargo_target_dir=slot)
    except VersionMismatch as error:
        report.update(status="version-mismatch", reason=str(error))
    except Unavailable as error:
        report["reason"] = str(error)
    except (OSError, ValueError, KeyError, subprocess.TimeoutExpired):
        report.update(status="failed", reason="offline acceptance execution failed; raw diagnostics withheld")
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(report, ensure_ascii=False, indent=2) + "\n")
    return 0 if report["passed"] else 1


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--agent", required=True)
    parser.add_argument("--mode", required=True, choices=tuple(TESTS))
    parser.add_argument("--manifest", required=True, type=Path)
    parser.add_argument("--output", required=True, type=Path)
    parser.add_argument("--offline-fixture", action="store_true", default=True,
                        help="owned loopback upstream (the only supported upstream)")
    return run(parser.parse_args())


if __name__ == "__main__":
    raise SystemExit(main())
