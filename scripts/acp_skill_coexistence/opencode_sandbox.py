"""Compare native stdio with the existing approved bridge in a private Seatbelt control."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import platform
import runpy
import secrets
import shutil
import socket
import subprocess
import sys
import tempfile
from urllib.parse import urlsplit

from verify_acp_approval import environment
from verify_acp_integration import Process, require
from verify_acp_skill_coexistence import make_skill, model_server
from verify_acp_model_source import ROOT

from . import opencode
from .cortex_code import mcp_server

RG_SHA256 = "c9a2697f2b817d0a80e2b35f9c5ca1f58fa2c903418120f42fed60dcd9b0988e"


def prepare_launch(executable, rg, workspace, home, port, internal_port):
    quote = lambda path: json.dumps(str(path))
    policy = f"""(version 1)
(allow default)
(deny file-read-data)
(allow file-read-data (literal "/") (subpath "/System") (subpath "/usr/lib") (subpath "/usr/share")
  (literal "/dev/null") (literal "/dev/urandom") (literal "/private/etc/hosts") (literal "/private/etc/localtime"))
(deny file-read-data (subpath "/System/Volumes/Data"))
(allow file-read-data (subpath {quote(workspace)}) (subpath {quote(home)})
  (literal {quote(executable)}) (literal {quote(rg)}))
(deny file-read-data (subpath {quote(workspace / ".mcp-probe")}))
(deny file-write*)
(allow file-write* (subpath {quote(home)}) (literal "/dev/null"))
(deny process-exec)
(allow process-exec (literal {quote(executable)}) (literal {quote(rg)}))
(deny network*)
(allow network-bind network-inbound (local ip "localhost:{internal_port}"))
(allow network-outbound (remote ip "localhost:{port}") (remote ip "localhost:{internal_port}"))
"""
    return ["/usr/bin/sandbox-exec", "-p", policy, str(executable), "acp", "--pure",
            "--hostname", "127.0.0.1", "--port", str(internal_port)]


def reserve_port():
    with socket.socket() as reservation:
        reservation.bind(("127.0.0.1", 0))
        return reservation.getsockname()[1]


def run_child(mode, executable, rg):
    """Spawn only on behalf of the private diagnostic parent."""
    import asyncio

    workspace = Path.cwd()
    home = Path(os.environ["HOME"])
    config = json.loads(os.environ["OPENCODE_CONFIG_CONTENT"])
    options = config["provider"]["session-provider"]["options"]
    endpoint = urlsplit(options["baseURL"])
    require(endpoint.scheme == "http" and endpoint.hostname == "127.0.0.1" and endpoint.port,
            "expected private model endpoint")
    internal_port = reserve_port()
    env = {"HOME": str(home), "TMPDIR": str(home), "PATH": f"{rg.parent}:/usr/bin:/bin",
           "OPENCODE_TEST_HOME": str(home), "OPENCODE_DISABLE_PROJECT_CONFIG": "1",
           "OPENCODE_DISABLE_MODELS_FETCH": "1", "OPENCODE_DISABLE_AUTOUPDATE": "1",
           "OPENCODE_SERVER_PASSWORD": secrets.token_hex(32)}
    for name in ("CONFIG", "CACHE", "DATA", "STATE"):
        env[f"XDG_{name}_HOME"] = str(home / name.lower())

    def configure(port):
        options["baseURL"] = f"http://127.0.0.1:{port}/v1"
        env["OPENCODE_CONFIG_CONTENT"] = json.dumps(config)

    def command(port):
        return prepare_launch(executable, rg, workspace, home, port, internal_port)

    if mode == "native":
        configure(endpoint.port)
        args = command(endpoint.port)
        os.execve(args[0], args, env)

    bridge = runpy.run_path(str(ROOT / "patches/acp-model-source/omp/mcp_bridge.py"))
    stdio = runpy.run_path(str(ROOT / "patches/acp-model-source/cortex-code/mcp_bridge.py"))
    return asyncio.run(bridge["run"](command, env, workspace, endpoint, options["apiKey"],
                                     configure, stdio["Stdio"], stdio["allowed_tool"]))


def turn(executable, rg, recipe, mode, phase):
    mounted = phase in ("mounted", "rejected")
    rejected = phase == "rejected"
    with tempfile.TemporaryDirectory(prefix="opencode-seatbelt-") as scratch:
        root = Path(scratch).resolve()
        env = environment(root)
        env["PYTHONPATH"] = str(ROOT / "scripts")
        home = Path(env["HOME"])
        workspace = root / "workspace"
        skills = {
            "native": make_skill(workspace / opencode.NATIVE_DIR, "native"),
            "selected": make_skill(workspace / ".shiju/skills", "selected"),
            "unselected": make_skill(workspace / ".shiju/skills", "unselected"),
        }
        view = home / "shiju-session-skills-fixture" / "skills"
        if mounted:
            view.mkdir(parents=True)
            selected = skills["selected"]
            (view / selected["name"]).symlink_to(selected["path"])
        attachment = root / "attachment"
        value = secrets.token_hex(24)
        attachment.write_text(value)
        audit = root / "audit"
        key = secrets.token_hex(24)
        probe = opencode.Probe({"skills": skills, "mcp": value}, mounted, rejected)
        with mcp_server(workspace, attachment, audit) as mcp, model_server(opencode.Handler, key) as model:
            model.probe = probe
            config = recipe.replace("{{model}}", opencode.MODEL).replace("{{key}}", key)
            config = json.loads(config.replace("{{endpoint}}", f"http://127.0.0.1:{model.server_port}/v1"))
            if mounted:
                config["skills"] = {"paths": [str(view)]}
                if mode == "native":
                    config["permission"] = {"coexistence_*": "ask"}
            env["OPENCODE_CONFIG_CONTENT"] = json.dumps(config)
            child = Process([sys.executable, "-m", "acp_skill_coexistence.opencode_sandbox",
                             "--child", mode, str(executable), str(rg)], workspace, env, root / "acp.log")
            try:
                child.call("initialize", {"protocolVersion": 1, "clientCapabilities": {}})
                created, _ = child.call("session/new", {"cwd": str(workspace),
                                                          "mcpServers": [mcp] if mounted else []})
                decisions = []

                def reply(frame):
                    if frame.get("method") != "session/request_permission":
                        return
                    call = frame["params"]["toolCall"]
                    require(call.get("title") == "mcp__coexistence__read_probe" and call.get("rawInput") == {},
                            "unexpected bridge approval request")
                    decision = "reject_once" if rejected else "allow_once"
                    options = frame["params"]["options"]
                    require(any(row["optionId"] == decision and row["kind"] == decision for row in options),
                            "missing bridge approval option")
                    decisions.append(decision)
                    child.child.stdin.write(json.dumps({"jsonrpc": "2.0", "id": frame["id"], "result": {
                        "outcome": {"outcome": "selected", "optionId": decision}}}) + "\n")
                    child.child.stdin.flush()

                try:
                    prompt, _ = child.call("session/prompt", {"sessionId": created["sessionId"],
                        "prompt": [{"type": "text", "text": "Check the available read-only resources."}]}, reply)
                except AssertionError:
                    require(mode == "native" and probe.failure == "unexpected OpenCode stdio MCP catalog"
                            and not audit.exists() and not decisions,
                            "unexpected restricted OpenCode prompt failure")
                    return {"phase": phase, "session": created["sessionId"], "failure": probe.failure,
                            "model_requests": probe.requests, "mcp_calls": 0}
                require(not probe.failure and prompt.get("stopReason") == "end_turn", "incomplete sandbox turn")
                expected_steps = {"native", "native_read"}
                if mounted:
                    expected_steps.update(("selected", "selected_read", "mcp"))
                require(probe.completed.keys() == expected_steps, "missing correlated sandbox calls")
                expected_decisions = [("reject_once" if rejected else "allow_once")] if mounted else []
                lines = audit.read_text().splitlines() if audit.exists() else []
                require(decisions == expected_decisions and lines == ["stdio"] * int(mounted and not rejected),
                        "unexpected sandbox approval or downstream calls")
                return {"phase": phase, "session": created["sessionId"], "stop": prompt["stopReason"],
                        "model_requests": probe.requests, "calls": probe.completed,
                        "decisions": decisions, "mcp_calls": len(lines)}
            finally:
                child.close()


def main():
    if sys.argv[1:2] == ["--child"]:
        mode, executable, rg = sys.argv[2:]
        return run_child(mode, Path(executable), Path(rg))
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--upstream", required=True, type=Path)
    parser.add_argument("--rg", required=True, type=Path)
    parser.add_argument("--output", required=True, type=Path)
    args = parser.parse_args()
    require((platform.system(), platform.machine()) == ("Darwin", "arm64"), "requires Darwin arm64")
    contract = json.loads((ROOT / "patches/acp-model-source/opencode/skill-patch.json").read_text())
    recipe = json.loads((ROOT / "patches/acp-model-source/opencode/recipe.json").read_text())
    recipe = recipe["session-model"]["files"]["opencode.json"]
    with tempfile.TemporaryDirectory(prefix="opencode-reviewed-") as scratch:
        executable = Path(shutil.copy2(args.upstream, Path(scratch).resolve() / "opencode"))
        rg = Path(shutil.copy2(args.rg, Path(scratch).resolve() / "rg"))
        for binary, digest in ((executable, contract["original_sha256"]), (rg, RG_SHA256)):
            require(hashlib.sha256(binary.read_bytes()).hexdigest() == digest, "sandbox executable digest mismatch")
            require(subprocess.run(["codesign", "--verify", "--strict", str(binary)],
                                   capture_output=True, timeout=15).returncode == 0,
                    "sandbox executable signature verification failed")
        native = turn(executable, rg, recipe, "native", "mounted")
        require("failure" in native, "expected the sandbox to withhold native stdio")
        turns = [turn(executable, rg, recipe, "bridge", phase)
                 for phase in ("baseline", "mounted", "removed", "rejected")]
        require(len({row["session"] for row in turns}) == 4, "sandbox sessions reused")
    report = {"version": contract["version"], "entry_sha256": contract["original_sha256"],
              "rg_sha256": RG_SHA256, "native_stdio": native,
              "projected_transport": "streamable-http to stdio", "turns": turns, "passed": True}
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(report, indent=2) + "\n")
    return 0


if __name__ == "__main__":
    sys.exit(main())
