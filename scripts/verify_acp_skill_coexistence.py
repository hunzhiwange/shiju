#!/usr/bin/env python3
"""Run native ACP skill coexistence through an isolated production gateway."""
import argparse
from concurrent.futures import ThreadPoolExecutor
from contextlib import ExitStack, contextmanager
import hashlib
from http.server import ThreadingHTTPServer
from importlib import import_module
import json
import os
from pathlib import Path
import platform
import secrets
import sys
import tempfile
import threading
import time
from urllib.parse import urlencode

from acp_mount_integration import McpHandler
from acp_skill_coexistence import verify_turn
from verify_acp_approval import environment, gateway, request, stream_failure_categories
from verify_acp_integration import require
from verify_acp_model_source import ROOT, Unavailable

# Add a key only once its supplier module implements the reviewed probe interface.
AGENTS = ("claude", "qwen", "gemini", "codebuddy", "vibe", "snow", "goose", "hermes", "copilot", "kilocode", "kiro", "opencode", "openclaw", "pi", "iflow", "auggie", "cursor", "droid", "kimi", "qoder", "trae", "amp-acp", "aionrs", "autohand", "corust-agent", "dimcode", "deepagents", "dirac", "glm-acp-agent", "cortex-code")
AGENTS += ("grok",)
AGENTS += ("omp",)
AGENTS += ("junie",)
AGENT_PROFILE = "acp-coexistence"
PROVIDER = "coexistence-fixture"
AGENTS += ("harn",)
AGENTS += ("poolside",)
AGENTS += ("mimo-code",)
AGENTS += ("nova",)
AGENTS += ("vtcode",)
PHASES = ("baseline", "mounted", "removed", "rejected")


@contextmanager
def model_server(handler, key):
    with ThreadingHTTPServer(("127.0.0.1", 0), handler) as server:
        server.key = key
        thread = threading.Thread(target=server.serve_forever, daemon=True)
        thread.start()
        try:
            yield server
        finally:
            server.shutdown()
            thread.join()


class SseMcpHandler(McpHandler):
    def do_GET(self):
        if self.authorized():
            super().do_GET()

    def do_POST(self):
        if self.authorized():
            super().do_POST()

    def authorized(self):
        if self.headers.get("Authorization") == "Bearer " + self.server.key:
            return True
        self.close_connection = True
        self.reply(401)
        return False


@contextmanager
def mcp_server(transport, attachment, audit):
    if transport == "stdio":
        yield {"name": "coexistence", "command": sys.executable,
               "args": [str(ROOT / "scripts/acp_mount_integration.py"), "--mcp-stdio",
                        str(attachment), str(audit)], "env": []}
        return
    require(transport == "sse", "unimplemented reviewed MCP transport")
    key = secrets.token_hex(24)
    with model_server(SseMcpHandler, key) as server:
        server.attachment = attachment
        server.audit = audit
        server.channels = {}
        server.stopping = threading.Event()
        try:
            yield {"name": "coexistence", "type": "sse",
                   "url": f"http://127.0.0.1:{server.server_port}/sse",
                   "headers": [{"name": "Authorization", "value": "Bearer " + key}]}
        finally:
            server.stopping.set()


def make_skill(parent, kind):
    name = f"{kind}-probe-{secrets.token_hex(4)}"
    path = parent / name
    (path / "references").mkdir(parents=True)
    body, attachment = secrets.token_hex(24), secrets.token_hex(24)
    (path / "SKILL.md").write_text(
        f"---\nname: {name}\ndescription: Read the {kind} fixture reference\n---\n"
        f"{body}\nRead references/value.txt using the native file reader.\n")
    (path / "references/value.txt").write_text(attachment)
    return {"name": name, "path": path, "body": body, "attachment": attachment}


def interactive_turn(url, query, body, probe, rejected):
    decisions = []
    skills = []
    approves_skill = getattr(probe, "approves_skill", None)
    live_access = None
    sid = body["session_id"]
    with ThreadPoolExecutor(max_workers=1) as pool:
        stream = pool.submit(request, url, "/chat/stream" + query, body)
        while not stream.done():
            if live_access is None or live_access.get("support") == "failed":
                controls = json.loads(request(url, f"/session/{sid}/acp-options" + query))
                if controls.get("connected"):
                    live_access = controls.get("access_status")
            pending = json.loads(request(url, "/permission" + query))
            for row in pending:
                if row["session_id"] != body["session_id"]:
                    continue
                metadata = row["metadata"]
                require(metadata.get("requested_channel") == "desktop-acp", "foreign approval channel")
                is_mcp = probe.approves(metadata)
                skill = approves_skill(metadata) if approves_skill is not None else None
                reply = "once" if skill or (is_mcp and not rejected) else "reject"
                route = f"/permission/{row['id']}/reply"
                require(json.loads(request(url, route + query, {"reply": reply})) is True,
                        "permission reply failed")
                if skill:
                    skills.append(skill)
                else:
                    decisions.append(reply if is_mcp else "unexpected")
            time.sleep(0.05)
        return stream.result(), decisions, skills, live_access


def readback(url, query, sid, backend, model, live_access):
    options = json.loads(request(url, f"/session/{sid}/acp-options" + query))
    summary = json.loads(request(url, f"/session/{sid}/ui/summary" + query))
    selected = options.get("selected_model") or {}
    access = live_access or options.get("access_status") or {}
    binding = summary.get("agent_binding") or {}
    require(binding.get("acp_agent") == backend and binding.get("agent") == AGENT_PROFILE,
            "gateway backend binding mismatch")
    require(options.get("acp_model_source") == selected.get("source") == "session_model"
            and selected.get("model") == model and selected.get("provider_id") == PROVIDER,
            "gateway selected model mismatch")
    require(summary.get("full_access") is False and access.get("requested") is False,
            "gateway permission readback mismatch")
    return {"binding": binding, "source": selected["source"], "model": model, "access": access}


def run(agent, binary, report):
    supplier = import_module("acp_skill_coexistence." + agent.replace("-", "_"))
    temp_parent = getattr(supplier, "TEMP_PARENT", None)
    with tempfile.TemporaryDirectory(prefix="shiju-acp-coexistence-", dir=temp_parent) as directory:
        root = Path(directory).resolve()
        env = environment(root)
        report["stage"] = "artifact"
        spec, report["artifact"] = supplier.launch(env)
        workspace = root / "workspace"
        workspace.mkdir()
        fixtures = {"skills": {
            "native": make_skill(workspace / supplier.NATIVE_DIR, "native"),
            "selected": make_skill(workspace / ".shiju/skills", "selected"),
            "unselected": make_skill(workspace / ".shiju/skills", "unselected"),
        }, "mcp": secrets.token_hex(24)}
        prepare_workspace = getattr(supplier, "prepare_workspace", None)
        if prepare_workspace is not None:
            prepare_workspace(workspace)
        attachment = root / "mcp.txt"
        attachment.write_text(fixtures["mcp"])
        audit = root / "mcp-audit.txt"
        config_path = workspace / ".vwacprc.json"
        env["ACP_COEXISTENCE_KEY"] = secrets.token_hex(24)
        query = "?" + urlencode({"directory": str(workspace)})
        model_id = PROVIDER + "/" + supplier.MODEL
        selected = fixtures["skills"]["selected"]["name"]
        report["stage"] = "gateway"
        with ExitStack() as stack:
            supplier_mcp = getattr(supplier, "mcp_server", None)
            mcp = stack.enter_context(supplier_mcp(workspace, attachment, audit) if supplier_mcp
                                      else mcp_server(supplier.TRANSPORT, attachment, audit))
            config_path.write_text(json.dumps({"mcpServers": [mcp]}))
            model = stack.enter_context(model_server(supplier.Handler, env["ACP_COEXISTENCE_KEY"]))
            gateway_options = {"cwd": workspace} if getattr(supplier, "GATEWAY_WORKSPACE_CWD", False) else {}
            url = stack.enter_context(gateway(str(binary), root, env, **gateway_options))
            endpoint = f"http://127.0.0.1:{model.server_port}{supplier.API_PATH}"
            config = {"workspace_dir": str(workspace), "acp": {agent: spec},
                      "providers": {PROVIDER: {"name": "Private coexistence fixture", "api": endpoint,
                          "adapter": supplier.ADAPTER, "env": ["ACP_COEXISTENCE_KEY"],
                          "models": {supplier.MODEL: {"status": "active", "temperature": True,
                              "tool_call": True, "reasoning": getattr(supplier, "MODEL_REASONING", False),
                              "limit": {"context": 100000,
                                  "output": getattr(supplier, "MODEL_OUTPUT_LIMIT", 1024)}}}}}}
            request(url, "/global/config", config, method="PATCH")
            for phase in PHASES:
                report["stage"] = phase
                row = {"phase": phase}
                report["turns"].append(row)
                mounted = phase in ("mounted", "rejected")
                rejected = phase == "rejected"
                profile = {"enabled": True, "runtime": {"kind": "acp", "backend": agent},
                           "defaults": {"skills": {"mode": "fixed", "value": [selected] if mounted else []},
                                        "mcp": {"mode": "fixed", "value": ["coexistence"] if mounted else []}}}
                request(url, "/global/config", {"agents": {AGENT_PROFILE: profile}}, method="PATCH")
                sid = json.loads(request(url, "/session" + query, {"title": "ACP coexistence " + phase}))["id"]
                row["session"] = sid
                model.probe = probe = supplier.Probe(fixtures, mounted, rejected)
                body = {"session_id": sid, "agent": AGENT_PROFILE, "acp_model_source": "session_model",
                        "model": model_id, "options": {"full_access": False, "cwd": str(workspace)},
                        "messages": [{"role": "user", "content": getattr(supplier, "PROMPT",
                            "Check the available read-only fixture resources.")}]}
                before = audit.read_text().splitlines() if audit.exists() else []
                stream, decisions, skill_approvals, live_access = interactive_turn(url, query, body, probe, rejected)
                after = audit.read_text().splitlines() if audit.exists() else []
                events = [json.loads(line[6:]) for line in stream.splitlines() if line.startswith("data: {")]
                observe_terminal = getattr(probe, "observe_terminal", None)
                if observe_terminal is not None and observe_terminal(events):
                    row["terminal_mcp_rejection"] = True
                mcp_trace = getattr(probe, "mcp_trace", None)
                if mcp_trace is not None and not probe.failure and probe.completed:
                    after, row["mcp_bridge_call_id"] = mcp_trace(events)
                types = [event.get("type") for event in events]
                row.update(done=types.count("chat.done"), errors=types.count("chat.error"),
                           mcp_calls=len(after) - len(before), mcp_decisions=decisions,
                           model_requests=probe.requests, calls=probe.completed)
                if getattr(probe, "approves_skill", None) is not None:
                    row["skill_approvals"] = skill_approvals
                if row["errors"]:
                    row["gateway_failures"] = stream_failure_categories(stream)
                require(not probe.failure, probe.failure)
                require(after == before + [supplier.TRANSPORT] * row["mcp_calls"],
                        "MCP audit transport mismatch")
                verify_turn(row, mounted, rejected)
                if "skill_approvals" in row:
                    expected = ["native", "selected"] if mounted else ["native"]
                    if hasattr(probe, "expected_skill_approvals"):
                        expected = probe.expected_skill_approvals()
                    require(skill_approvals == expected, "missing or unexpected skill approval")
                row.update(readback(url, query, sid, agent, model_id, live_access))
            verify_native_plugins = getattr(supplier, "verify_native_plugins", None)
            if verify_native_plugins is not None:
                report["stage"] = "native_plugins"
                report["native_plugins"] = verify_native_plugins(spec, root, env, model, make_skill)
        require(len({row["session"] for row in report["turns"]}) == len(PHASES), "sessions reused across phases")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--agent", choices=AGENTS, required=True)
    parser.add_argument("--gateway-bin", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    report = {"agent": args.agent, "platform": f"{platform.system()}-{platform.machine()}",
              "cargo_target_dir": os.environ.get("CARGO_TARGET_DIR"),
              "cargo_build_build_dir": os.environ.get("CARGO_BUILD_BUILD_DIR"),
              "turns": [], "passed": False}
    try:
        binary = args.gateway_bin.resolve(strict=True)
        digest = hashlib.sha256()
        with binary.open("rb") as stream:
            for block in iter(lambda: stream.read(1024 * 1024), b""):
                digest.update(block)
        report["gateway_sha256"] = digest.hexdigest()
        run(args.agent, binary, report)
        report.pop("stage", None)
        report["passed"] = True
    except AssertionError as error:
        report["failure"] = str(error)
    except (OSError, Unavailable, ValueError, KeyError, TypeError) as error:
        # Gateway and provider errors can embed request content. Keep only the class.
        report["failure"] = type(error).__name__
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(report, indent=2) + "\n")
    if not report["passed"]:
        print("Coexistence failed: " + report["failure"], file=sys.stderr)
    return int(not report["passed"])


if __name__ == "__main__":
    sys.exit(main())
