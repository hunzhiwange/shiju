"""Exercise the Hermes coexistence candidate without changing the desktop access gate."""

import argparse
from contextlib import contextmanager
import hashlib
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

PATCH = Path(__file__).with_suffix(".patch")
# These two upstream files are outside the existing v2 patch contract.
EXTRA_V2_FILES = {
    "tools/mcp_tool.py": "a7328f3f3762ae43f6a9426646b0c28c17ec8663aa391506f48d628035ad5460",
    "acp_adapter/server.py": "e780061d47fa6c874f89676afc75ab386b50f79bb269fdef38095245e8f32145",
}
PATCHED_FILES = {
    "agent/skill_utils.py": "b388417c6339b1cb138105395429a28f50b2c6f3f72b699bdc6926e03787db75",
    "agent/prompt_builder.py": "bef52d0b47708f18532e3447bb0748491a1fcb73a8087b790adc62483ff15b4a",
    "tools/skills_tool.py": "fccaff108b9d850d84087f506d04d53aea9dc75e767796ea64348cb8b8bff275",
    "tools/mcp_tool.py": "703923450ccbb18a6736015c90423599b25d65b9038bc50a707ff8eb38c918d0",
    "acp_adapter/server.py": "402d2f5ae4041889d970e338eaa34a4c31cf4b394335ce31b79bfc55dbb9dff7",
}


def patch_runtime(entry):
    site = entry.parents[1] / "lib/python3.12/site-packages"
    for name in PATCHED_FILES:
        file = site / name
        require(file.resolve(strict=True) == file, "candidate patch target is a symlink")
    for name, digest in EXTRA_V2_FILES.items():
        require(hashlib.sha256((site / name).read_bytes()).hexdigest() == digest,
                "unexpected Hermes candidate input: " + name)
    subprocess.run(["/usr/bin/patch", "-s", "-f", "-F0", "-p1", "-i", str(PATCH.resolve())],
                   cwd=site, check=True, timeout=10)
    for name, digest in PATCHED_FILES.items():
        require(hashlib.sha256((site / name).read_bytes()).hexdigest() == digest,
                "unexpected Hermes candidate output: " + name)


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


def sandbox(python, home, workspace, view, model_port, mcp_port):
    real_python = python.resolve(strict=True)
    paths = (python.parents[1], real_python.parents[1], home, workspace, view)
    reads = " ".join(f"(subpath {json.dumps(str(path))})" for path in paths)
    return f"""(version 1)
(allow default)
(deny file-read-data)
(allow file-read-data (literal "/") (subpath "/System") (subpath "/usr/lib") (subpath "/usr/share"))
(deny file-read-data (subpath "/System/Volumes/Data"))
(allow file-read-data (literal "/dev/null") (literal "/dev/urandom")
    (literal "/private/etc/hosts") (literal "/private/etc/localtime") {reads})
(deny file-write*)
(allow file-write* (subpath {json.dumps(str(home))}) (literal "/dev/null"))
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
    fixtures = {"skills": {
        "native": make_skill(home / "skills", "native"),
        "selected": make_skill(workspace / ".shiju/skills", "selected"),
        "unselected": make_skill(workspace / ".shiju/skills", "unselected"),
    }, "mcp": value}
    skill = fixtures["skills"]["selected"]
    if mounted:
        (view / skill["name"]).symlink_to(skill["path"], target_is_directory=True)
    env = {"PATH": "/usr/bin:/bin", "HOME": str(home), "HERMES_HOME": str(home),
           "TMPDIR": str(home), "NO_PROXY": "*", "no_proxy": "*",
           "VW_ACP_LAUNCH_REFERENCE": "vw-session-model:" + secrets.token_hex(32),
           "SHIJU_HERMES_SKILL_ROOT": str(view) if mounted else ""}
    model.probe = probe = Probe(fixtures, mounted, rejected)
    model_port = model.server_port
    route = {"provider": "custom", "base_url": f"http://127.0.0.1:{model_port}/v1",
             "api_key": model.key, "api_mode": "chat_completions",
             "default": MODEL, "context_length": 100000}
    (home / "config.yaml").write_text(json.dumps({"model": route}))
    (home / ".env").touch()
    python = entry.with_name("python")
    policy = root / "hermes.sb"
    policy.write_text(sandbox(python, home, workspace, view, model_port, urlsplit(mcp["url"]).port))
    command = ["/usr/bin/sandbox-exec", "-f", str(policy), str(python), "-IB"]
    subprocess.run(command + ["-c", DENIAL_PROBE, str(foreign), str(workspace / "denied-write")],
                   env=env, cwd=workspace, check=True, timeout=10)
    before = audit.read_text().splitlines() if audit.exists() else []
    client = Process(command + [str(entry), "acp", "--session-model-binding"],
                     workspace, env, root / "hermes.log")
    decisions = []

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
            {"type": "text", "text": "Check the available read-only fixture resources."}]},
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
            "sandbox_denied": ["sibling_read", "workspace_write", "shell_spawn", "unlisted_port"],
        })
    finally:
        client.close()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    report = {"passed": False, "scope": "direct-acp-prototype", "turns": []}
    try:
        with TemporaryDirectory(prefix="shiju-hermes-sandbox-") as directory:
            root = Path(directory).resolve()
            spec, report["artifact"] = launch(environment(root))
            entry = Path(spec["command"])
            patch_runtime(entry)
            report["candidate_files"] = PATCHED_FILES
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
