"""Exercise the Hermes coexistence candidate without changing the desktop access gate."""

import argparse
from contextlib import contextmanager
from http.server import BaseHTTPRequestHandler
import json
from pathlib import Path
from queue import Empty
import secrets
import subprocess
import sys
from tempfile import TemporaryDirectory
from threading import Lock
from urllib.parse import urlsplit

from acp_skill_coexistence.hermes import Handler, MCP_TOOL, MODEL, Probe, launch
from verify_acp_approval import environment
from verify_acp_integration import Process, require, text_updates
from verify_acp_model_source import ROOT
from verify_acp_skill_coexistence import PHASES, make_skill, model_server

def patch_runtime(entry, root):
    source = ROOT / "patches/acp-model-source/hermes"
    assets = root / "patch"
    assets.mkdir()
    for name in ("patch.json", "coexistence.patch"):
        (assets / name).write_bytes((source / name).read_bytes())
    apply = assets / "apply_patch.py"
    reviewed = ROOT / "docs/shiju/acp-model-source/evidence/hermes/apply_patch.py"
    apply.write_bytes(reviewed.read_bytes())
    site = entry.parents[1] / "lib/python3.12/site-packages"
    subprocess.run([sys.executable, str(apply), "--package-dir", str(site)],
                   check=True, timeout=10, capture_output=True)
    files = json.loads((assets / "patch.json").read_text())["files"]
    return {name: row["patched_sha256"] for name, row in files.items()}


class StdioRelayHandler(BaseHTTPRequestHandler):
    def log_message(self, *_args):
        pass

    def do_POST(self):
        if (self.path != "/mcp" or
                self.headers.get("Authorization") != "Bearer " + self.server.key):
            self.send_error(403)
            return
        try:
            size = int(self.headers["Content-Length"])
            require(0 < size < 100_000, "invalid MCP request size")
            request = json.loads(self.rfile.read(size))
            require(request["method"] in ("initialize", "notifications/initialized",
                                          "tools/list", "tools/call"), "unexpected MCP method")
            if request["method"] == "tools/call":
                require(request["params"]["name"] == "read_probe"
                        and request["params"].get("arguments", {}) == {}, "unexpected MCP tool")
        except (AssertionError, KeyError, ValueError, TypeError):
            self.send_error(400)
            return
        with self.server.lock:
            peer = self.server.peer
            peer.stdin.write(json.dumps(request) + "\n")
            peer.stdin.flush()
            data = peer.stdout.readline().encode() if "id" in request else b""
        self.send_response(200 if data else 202)
        self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", str(len(data)))
        self.end_headers()
        self.wfile.write(data)


@contextmanager
def stdio_relay(attachment, audit):
    key = secrets.token_hex(24)
    script = ROOT / "scripts/acp_mount_integration.py"
    with subprocess.Popen([sys.executable, str(script), "--mcp-stdio",
                           str(attachment), str(audit)], stdin=subprocess.PIPE,
                          stdout=subprocess.PIPE, text=True, env={"PATH": "/usr/bin:/bin"}) as peer:
        try:
            with model_server(StdioRelayHandler, key) as relay:
                relay.peer = peer
                relay.lock = Lock()
                yield {"name": "coexistence", "type": "http",
                       "url": f"http://127.0.0.1:{relay.server_port}/mcp",
                       "headers": [{"name": "Authorization", "value": "Bearer " + key}]}
        finally:
            peer.stdin.close()
            try:
                peer.wait(timeout=5)
            except subprocess.TimeoutExpired:
                peer.kill()
                peer.wait()


def sandbox(python, home, workspace, history, skill_roots, model_port, mcp_port):
    real_python = python.resolve(strict=True)
    paths = (python.parents[1], real_python.parents[1], home, history, *skill_roots)
    reads = " ".join(f"(subpath {json.dumps(str(path))})" for path in paths)
    return f"""(version 1)
(allow default)
(deny file-read-data)
(allow file-read-data (literal "/") (subpath "/System") (subpath "/usr/lib") (subpath "/usr/share"))
(deny file-read-data (subpath "/System/Volumes/Data"))
(allow file-read-data (literal "/dev/null") (literal "/dev/urandom")
    (literal "/private/etc/hosts") (literal "/private/etc/localtime")
    (literal {json.dumps(str(workspace))}) {reads})
(deny file-write*)
(allow file-write* (subpath {json.dumps(str(home))}) (subpath {json.dumps(str(history))}) (literal "/dev/null"))
(deny process-fork process-exec)
(allow process-exec (literal {json.dumps(str(python))}) (literal {json.dumps(str(real_python))}))
(deny network*)
(allow network-outbound (remote ip "localhost:{model_port}") (remote ip "localhost:{mcp_port}"))
"""


# Executed under the same profile as ACP, against new synthetic files.
DENIAL_PROBE = """import pathlib, socket, subprocess, sys
for operation in (
    lambda: pathlib.Path(sys.argv[1]).read_bytes(),
    lambda: pathlib.Path(sys.argv[2]).write_text("unexpected"),
    lambda: subprocess.run(["/bin/sh", "-c", "exit"], check=True),
    lambda: socket.create_connection(("127.0.0.1", 1), timeout=1),
    lambda: pathlib.Path(sys.argv[3]).write_text("unexpected"),
    lambda: pathlib.Path(sys.argv[4]).read_bytes(),
):
    try:
        operation()
    except PermissionError:
        continue
    raise AssertionError("sandbox allowed forbidden operation")
"""


def run_phase(phase, root, entry, model, mcp, value, audit, saved_probe, foreign):
    home, workspace, view = (root / name for name in ("home", "workspace", "view"))
    for folder in (home, workspace, view):
        folder.mkdir(parents=True, mode=0o700)
    mounted = phase in ("mounted", "rejected")
    rejected = phase == "rejected"
    native = workspace / ".hermes/skills"
    fixtures = {"skills": {
        "native": make_skill(native, "native"),
        "selected": make_skill(workspace / ".shiju-harness/skills", "selected"),
        "unselected": make_skill(workspace / ".shiju-harness/skills", "unselected"),
    }, "mcp": value}
    (home / "skills").symlink_to(native, target_is_directory=True)
    history = root / secrets.token_hex(32)
    history.mkdir(mode=0o700)
    skill = fixtures["skills"]["selected"]
    skill_roots = [native]
    if mounted:
        skill_roots.extend((view, skill["path"]))
        (view / skill["name"]).symlink_to(skill["path"], target_is_directory=True)
    env = {"PATH": "/usr/bin:/bin", "HOME": str(home), "HERMES_HOME": str(home),
           "TMPDIR": str(home), "NO_PROXY": "*", "no_proxy": "*",
           "VW_ACP_LAUNCH_REFERENCE": "vw-session-model:" + history.name,
           "SHIJU_HERMES_HISTORY": str(history),
           "SHIJU_HERMES_SKILL_ROOT": str(view) if mounted else ""}
    model.probe = probe = Probe(fixtures, mounted, rejected, observe_stdio=False)
    model_port = model.server_port
    route = {"provider": "custom", "base_url": f"http://127.0.0.1:{model_port}/v1",
             "api_key": model.key, "api_mode": "chat_completions",
             "default": MODEL, "context_length": 100000}
    (home / "config.yaml").write_text(json.dumps({"model": route}))
    (home / ".env").touch()
    (home / "workspace-link").symlink_to(workspace, target_is_directory=True)
    python = entry.with_name("python")
    policy = root / "hermes.sb"
    policy.write_text(sandbox(python, home, workspace, history, skill_roots, model_port,
                              urlsplit(mcp["url"]).port))
    command = ["/usr/bin/sandbox-exec", "-f", str(policy), str(python), "-IB"]
    workspace_decoy = workspace / "unrelated.txt"
    workspace_decoy.write_text(secrets.token_hex(24))
    subprocess.run(command + ["-c", DENIAL_PROBE, str(foreign), str(workspace / "denied-write"),
                              str(home / "workspace-link/denied-link-write"), str(workspace_decoy)],
                   env=env, cwd=workspace, check=True, timeout=10)
    before = audit.read_text().splitlines() if audit.exists() else []
    command += [str(entry), "acp", "--session-model-binding"]
    client = Process(command, workspace, env, root / "hermes.log")
    decisions = []
    prompt = "Check the available read-only fixture resources."

    def respond(frame):
        if frame.get("method") != "session/request_permission":
            return
        call = frame["params"]["toolCall"]
        require(frame["params"]["sessionId"] == sid and call["title"] == MCP_TOOL
                and call.get("rawInput") == {}, "unexpected prototype permission")
        option = "deny" if rejected else "allow"
        decisions.append(option)
        answer = {"jsonrpc": "2.0", "id": frame["id"], "result": {
            "outcome": {"outcome": "selected", "optionId": option}}}
        client.child.stdin.write(json.dumps(answer) + "\n")
        client.child.stdin.flush()

    try:
        client.call("initialize", {"protocolVersion": 1, "clientCapabilities": {}})
        created, _ = client.call("session/new", {"cwd": str(workspace),
                                                 "mcpServers": [mcp] if mounted else []})
        sid = created["sessionId"]
        result, updates = client.call("session/prompt", {"sessionId": sid, "prompt": [
            {"type": "text", "text": prompt}]},
            on_frame=respond)
        require(probe.failure is None, probe.failure)
        require(result["stopReason"] == "end_turn"
                and "COEXISTENCE_DONE" in "".join(text_updates(updates)), "prototype prompt did not complete")
        after = audit.read_text().splitlines() if audit.exists() else []
        require(after == before + (["stdio"] if mounted and not rejected else []), "MCP audit mismatch")
        require(decisions == (["deny" if rejected else "allow"] if mounted else []),
                "MCP approval mismatch")
        saved_probe.append({
            "phase": phase, "session": sid, "stop_reason": result["stopReason"],
            "model_requests": probe.requests, "calls": probe.completed,
            "mcp_decisions": decisions, "mcp_calls": len(after) - len(before),
            "sandbox_denied": ["sibling_read", "workspace_write", "shell_spawn", "unlisted_port", "linked_workspace_write",
                               "unselected_workspace_read"],
        })
    finally:
        client.close()

    if phase == "baseline":
        require((history / "state.db").is_file(), "Hermes did not write its bound history")
        restored = Process(command, workspace, env, root / "restore.log")
        try:
            restored.call("initialize", {"protocolVersion": 1, "clientCapabilities": {}})
            _, frames = restored.call("session/load", {"sessionId": sid, "cwd": str(workspace),
                                                       "mcpServers": []})
            require(prompt in json.dumps(frames), "Hermes did not replay its bound history")
            saved_probe[-1]["history_restored"] = True
        finally:
            restored.close()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    report = {"passed": False, "scope": "direct-acp-prototype", "turns": []}
    try:
        with TemporaryDirectory(prefix="shiju-hermes-sandbox-") as directory:
            root = Path(directory).resolve()
            spec, report["artifact"] = launch(environment(root), prototype=True)
            entry = Path(spec["command"])
            report["candidate_files"] = patch_runtime(entry, root)
            attachment, audit, foreign = (root / name for name in ("mcp.txt", "audit.txt", "foreign.txt"))
            value = secrets.token_hex(24)
            attachment.write_text(value)
            foreign.write_text(secrets.token_hex(24))
            with model_server(Handler, secrets.token_hex(24)) as model, stdio_relay(attachment, audit) as mcp:
                for phase in PHASES:
                    report["stage"] = phase
                    run_phase(phase, root / phase, entry, model, mcp, value, audit, report["turns"], foreign)
            require(len({row["session"] for row in report["turns"]}) == len(PHASES), "session ID reused")
        report.pop("stage", None)
        report["passed"] = True
    except AssertionError as error:
        report["failure"] = str(error)
    except (Empty, OSError, ValueError, subprocess.SubprocessError) as error:
        report["failure"] = type(error).__name__
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(report, indent=2) + "\n")
    if not report["passed"]:
        print(report["failure"], file=sys.stderr)
    return int(not report["passed"])


if __name__ == "__main__":
    raise SystemExit(main())
