#!/usr/bin/env python3
"""Run a fixed production ACP acceptance entry against an owned loopback fixture."""
import argparse
from contextlib import ExitStack, contextmanager
import hashlib
import socket
import time
import json
import os
import platform
from pathlib import Path
import re
import shutil
import signal
import subprocess
import tempfile

from acp_model_source_fixture import ModelFixture, successful
from acp_model_source_pi import PiIdentityError, pi_identity

ROOT = Path(__file__).resolve().parents[1]
TESTS = {"session-model": "acp_model_source_real_cli_session_model",
         "cli-local": "acp_model_source_real_cli_cli_local"}


def configure_opencode_effort_fixture(fixture):
    fixture.models = ("gpt-5.1",)
    fixture.chat_effort_probe = "opencode"


def opencode_effort_successful(evidence, flow):
    requests = evidence["requests"]
    return (all(flow[key] for key in EFFORT_CHECKPOINTS)
            and evidence["errors"] == 0
            and all(row["authenticated"] and row["route"] == "/v1/chat/completions"
                    and row["model"] == "gpt-5.1" for row in requests)
            and all([row["reasoning_effort"] for row in requests if row["turn"] == turn] == [effort]
                    for turn, effort in (("DEFAULT", None), ("A", "low"), ("B", "high"))))


def configure_claude_effort_fixture(fixture):
    fixture.models = ("claude-opus-4-6",)
    fixture.messages_effort_probe = True


def claude_effort_successful(evidence, flow):
    requests = evidence["requests"]
    return (all(flow[key] for key in EFFORT_CHECKPOINTS)
            and evidence["errors"] == 0
            and all(row["authenticated"] and row["route"] == "/v1/messages"
                    and row["model"] == "claude-opus-4-6" for row in requests)
            and all([(row["effort"], row["thinking"]) for row in requests if row["turn"] == turn]
                    == [(effort, "adaptive")]
                    for turn, effort in (("DEFAULT", "high"), ("A", "low"), ("B", "high"))))


EFFORT_CHECKPOINTS = ("isolated", "default_turn", "managed_model", "advertised",
                      "low_turn", "high_turn", "invalid_rejected", "same_session")
EFFORT_CHECKS = {("opencode", "session-model"): {
    "test": "acp_model_source_opencode_effort_real_cli",
    "configure_fixture": configure_opencode_effort_fixture,
    "identity": lambda: {"engine": "1.19.5", "openai_compatible_sdk": "2.0.41"},
    "checkpoints": EFFORT_CHECKPOINTS,
    "successful": opencode_effort_successful,
}, ("claude", "session-model"): {
    "test": "acp_model_source_claude_effort_real_cli",
    "configure_fixture": configure_claude_effort_fixture,
    "checkpoints": EFFORT_CHECKPOINTS,
    "successful": claude_effort_successful,
}}


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


def claude_sdk_identity(executable, entry, env):
    """Read installed adapter identity; execute its actual bundled engine version probe."""
    path = Path(executable).resolve()
    package = next((parent for parent in path.parents if (parent / "package.json").is_file()), None)
    if package is None:
        raise Unavailable("Claude adapter package identity is unavailable")
    metadata = json.loads((package / "package.json").read_text())
    distribution, underlying = entry["distribution"], entry["underlying"]
    effort_runtime = entry.get("effort_runtime")
    if effort_runtime and metadata.get("version") == effort_runtime["distribution"]["release"]:
        distribution, underlying = effort_runtime["distribution"], effort_runtime["underlying"]
    if (metadata.get("name") != "@agentclientprotocol/claude-agent-acp"
            or metadata.get("version") != distribution["release"]
            or path != package / "dist/index.js"):
        raise VersionMismatch("installed Claude adapter identity differs from manifest")
    node = shutil.which("node", path=env.get("PATH"))
    if not node:
        raise Unavailable("Claude SDK requires installed Node")
    # Node resolves the dependency from the adapter, including legitimate hoisting.
    probe = "const {createRequire}=require('node:module'); console.log(createRequire(process.argv[1]).resolve('@anthropic-ai/claude-agent-sdk'));"
    result = subprocess.run([node, "-e", probe, str(path)], env=env, capture_output=True, timeout=20)
    if result.returncode:
        raise Unavailable("Claude adapter SDK resolution failed")
    sdk = Path(result.stdout.decode().strip()).resolve().parent
    metadata = json.loads((sdk / "package.json").read_text())
    if (metadata.get("name") != underlying.get("package")
            or metadata.get("version") != underlying.get("declared_version")
            or metadata.get("claudeCodeVersion") != underlying.get("embedded_cli_release")):
        raise VersionMismatch("installed Claude SDK identity differs from manifest")
    engine_command = [node, str(sdk / "cli.js")]
    if "native_engine" in underlying:
        native = underlying["native_engine"]
        if (platform.system(), platform.machine()) != ("Darwin", "arm64"):
            raise Unavailable("Claude native engine identity is unverified on this platform")
        probe = "const {createRequire}=require('node:module'); console.log(createRequire(process.argv[1]).resolve(process.argv[2]));"
        result = subprocess.run([node, "-e", probe, str(sdk / "sdk.mjs"), native["package"] + "/claude"],
                                env=env, capture_output=True, timeout=20)
        if result.returncode:
            raise Unavailable("Claude native engine resolution failed")
        engine = Path(result.stdout.decode().strip()).resolve()
        metadata_native = json.loads((engine.parent / "package.json").read_text())
        if (metadata_native.get("name"), metadata_native.get("version")) != (native["package"], native["release"]):
            raise VersionMismatch("installed Claude native engine package differs from manifest")
        digest = hashlib.sha256()
        with engine.open("rb") as stream:
            for block in iter(lambda: stream.read(1024 * 1024), b""):
                digest.update(block)
        if digest.hexdigest() != native["sha256"]:
            raise VersionMismatch("installed Claude native engine digest differs from manifest")
        engine_command = [str(engine)]
    result = subprocess.run([*engine_command, "--version"], env=env, capture_output=True, timeout=20)
    expected = re.escape(underlying["embedded_cli_release"])
    if result.returncode or not re.search(r"(?<![\w.])" + expected + r"(?![\w.])", result.stdout.decode(errors="replace")):
        raise VersionMismatch("installed Claude engine differs from manifest")
    return {"adapter": distribution["release"], "sdk": metadata["version"],
            "engine": underlying["embedded_cli_release"]}


def copilot_native_executable(executable, release, env):
    """Resolve a fixed native or reviewed managed entry without changing its installation."""
    if (platform.system(), platform.machine()) != ("Darwin", "arm64"):
        raise Unavailable("Copilot native package identity is unverified on this platform")
    path = Path(executable).resolve()
    native_name = "@github/copilot-darwin-arm64"
    metadata_path = path.parent / "package.json"
    try:
        metadata = json.loads(metadata_path.read_text())
    except (OSError, ValueError) as error:
        raise Unavailable("Copilot installed package identity is unavailable") from error
    if metadata.get("version") != release:
        raise VersionMismatch("Copilot installed package version differs from manifest")
    if path.name == "index.js" and path.parent.name == "embedded":
        # Managed installation: the entry and all reviewed executable inputs are
        # checked against the repository contract, not its self-reported version.
        contract = json.loads((ROOT / "crates/vw-acp/src/managed_runtime/contracts.json").read_text())["copilot"]
        package = path.parent.parent
        metadata = json.loads((package / "package.json").read_text())
        if (metadata.get("name"), metadata.get("version")) != (contract["package"], release) or contract["version"] != release:
            raise VersionMismatch("Copilot managed package identity differs")
        import hashlib
        for name, expected in contract["files"].items():
            if hashlib.sha256((package / name).read_bytes()).hexdigest() != expected:
                raise VersionMismatch("Copilot managed artifact checksum differs")
        return str(Path(executable).absolute())
    if metadata.get("name") == "@github/copilot" and path.name == "npm-loader.js":
        node = shutil.which("node", path=env.get("PATH"))
        if not node:
            raise Unavailable("Copilot npm package resolution requires installed Node")
        probe = "const {createRequire}=require('node:module'); console.log(createRequire(process.argv[1]).resolve('@github/copilot-darwin-arm64'));"
        result = subprocess.run([node, "-e", probe, str(path)], env=env, capture_output=True, timeout=20)
        if result.returncode:
            raise Unavailable("Copilot installed native package resolution failed")
        path = Path(result.stdout.decode().strip()).resolve()
        try:
            metadata = json.loads((path.parent / "package.json").read_text())
        except (OSError, ValueError) as error:
            raise Unavailable("Copilot native package metadata is unavailable") from error
    if (metadata.get("name"), metadata.get("version"), path.name) != (native_name, release, "copilot") or not path.is_file():
        raise VersionMismatch("Copilot native package identity differs from manifest")
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
    if entry.get("key") == "claude" and entry.get("distribution", {}).get("package") == "@agentclientprotocol/claude-agent-acp":
        if entry.get("acp_launch") != ["claude-agent-acp"]:
            raise Unavailable("Claude manifest must identify the pinned adapter launch")
        return {"command": str(Path(executable).absolute()), "args": [], "env": {}}
    if entry.get("key") == "pi":
        try:
            pi_identity(executable, entry, env)
        except PiIdentityError as error:
            raise VersionMismatch(str(error)) from error
        return {"command": str(Path(executable).absolute()), "args": [], "env": {}}
    result = subprocess.run([executable, "--version"], env=env, capture_output=True, timeout=20)
    version = (result.stdout + result.stderr).decode(errors="replace")
    if entry.get("key") == "copilot":
        # The fixed CLI prints a sentence-ending period. Keep this product-specific
        # and exact: a fourth component or prerelease must never match the pin.
        matches_version = version.splitlines()[:1] == ["GitHub Copilot CLI " + release + "."]
    elif entry.get("key") == "goose":
        # The reviewed build prints the release without the source tag's v prefix.
        matches_version = version.strip().removeprefix("v") == release.removeprefix("v")
    else:
        version_prefix = r"v?" if entry.get("key") == "hermes" else ""
        matches_version = re.search(r"(?<![\w.])" + version_prefix + re.escape(release) + r"(?![\w.])", version)
    if result.returncode or not matches_version:
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
    if entry.get("key") == "copilot":
        if underlying != {"kind": "same_distribution", "release": release}:
            raise VersionMismatch("Copilot native distribution differs from manifest")
        executable = copilot_native_executable(executable, release, env)
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
    values = {"origin": fixture.url, "endpoint": fixture.url + "/v1", "key": fixture.key,
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
        if any(url != fixture.url and not url.startswith(fixture.url + "/") for url in urls):
            raise Unavailable("native configuration contains a non-fixture URL")
        target = directory / path
        target.parent.mkdir(parents=True, exist_ok=True, mode=0o700)
        with target.open("x") as file:
            os.chmod(target, 0o600)
            file.write(content)
    environment = {env_key: str(directory)}
    if entry.get("key") == "kiro":
        # Fixed reviewed native recipe; arbitrary environment injection is not accepted.
        environment.update(KIRO_KAS_ENDPOINT=fixture.url, KIRO_KAS_CONTROL_PLANE_ENDPOINT=fixture.url,
                           KIRO_KAS_REGION="us-east-1", KIRO_API_KEY=values["key"],
                           KIRO_RUNTIME_DIR=str(directory / "runtime"), KIRO_NO_AUTO_UPDATE="1")
        fixture.kiro_controls = True
    if entry.get("key") == "pi":
        state = directory / "acp"
        state.mkdir(mode=0o700)
        environment["PI_ACP_STATE_DIR"] = str(state)
    return environment


@contextmanager
def openclaw_native_gateway(config, env, native):
    """Own the genuine foreground Gateway used only by cli-local acceptance."""
    path = native / "openclaw.json"
    settings = json.loads(path.read_text())
    with socket.socket() as listener:
        listener.bind(("127.0.0.1", 0))
        port = listener.getsockname()[1]
    settings["gateway"]["port"] = port
    path.write_text(json.dumps(settings))
    process = subprocess.Popen([config["command"], "gateway", "run", "--port", str(port),
                                "--bind", "loopback"], env=env, stdin=subprocess.DEVNULL,
                               stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL,
                               start_new_session=True)
    try:
        deadline = time.monotonic() + 30
        while True:
            if process.poll() is not None:
                raise Unavailable("native OpenClaw Gateway exited before readiness")
            try:
                with socket.create_connection(("127.0.0.1", port), timeout=0.2):
                    break
            except OSError:
                if time.monotonic() >= deadline:
                    raise Unavailable("native OpenClaw Gateway readiness timed out")
                time.sleep(0.05)
        yield
    finally:
        try:
            os.killpg(process.pid, signal.SIGKILL)
        except ProcessLookupError:
            pass
        process.wait()


def execute_cargo(command, *, env, timeout=1800):
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
              "status": "unavailable", "passed": False, "platform": platform.system() + "-" + platform.machine()}
    try:
        slot = cargo_slot(os.environ)
        effort_check = None
        expected_claude_effort = None
        if getattr(args, "effort_check", False):
            effort_check = EFFORT_CHECKS.get((args.agent, args.mode))
            if effort_check is None:
                raise Unavailable("no effort check for the selected agent and model source")
        manifest = json.loads(args.manifest.read_text())
        entries = [e for e in manifest["entries"] if e["key"] == args.agent]
        if len(entries) != 1:
            raise Unavailable("manifest must identify exactly one agent")
        entry = entries[0]
        opencode_effort = args.agent == "opencode" and effort_check is not None
        claude_effort = args.agent == "claude" and effort_check is not None
        if opencode_effort and entry["distribution"]["release"] != "1.19.5":
            raise VersionMismatch("OpenCode effort check requires the reviewed 1.19.5 release")
        upstream_protocol = entry.get("offline_acceptance", {}).get(args.mode, {}).get("upstream_protocol", "chat")
        if upstream_protocol not in ("chat", "messages") or (claude_effort and upstream_protocol != "messages"):
            raise Unavailable("unsupported offline fixture protocol")
        with tempfile.TemporaryDirectory(prefix="acp-model-acceptance-") as temp, ModelFixture() as fixture, ExitStack() as owned:
            if effort_check:
                effort_check["configure_fixture"](fixture)
            root = Path(temp).resolve() if opencode_effort else Path(temp)
            os.chmod(root, 0o700)
            native = root / "native"
            native.mkdir(mode=0o700)
            # Drop account, proxy, model and config variables before even probing CLI versions.
            env = {k: os.environ[k] for k in ("PATH", "HOME", "USERPROFILE", "HOMEDRIVE", "HOMEPATH", "SystemRoot", "TMPDIR", "RUST_MIN_STACK") if k in os.environ}
            env.setdefault("RUST_MIN_STACK", "33554432")
            env.update(CARGO_TARGET_DIR=slot, CARGO_BUILD_BUILD_DIR=slot,
                       VIBEWINDOW_CONFIG_DIR=str(root / "config"))
            native_env = isolated_environment(entry, native, fixture, args.mode)
            if opencode_effort:
                for key in ("XDG_CONFIG_HOME", "XDG_DATA_HOME", "XDG_STATE_HOME",
                            "XDG_CACHE_HOME", "OPENCODE_TEST_HOME"):
                    path = native / key.lower()
                    path.mkdir(mode=0o700)
                    native_env[key] = str(path)
                (root / "opencode.json").write_text(json.dumps({
                    "model": "forbidden/local", "small_model": "forbidden/small",
                    "agent": {"build": {"model": "forbidden/main"}}}))
            env.update(native_env)
            if args.agent == "claude" and args.mode == "session-model":
                project_settings = root / ".claude/settings.local.json"
                project_settings.parent.mkdir(mode=0o700)
                project_settings.write_text(json.dumps({"model": "forbidden-native-model",
                    "apiKeyHelper": "printf offline-native-account-must-not-be-used", "env": {
                    "ANTHROPIC_BASE_URL": fixture.url, "ANTHROPIC_AUTH_TOKEN": "offline-native-account-must-not-be-used",
                    "ANTHROPIC_MODEL": "forbidden-native-model", "CLAUDE_CODE_USE_BEDROCK": "1"}}))
                os.chmod(project_settings, 0o600)
                credentials = native / ".credentials.json"
                credentials.write_text(json.dumps({"claudeAiOauth": {
                    "accessToken": "offline-native-oauth-must-not-be-used",
                    "refreshToken": "offline-native-refresh-must-not-be-used", "expiresAt": 4102444800000}}))
                os.chmod(credentials, 0o600)
            config = locate(entry, env)
            if opencode_effort:
                report["identity"] = effort_check["identity"]()
            if args.agent == "pi":
                fixture.pi_compaction = True
                report["identity"] = pi_identity(config["command"], entry, env)
                project_settings = root / ".pi/settings.json"
                project_settings.parent.mkdir(mode=0o700)
                settings = {"compaction": {"keepRecentTokens": 1, "reserveTokens": 128}}
                if args.mode == "session-model":
                    settings.update(defaultProvider="session-provider", defaultModel="forbidden-native-model")
                project_settings.write_text(json.dumps(settings))
                os.chmod(project_settings, 0o600)
            if args.agent == "claude":
                report["identity"] = claude_sdk_identity(config["command"], entry, env)
                contract = entry.get("effort_runtime")
                qualified = contract and report["identity"]["adapter"] == contract["distribution"]["release"]
                if claude_effort and not qualified:
                    raise VersionMismatch("Claude effort check requires the reviewed effort runtime")
                if qualified and not claude_effort:
                    expected_claude_effort = contract["fixture_effort"]
            if args.agent == "openclaw" and args.mode == "cli-local":
                owned.enter_context(openclaw_native_gateway(config, env, native))
            config["env"] = native_env if args.mode == "cli-local" else {}
            (root / "challenge.txt").write_text(fixture.challenge)
            fixture.read_path = str(root / "challenge.txt")
            if args.agent == "claude":
                (root / "challenge-second.txt").write_text(fixture.challenge)
                fixture.second_read_path = str(root / "challenge-second.txt")
                fixture.probe_auxiliary = not claude_effort
            if args.agent == "kimi":
                fixture.probe_auxiliary = True
                fixture.auxiliary_profile = "coder"
                fixture.auxiliary_model = None
            if args.agent == "kiro":
                (root / "challenge-second.txt").write_text(fixture.challenge)
                fixture.second_read_path = str(root / "challenge-second.txt")
                fixture.probe_auxiliary = True
                fixture.auxiliary_profile = "general-task-execution"
                fixture.auxiliary_model = None
            if args.agent == "openclaw":
                (root / "challenge-second.txt").write_text(fixture.challenge)
                fixture.second_read_path = str(root / "challenge-second.txt")
                fixture.probe_auxiliary = True
                fixture.auxiliary_profile = "openclaw"
                fixture.auxiliary_model = None
            runtime = root / "entry.json"
            runtime.write_text(json.dumps({"agent": args.agent, "mode": args.mode, "upstream_protocol": upstream_protocol,
                                          "config": config, "isolation_env": native_env, "root": str(root),
                                          "fixture": fixture.url, "key": fixture.key,
                                          "control": fixture.control, "challenge": fixture.challenge,
                                          "effort_check": bool(effort_check)}))
            os.chmod(runtime, 0o600)
            flow_path = root / "flow.json"
            env.update(VW_ACP_ACCEPTANCE_MANIFEST=str(runtime), VW_ACP_ACCEPTANCE_FLOW=str(flow_path))
            diagnostic_path = root / "diagnostics.json"
            if args.agent in ("claude", "copilot", "kiro", "openclaw", "pi"):
                diagnostic_path.write_text("{}")
                os.chmod(diagnostic_path, 0o600)
                env["VW_ACP_ACCEPTANCE_DIAGNOSTICS"] = str(diagnostic_path)
            test = effort_check["test"] if effort_check else TESTS[args.mode]
            command = ["cargo", "test", "-p", "vw-agent", "--lib", "--all-features", test,
                       "--", "--ignored", "--nocapture", "--test-threads=1"]
            # Logs can contain supplier content. Never persist or echo raw stdout/stderr.
            result = execute_cargo(command, env=env)
            text = result.stdout.decode(errors="replace")
            exactly_one = bool(re.search(r"test result: ok\. 1 passed; 0 failed; 0 ignored;", text))
            evidence = fixture.snapshot()
            if diagnostic_path.exists():
                diagnostics = json.loads(diagnostic_path.read_text())
                allowed = {"out_initialize", "out_new", "out_load", "out_resume", "out_prompt",
                           "in_read", "in_terminal", "in_permission", "in_update", "sdk_settings_sent",
                           "sdk_settings_exists", "initialize_returned", "new_returned", "out_error", "in_error",
                           "first_error_code", "error_session_missing", "error_process_exit", "error_auth",
                           "error_forbidden", "error_unsupported", "error_validation",
                           "out_set_model", "out_model_primary", "out_model_switched", "out_model_other"}
                report["diagnostics"] = {key: value for key, value in diagnostics.items()
                                         if key in allowed and isinstance(value, (bool, int))}
                if expected_claude_effort and diagnostics.get("claude_effort") == expected_claude_effort:
                    report["claude_effort"] = expected_claude_effort
            relay_diagnostics = diagnostic_path.with_suffix(".relay.json")
            if relay_diagnostics.exists():
                observations = json.loads(relay_diagnostics.read_text())
                report["relay_observations"] = [row for row in observations
                    if isinstance(row, dict) and row.get("route") in ("messages", "count_tokens", "models", "other")
                    and isinstance(row.get("status"), int) and 100 <= row["status"] <= 599]
            raw_flow = json.loads(flow_path.read_text()) if flow_path.exists() else {}
            if not isinstance(raw_flow, dict):
                raise ValueError("invalid flow result")
            if effort_check:
                flow = {key: raw_flow.get(key) is True for key in effort_check["checkpoints"]}
            else:
                flow = {key: raw_flow.get(key) is True for key in
                        ("new", "second_turn", "cancel", "restart", "switch", "isolated", "handshake")}
                flow["requires_auxiliary"] = args.agent in ("claude", "kimi", "kiro", "openclaw", "pi")
                if args.agent == "pi":
                    flow["auxiliary"] = raw_flow.get("auxiliary") is True
                    flow["auxiliary_switched"] = raw_flow.get("auxiliary_switched") is True
            flow["mode"] = args.mode
            predicate = effort_check["successful"] if effort_check else successful
            passed = result.returncode == 0 and exactly_one and predicate(evidence, flow)
            if expected_claude_effort:
                passed = passed and "claude_effort" in report
            report.update(status="passed" if passed else ("handshake-only" if flow.get("handshake") and not evidence["requests"] else "failed"), passed=passed,
                          cargo_exit_code=result.returncode, effective_tests=1 if re.search(r"running 1 test\b", text) else 0,
                          failure_locations=sorted(set(re.findall(
                              r"(?:crates|src)/[a-zA-Z0-9_./-]+\.rs:\d+(?::\d+)?",
                              (result.stdout + result.stderr).decode(errors="replace")))) if not passed else [],
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


def parse_args():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--agent", required=True)
    parser.add_argument("--mode", required=True, choices=tuple(TESTS))
    parser.add_argument("--manifest", required=True, type=Path)
    parser.add_argument("--output", required=True, type=Path)
    parser.add_argument("--offline-fixture", action="store_true", default=True,
                        help="owned loopback upstream (the only supported upstream)")
    parser.add_argument("--effort-check", action="store_true",
                        help="verify native reasoning effort for the selected agent and source")
    return parser.parse_args()


def main():
    return run(parse_args())


if __name__ == "__main__":
    raise SystemExit(main())
