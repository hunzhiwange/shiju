"""Exercise the reviewed Auggie skill scanner under a read-only Darwin sandbox."""

import argparse
import hashlib
import json
from pathlib import Path
import platform
import secrets
import shutil
import subprocess
import sys
import tempfile
from urllib.parse import urlsplit

from .auggie import Handler, MCP_TOOL, MODEL, NATIVE_DIR, Probe
from .hermes_sandbox import stdio_relay
from verify_acp_approval import environment
from verify_acp_integration import Process, require, text_updates
from verify_acp_model_source import ROOT
from verify_acp_skill_coexistence import PHASES, make_skill, model_server

DENIAL_PROBE = """
const fs = require("node:fs");
const net = require("node:net");
const {spawnSync} = require("node:child_process");
const assert = require("node:assert/strict");
const denied = error => ["EPERM", "EACCES"].includes(error?.code);
assert.throws(() => fs.readFileSync(process.argv[1]), denied);
assert.throws(() => fs.writeFileSync(process.argv[2], "unexpected"), denied);
assert(denied(spawnSync("/bin/sh", ["-c", "exit"]).error));
assert(denied(spawnSync(process.execPath, ["-e", "process.exit(17)"]).error));
const socket = net.connect(1, "127.0.0.1");
socket.once("connect", () => { throw new Error("unexpected connection"); });
socket.once("error", error => assert(denied(error)));
"""


class DirectProbe(Probe):
    def response(self, body):
        names = {item["function"]["name"] for item in body.get("tools", [])}
        require("web-fetch" not in names, "Auggie ignored project removedTools")
        return super().response(body)


class RestoreProbe(DirectProbe):
    def response(self, body):
        require(body["model"] == MODEL, "unexpected Auggie restored model")
        self.requests += 1
        catalogue = "\n".join(row["content"] for row in body["messages"] if row["role"] == "system")
        for kind, skill in self.fixtures["skills"].items():
            require((skill["name"] in catalogue) == (kind == "native"),
                    f"unexpected restored {kind} catalogue")
        require(all(row["function"]["name"] != MCP_TOOL for row in body.get("tools", [])),
                "restored Auggie retained an old MCP")
        return {"content": "COEXISTENCE_RESTORED"}, "stop"


class DirectHandler(Handler):
    def do_POST(self):
        # The supplier uses the auth endpoint for non-model RPCs in direct ACP.
        if self.path != "/v1/chat/completions":
            self.send_error(404)
            return
        super().do_POST()


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def copy_runtime(source, root):
    manifest = ROOT / "crates/libs/acp-agent-registry/src/managed_runtime/contracts.json"
    contract = json.loads(manifest.read_text())["auggie"]
    require(contract["version"] == "0.37.1", "unexpected Auggie contract")
    source = source.resolve(strict=True)
    package = root / "package"
    package.mkdir()
    for name, expected in contract["files"].items():
        original = source / name
        require(digest(original) == expected, "unexpected Auggie input digest")
        shutil.copy2(original, package / name)
    return package / contract["bin"]


def sandbox(node, read_dirs, read_files, write_dirs, ports):
    def rules(kind, paths):
        return " ".join(f"({kind} {json.dumps(str(path))})" for path in paths)

    network = " ".join(f'(remote ip "localhost:{port}")' for port in ports)
    return f"""(version 1)
(allow default)
(deny file-read-data)
(allow file-read-data (literal "/") (subpath "/System") (subpath "/usr/lib") (subpath "/usr/share")
    (literal "/private/etc/hosts") (literal "/private/etc/localtime") (literal "/dev/null")
    (literal "/dev/urandom") {rules("subpath", read_dirs)} {rules("literal", read_files)})
(deny file-read-data (subpath "/System/Volumes/Data"))
(deny file-write*)
(allow file-write* {rules("subpath", write_dirs)} (literal "/dev/null"))
(deny process-fork process-exec)
(allow process-exec (literal {json.dumps(str(node))}))
(deny network*)
(allow network-outbound {network})
"""


def run_phase(phase, root, entry, node, model, mcp, attachment, audit, foreign):
    root.mkdir(mode=0o700)
    env = environment(root)
    workspace, view, history = (root / directory for directory in
                                ("workspace", "shiju-session-skills-probe", "history"))
    for directory in (workspace, view, history):
        directory.mkdir(mode=0o700)
    mounted = phase in ("mounted", "rejected")
    rejected = phase == "rejected"
    skills = {
        "native": make_skill(workspace / NATIVE_DIR, "native"),
        "selected": make_skill(root / "selected-source", "selected"),
        "unselected": make_skill(root / "unselected-source", "unselected"),
    }
    selected = skills["selected"]["path"]
    if mounted:
        (view / selected.name).symlink_to(selected, target_is_directory=True)
    skills["disabled"] = make_skill(workspace / NATIVE_DIR, "disabled")
    features = Path(env["HOME"]) / ".augment/feature-config.json"
    features.parent.mkdir(mode=0o700)
    features.write_text(json.dumps({"skillModes": {skills["disabled"]["name"]: "disabled"}}))
    settings = workspace / ".augment/settings.json"
    project_settings = json.dumps({"removedTools": ["web-fetch"]})
    settings.write_text(project_settings)
    endpoint = f"http://127.0.0.1:{model.server_port}/"
    config, auth = root / "model.json", root / "auth.json"
    config.write_text(json.dumps({"model": MODEL, "endpoint": endpoint, "token": model.key,
                                  "outputLimit": 1024}))
    auth.write_text(json.dumps({"accessToken": model.key, "tenantURL": endpoint, "scopes": ["email"]}))
    env.update(SHIJU_AUGGIE_MODEL_CONFIG=str(config), AUGMENT_DISABLE_AUTO_UPDATE="1",
               SHIJU_AUGGIE_SKILL_VIEW=str(view) if mounted else "", NO_PROXY="*",
               SHIJU_AUGGIE_RESTRICTED="1", SHIJU_AUGGIE_WORKSPACE=str(workspace),
               SHIJU_AUGGIE_MCP_TOOLS=json.dumps([MCP_TOOL] if mounted else []))
    private = [Path(env["HOME"]), Path(env["TMPDIR"]), history]
    read_dirs = [workspace, *private]
    read_files = [node, config, auth, entry, entry.with_name("session-model.mjs")]
    ports = [model.server_port]
    if mounted:
        read_dirs.extend((view, selected))
        ports.append(urlsplit(mcp["url"]).port)
    policy = sandbox(node, read_dirs, read_files, private, ports)
    command = ["/usr/bin/sandbox-exec", "-p", policy, str(node)]
    subprocess.run([*command, "-e", DENIAL_PROBE, str(foreign), str(workspace / "write-test")],
                   env=env, cwd=workspace, check=True, timeout=10, capture_output=True)
    flags = ["--permission", MCP_TOOL + ":ask-user"] if mounted else []
    model.probe = probe = DirectProbe({"skills": skills, "mcp": attachment.read_text()}, mounted, rejected, observe_stdio=False)
    before = len(audit.read_text().splitlines()) if audit.exists() else 0
    process = Process([*command, str(entry), "--acp", "--model", MODEL, "--augment-cache-dir",
                       str(history), "--augment-session-json", str(auth), *flags],
                      workspace, env, root / "stderr")
    decisions = []

    def on_frame(frame):
        if frame.get("method") != "session/request_permission":
            return
        call = frame["params"]["toolCall"]
        require(probe.pending == ("mcp", call["toolCallId"]) and call.get("title") == "mcp__auggie__" + MCP_TOOL
                and call.get("rawInput") == {}, "unexpected Auggie approval")
        kind = "reject_once" if rejected else "allow_once"
        option = next(row for row in frame["params"]["options"] if row["kind"] == kind)
        response = {"jsonrpc": "2.0", "id": frame["id"],
                    "result": {"outcome": {"outcome": "selected", "optionId": option["optionId"]}}}
        process.child.stdin.write(json.dumps(response) + "\n")
        process.child.stdin.flush()
        decisions.append(kind)

    try:
        process.call("initialize", {"protocolVersion": 1, "clientCapabilities": {}})
        session, _ = process.call("session/new", {"cwd": str(workspace),
                                                  "mcpServers": [mcp] if mounted else []})
        require(session["models"]["currentModelId"] == MODEL, "unexpected Auggie bound model")
        result, events = process.call("session/prompt", {
            "sessionId": session["sessionId"],
            "prompt": [{"type": "text", "text": "Check available read-only resources."}],
        }, on_frame)
    finally:
        process.close()
    after = len(audit.read_text().splitlines()) if audit.exists() else 0
    require(probe.failure is None, probe.failure)
    require(result["stopReason"] == "end_turn" and "".join(text_updates(events)) == "COEXISTENCE_DONE",
            "unexpected Auggie prompt result")
    expected = {"native", "native_read"}
    if mounted:
        expected.update(("selected", "selected_read", "mcp"))
    require(set(probe.completed) == expected and probe.requests == len(expected) + 1,
            "incomplete Auggie continuation")
    require(decisions == (["reject_once" if rejected else "allow_once"] if mounted else []),
            "unexpected Auggie approval count")
    require(after - before == int(mounted and not rejected), "unexpected Auggie downstream calls")
    require(settings.read_text() == project_settings, "Auggie changed project settings")
    row = {"phase": phase, "session": session["sessionId"], "stop": result["stopReason"],
           "calls": probe.completed, "requests": probe.requests, "decisions": decisions,
           "downstream": after - before, "sandbox_denials": 5}
    if phase == "mounted":
        # Resume the original vendor conversation in a new process after revoking the view and relay.
        (view / selected.name).unlink()
        policy = sandbox(node, [workspace, *private], read_files,
                         private, [model.server_port])
        env.update(SHIJU_AUGGIE_SKILL_VIEW="", SHIJU_AUGGIE_MCP_TOOLS="[]")
        model.probe = restore = RestoreProbe(probe.fixtures, False, False, observe_stdio=False)
        process = Process(["/usr/bin/sandbox-exec", "-p", policy, str(node), str(entry), "--acp",
                           "--model", MODEL, "--augment-cache-dir", str(history),
                           "--augment-session-json", str(auth)], workspace, env, root / "restore-stderr")
        try:
            process.call("initialize", {"protocolVersion": 1, "clientCapabilities": {}})
            process.call("session/load", {"sessionId": session["sessionId"],
                                          "cwd": str(workspace), "mcpServers": []})
            result, events = process.call("session/prompt", {
                "sessionId": session["sessionId"],
                "prompt": [{"type": "text", "text": "Check the current resource catalogue."}],
            })
        finally:
            process.close()
        require(restore.failure is None, restore.failure)
        require(result["stopReason"] == "end_turn" and restore.requests == 1
                and "".join(text_updates(events)) == "COEXISTENCE_RESTORED",
                "Auggie restore did not use the current catalogue")
        require(len(audit.read_text().splitlines()) == after, "Auggie restore executed revoked MCP")
        row["restore"] = {"stop": result["stopReason"], "requests": restore.requests, "downstream": 0}
    return row


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--package", type=Path, required=True, help="verified 0.37.1 package directory")
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    require((platform.system(), platform.machine()) == ("Darwin", "arm64"),
            "unsupported Auggie sandbox platform")
    node = Path(shutil.which("node") or "/nonexistent").resolve(strict=True)
    with tempfile.TemporaryDirectory(prefix="shiju-auggie-sandbox-") as folder:
        root = Path(folder).resolve()
        entry = copy_runtime(args.package, root)
        foreign, attachment, audit = (root / name for name in ("foreign", "value", "audit"))
        foreign.write_text(secrets.token_hex(24))
        attachment.write_text(secrets.token_hex(24))
        with model_server(DirectHandler, secrets.token_hex(24)) as model:
            with stdio_relay(attachment, audit) as mcp:
                turns = [run_phase(phase, root / phase, entry, node, model, mcp, attachment, audit,
                                   foreign) for phase in PHASES]
        require(len({row["session"] for row in turns}) == len(PHASES), "Auggie reused a session")
        report = {"platform": "Darwin-arm64", "model": MODEL, "node_sha256": digest(node),
                  "entry_sha256": digest(entry), "turns": turns}
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(report, indent=2) + "\n")


if __name__ == "__main__":
    main()
