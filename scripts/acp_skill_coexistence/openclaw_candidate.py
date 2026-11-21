"""Run the pinned scanner candidate in a restricted private OpenClaw Gateway."""

import argparse
import asyncio
from contextlib import contextmanager, nullcontext
import hashlib
from http.server import ThreadingHTTPServer
import inspect
import json
import os
from pathlib import Path
import runpy
import secrets
import shutil
import signal
import socket
import subprocess
import sys
from tempfile import TemporaryDirectory
from threading import Thread
import time
from urllib.parse import urlsplit

from acp_mount_integration import rpc as fixture_rpc
from verify_acp_approval import environment
from verify_acp_integration import Process, require, text_updates
from verify_acp_model_source import ROOT
from verify_acp_skill_coexistence import make_skill, model_server

from acp_skill_coexistence.openclaw import Handler, MCP_TOOL, MODEL, Probe, VERSION


def digest(data):
    return hashlib.sha256(data).hexdigest()


def install_candidate(runtime):
    patches = ROOT / "patches/acp-model-source/openclaw"
    contract = json.loads((patches / "patch.json").read_text())
    spec = json.loads((patches / "coexistence-candidate.json").read_text())
    source = (Path.home() / ".local/share/shiju/acp-runtime"
              / f"openclaw-{VERSION}-model-source-v1")
    subprocess.run(["/bin/cp", "-cR", str(source), str(runtime)], check=True, timeout=60)
    package = runtime / "node_modules/openclaw"
    manifest = ROOT / "crates/libs/acp-agent-registry/src/managed_runtime/contracts.json"
    pin = json.loads(manifest.read_text())["openclaw"]
    metadata = json.loads((package / "package.json").read_text())
    require((metadata["name"], metadata["version"]) == ("openclaw", VERSION),
            "unexpected candidate package identity")
    entry = package / pin["bin"]
    require(digest(entry.read_bytes()) == pin["files"][pin["bin"]], "candidate entry mismatch")
    scanner = package / spec["file"]
    raw = scanner.read_bytes()
    require(digest(raw) == contract["patched"] == spec["original"], "candidate scanner input mismatch")
    for change in spec["changes"]:
        old, new = change["old"].encode(), change["new"].encode()
        require(raw.count(old) == 1, "candidate scanner patch anchor mismatch")
        raw = raw.replace(old, new)
    require(digest(raw) == spec["patched"], "candidate scanner output mismatch")
    scanner.write_bytes(raw)
    return entry, spec["patched"]


def policy(node, runtime, state, workspace, extra_reads, port, model_port, mcp_port):
    # Both processes and the SQLite inspection worker inherit the same
    # filesystem and loopback limits. The worker uses the pinned Node path.
    def subpaths(paths):
        return " ".join(f"(subpath {json.dumps(str(path))})" for path in paths)

    mcp_network = f' (remote ip "localhost:{mcp_port}")' if mcp_port else ""
    return f"""(version 1)
(allow default)
(deny file-read-data)
(allow file-read-data (literal "/") (subpath "/System") (subpath "/usr/lib") (subpath "/usr/share"))
(deny file-read-data (subpath "/System/Volumes/Data"))
(allow file-read-data (literal "/dev/null") (literal "/dev/urandom") (literal "/dev/random")
 (literal "/private/etc/hosts") (literal "/private/etc/localtime")
 (literal {json.dumps(str(node))}) {subpaths((runtime, state, workspace, *extra_reads))})
(deny file-write*)
(allow file-write* {subpaths((state,))} (literal "/dev/null"))
(deny process-exec)
(allow process-exec (literal {json.dumps(str(node))}))
(deny network*)
(allow network-bind (local ip "localhost:{port}"))
(allow network-inbound (local ip "localhost:{port}"))
(allow network-outbound (remote ip "localhost:{port}") (remote ip "localhost:{model_port}"){mcp_network})
"""


@contextmanager
def gateway(command, workspace, env, log, port):
    with log.open("wb") as stderr:
        child = subprocess.Popen(command, cwd=workspace, env=env, stdin=subprocess.DEVNULL,
                                 stdout=stderr, stderr=stderr, start_new_session=True)
        try:
            for _ in range(300):
                require(child.poll() is None, "restricted OpenClaw Gateway exited before readiness")
                try:
                    with socket.create_connection(("127.0.0.1", port), timeout=0.2):
                        break
                except OSError:
                    time.sleep(0.1)
            else:
                raise AssertionError("restricted OpenClaw Gateway timed out")
            yield
        finally:
            if child.poll() is None:
                os.killpg(child.pid, signal.SIGKILL)
            child.wait()


@contextmanager
def mcp_relay(workspace, model, attachment, rejected, decisions, calls):
    patches = ROOT / "patches/acp-model-source"
    relay = runpy.run_path(str(patches / "omp/mcp_bridge.py"))
    stdio = runpy.run_path(str(patches / "cortex-code/mcp_bridge.py"))
    class AuditedStdio(stdio["Stdio"]):
        async def call(self, name, arguments):
            result = await super().call(name, arguments)
            audit = Path(self.scratch.name) / "audit.txt"
            calls.extend(audit.read_text().splitlines())
            audit.unlink()
            return result

    loop = asyncio.new_event_loop()
    session = relay["Session"](workspace, AuditedStdio, stdio["allowed_tool"])
    upstream = urlsplit(f"http://127.0.0.1:{model.server_port}")
    server = ThreadingHTTPServer(("127.0.0.1", 0), relay["handler"](loop, session, upstream, model.key))
    loop_thread = Thread(target=loop.run_forever)
    server_thread = Thread(target=server.serve_forever)
    loop_thread.start()
    server_thread.start()

    def submit(coroutine):
        return asyncio.run_coroutine_threadsafe(coroutine, loop).result(20)

    def emit(frame):
        params = frame["params"]
        require(frame["method"] == "session/request_permission" and params["sessionId"] == session.session,
                "unexpected MCP approval session")
        call = params["toolCall"]
        require(call["title"] == "mcp__coexistence__read_probe" and call["rawInput"] == {},
                "unexpected MCP approval tool")
        option = "reject_once" if rejected else "allow_once"
        decisions.append(option)
        loop.call_soon(session.decision, {"id": frame["id"], "result": {
            "outcome": {"outcome": "selected", "optionId": option},
        }})

    session.emit = emit
    script = workspace / "read_probe.py"
    script.write_text("import json, os, sys\nfrom pathlib import Path\n"
                      "def require(condition, message):\n    assert condition, message\n"
                      + inspect.getsource(fixture_rpc)
                      + "\nfor line in sys.stdin:\n"
                        "    reply = rpc(json.loads(line), sys.argv[1], Path(os.environ['TMPDIR']) / 'audit.txt', 'stdio')\n"
                        "    if reply is not None:\n        print(json.dumps(reply), flush=True)\n")
    framework_python = Path(sys.base_prefix) / "Resources/Python.app/Contents/MacOS/Python"
    python = framework_python if framework_python.is_file() else Path(sys.executable)
    params = {"cwd": str(workspace), "mcpServers": [{
        "name": "coexistence", "command": str(python.resolve(strict=True)),
        "args": [str(script), str(attachment)], "env": [],
        "_meta": {"shiju.omp.toolPolicy": {"allow": ["read_probe"]}},
    }]}
    try:
        submit(session.stage({"params": params}, server.server_port))
        url = params["mcpServers"][0]["url"]

        async def bind(session_id):
            session.session = session_id
            session.turn = session_id

        yield server.server_port, url, lambda session_id: submit(bind(session_id))
    finally:
        try:
            submit(session.clear())
        finally:
            server.shutdown()
            server.server_close()
            server_thread.join()
            loop.call_soon_threadsafe(loop.stop)
            loop_thread.join()
            loop.close()


def phase(root, entry, node, model, name, with_mcp):
    mounted = name in ('mounted', 'rejected')
    rejected = name == 'rejected'
    root.mkdir(mode=0o700)
    env = environment(root)
    workspace = root / "workspace"
    native = make_skill(workspace / "skills", "native")
    selected = make_skill(root / "selected", "selected")
    unselected = make_skill(root / "unselected", "unselected")
    make_skill(workspace / ".agents/skills", "other")
    view = root / "shiju-session-skills-candidate/skills"
    view.mkdir(parents=True)
    (view / selected["name"]).symlink_to(selected["path"], target_is_directory=True)
    state = root / "state"
    state.mkdir(mode=0o700)
    token = secrets.token_hex(24)
    token_file = state / "gateway-token"
    token_file.write_text(token)
    token_file.chmod(0o600)
    with socket.socket() as listener:
        listener.bind(("127.0.0.1", 0))
        port = listener.getsockname()[1]
    config_file = state / "openclaw.json"
    config = {
        "gateway": {"mode": "local", "bind": "loopback", "port": port,
                    "auth": {"mode": "token", "token": "${VW_OPENCLAW_GATEWAY_TOKEN}"},
                    "controlUi": {"enabled": False}},
        "discovery": {"mdns": {"mode": "off"}}, "browser": {"enabled": False},
        "plugins": {"enabled": False},
        "agents": {"defaults": {"workspace": str(workspace), "skipBootstrap": True,
                               "model": {"primary": "session-provider/" + MODEL, "fallbacks": []}}},
        "models": {"mode": "replace", "providers": {"session-provider": {
            "baseUrl": f"http://127.0.0.1:{model.server_port}/v1",
            "apiKey": "${VW_OPENCLAW_RELAY_TOKEN}", "api": "openai-completions",
            "models": [{"id": MODEL, "name": "Session model", "contextWindow": 100000,
                        "maxTokens": 1024, "reasoning": False, "input": ["text"]}]} }},
        "tools": {"allow": ["read"], "toolSearch": False, "fs": {"workspaceOnly": True}},
    }
    env.update(OPENCLAW_STATE_DIR=str(state), OPENCLAW_CONFIG_PATH=str(config_file),
               OPENCLAW_SKIP_CHANNELS="1", OPENCLAW_SKIP_CANVAS_HOST="1",
               VW_OPENCLAW_GATEWAY_TOKEN=token, VW_OPENCLAW_RELAY_TOKEN=model.key,
               SHIJU_OPENCLAW_NATIVE_SKILL_WORKSPACE=str(workspace),
               SHIJU_OPENCLAW_SKILL_ROOT=str(view) if mounted else "")
    env["HOME"] = env["TMPDIR"] = str(state)
    # This file is readable by the process, but must be denied to the native reader.
    boundary_file = state / "outside-reader.txt"
    boundary_value = secrets.token_hex(24)
    boundary_file.write_text(boundary_value)
    mcp_value = secrets.token_hex(24)
    attachment = workspace / "attachment.txt"
    attachment.write_text(mcp_value)
    decisions, calls = [], []
    relay = (mcp_relay(workspace, model, attachment, rejected, decisions, calls)
             if with_mcp and mounted else nullcontext((None, None, None)))
    with relay as (mcp_port, url, bind):
        if url is not None:
            config["mcp"] = {"servers": {"coexistence": {"url": url, "transport": "streamable-http"}}}
            config["tools"]["allow"].append(MCP_TOOL)
        config_file.write_text(json.dumps(config))
        model.probe = Probe({"skills": {"native": native, "selected": selected, "unselected": unselected},
                             "mcp": mcp_value, "read_boundary": {"path": boundary_file, "value": boundary_value}},
                            mounted, rejected, restricted=True, skills_only=not with_mcp)
        extras = (view, selected["path"]) if mounted else ()
        sandbox = policy(node, entry.parents[2], state, workspace, extras,
                         port, model.server_port, mcp_port)
        command = ["/usr/bin/sandbox-exec", "-p", sandbox, str(node), str(entry)]
        with gateway(command + ["gateway", "run", "--port", str(port), "--bind", "loopback"],
                     workspace, env, root / "gateway.log", port):
            client = Process(command + ["acp", "--url", f"ws://127.0.0.1:{port}",
                                       "--token-file", str(token_file), "--no-prefix-cwd"],
                             workspace, env, root / "bridge.log")
            try:
                client.call("initialize", {"protocolVersion": 1, "clientCapabilities": {}})
                session, _ = client.call("session/new", {"cwd": str(workspace), "mcpServers": []})
                if bind is not None:
                    bind(session["sessionId"])
                result, events = client.call("session/prompt", {"sessionId": session["sessionId"],
                    "prompt": [{"type": "text", "text": "Check the available read-only fixture resources."}]})
                require(not model.probe.failure, model.probe.failure)
                require(result["stopReason"] == "end_turn"
                        and "COEXISTENCE_DONE" in "".join(text_updates(events)), "unexpected candidate ACP outcome")
                expected = {"native", "native_read", "outside_read_denied"}
                if mounted:
                    expected.update(("selected", "selected_read"))
                if url is not None:
                    expected.add("mcp")
                    require(decisions == ["reject_once" if rejected else "allow_once"], "unexpected MCP approval count")
                require(set(model.probe.completed) == expected, "candidate native reads incomplete")
                require(calls == (["stdio"] if url is not None and not rejected else []),
                        "unexpected downstream MCP audit")
                return {"stop_reason": result["stopReason"], "model_requests": model.probe.requests,
                        "session": session["sessionId"], "calls": model.probe.completed,
                        "decisions": decisions, "mcp_calls": len(calls)}
            finally:
                client.close()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--with-mcp", action="store_true")
    args = parser.parse_args()
    output = args.output.resolve()
    output.parent.mkdir(parents=True, exist_ok=True)
    node = Path(shutil.which("node")).resolve(strict=True)
    with TemporaryDirectory(prefix="shiju-openclaw-candidate-") as directory:
        root = Path(directory).resolve()
        entry, sha256 = install_candidate(root / "runtime")
        report = {"scanner_sha256": sha256, "phases": {}}
        try:
            with model_server(Handler, secrets.token_hex(24)) as model:
                for name in (("baseline", "mounted", "removed", "rejected") if args.with_mcp
                             else ("baseline", "mounted", "removed")):
                    report["stage"] = name
                    report["phases"][name] = phase(root / name, entry, node, model, name, args.with_mcp)
            sessions = {row["session"] for row in report["phases"].values()}
            require(len(sessions) == len(report["phases"]), "candidate reused an ACP session")
            report.pop("stage")
            report["passed"] = True
        except Exception as error:
            report["failure"] = str(error) if isinstance(error, AssertionError) and not str(error).startswith("{") else type(error).__name__
            print(report["failure"], file=sys.stderr)
            return 1
        finally:
            output.write_text(json.dumps(report, indent=2) + "\n")
    return 0


if __name__ == "__main__":
    sys.exit(main())
