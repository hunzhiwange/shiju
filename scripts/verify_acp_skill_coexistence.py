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
from verify_acp_integration import platform_process, require
from verify_acp_model_source import ROOT, Unavailable

# Add a key only once its supplier module implements the reviewed probe interface.
AGENTS = ("claude", "qwen", "gemini", "codebuddy", "vibe", "snow", "goose", "hermes", "copilot", "kilocode", "kiro", "opencode", "openclaw", "pi", "iflow", "auggie", "cursor", "droid", "kimi", "qoder", "trae", "amp-acp", "aionrs", "autohand", "corust-agent", "cortex-code", "dimcode", "deepagents", "dirac", "glm-acp-agent")
AGENTS += ("grok",)
AGENTS += ("omp",)
AGENTS += ("junie",)
AGENT_PROFILE = "acp-coexistence"
PROVIDER = "coexistence-fixture"
AGENTS += ("harn",)
AGENTS += ("mimo-code",)
AGENTS += ("nova",)
AGENTS += ("poolside",)
AGENTS += ("sigit",)
AGENTS += ("stakpak",)
AGENTS += ("vtcode",)
PHASES = ("baseline", "mounted", "removed", "rejected")
IFLOW_NATIVE_PHASES = ("baseline", "native-rejected", "native-cancelled")
GLM_SKILL_PHASES = ("mounted", "removed")


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


def interactive_turn(url, query, body, probe, rejected, *, native_read_decision=None):
    decisions = []
    skills = []
    approves_skill = getattr(probe, "approves_skill", None)
    live_access = None
    sid = body["session_id"]
    request_id = secrets.token_hex(16) if native_read_decision == "cancel" else None
    stream_headers = {"X-VW-Stream-Request-ID": request_id} if request_id else {}
    stopped_permission = None
    with ThreadPoolExecutor(max_workers=1) as pool:
        stream = pool.submit(request, url, "/chat/stream" + query, body,
                             extra_headers=stream_headers)
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
                if native_read_decision == "cancel" and skill == "native_read":
                    require(stopped_permission in (None, row["id"]), "unexpected second pending read")
                    if stopped_permission is None:
                        stop = json.loads(request(url, f"/chat/stream/{request_id}/stop" + query,
                                                  {}, method="POST"))
                        require(stop["cancelled"] is True, "gateway did not cancel pending read")
                        stopped_permission = row["id"]
                        skills.append("native_read:cancel")
                    continue
                reply = "once" if skill or (is_mcp and not rejected) else "reject"
                if native_read_decision == "reject" and skill == "native_read":
                    reply = "reject"
                route = f"/permission/{row['id']}/reply"
                require(json.loads(request(url, route + query, {"reply": reply})) is True,
                        "permission reply failed")
                if skill:
                    skills.append(skill if reply == "once" else skill + ":reject")
                else:
                    decisions.append(reply if is_mcp else "unexpected")
            time.sleep(0.05)
        result = stream.result()
        if stopped_permission is not None:
            deadline = time.monotonic() + 5
            while True:
                pending = json.loads(request(url, "/permission" + query))
                if all(row["session_id"] != sid for row in pending):
                    break
                require(time.monotonic() < deadline, "cancelled read permission remained pending")
                time.sleep(0.05)
        return result, decisions, skills, live_access


def readback(url, query, sid, backend, model, live_access, *, require_live_access=True):
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
    require(summary.get("full_access") is False, "gateway requested full access")
    if access or require_live_access:
        require(access.get("requested") is False, "gateway permission readback mismatch")
    return {"binding": binding, "source": selected["source"], "model": model, "access": access}


def glm_skill_revision(binary, root, env, workspace, session, *, remove):
    log = root / ("glm-skill-remove.log" if remove else "glm-skill-load.log")
    with platform_process([str(binary), "acp"], workspace, env, log) as acp:
        acp.call("initialize", {"protocolVersion": 1, "clientCapabilities": {}})
        loaded, _ = acp.call("session/load", {"sessionId": session, "cwd": str(workspace),
                                             "mcpServers": []})
        revision = loaded["_meta"]["shiju.mounts"]["revision"]
        if not remove:
            return revision
        updated, _ = acp.call("_shiju/session/set_mounts", {"sessionId": session,
            "expectedRevision": revision, "skills": [], "mcpServers": []})
        require(updated["revision"] == revision + 1, "GLM session mount revision did not advance")
        return updated["revision"]


def run(agent, binary, report, phases=PHASES):
    supplier = import_module("acp_skill_coexistence." + agent.replace("-", "_"))
    iflow_native = agent == "iflow" and phases == IFLOW_NATIVE_PHASES
    glm_skills = agent == "glm-acp-agent" and phases == GLM_SKILL_PHASES
    temp_parent = getattr(supplier, "TEMP_PARENT", None)
    with tempfile.TemporaryDirectory(prefix="shiju-acp-coexistence-", dir=temp_parent) as directory:
        root = Path(directory).resolve()
        env = environment(root)
        report["stage"] = "artifact"
        spec, report["artifact"] = supplier.launch(env)
        workspace = root / "workspace"
        workspace.mkdir()
        selected_parent = (Path(env["HOME"]) / ".skills"
                           if getattr(supplier, "GLOBAL_SKILL_FIXTURES", False)
                           else workspace / ".shiju/skills")
        fixtures = {"skills": {
            "native": make_skill(workspace / supplier.NATIVE_DIR, "native"),
            "selected": make_skill(selected_parent, "selected"),
            "unselected": make_skill(selected_parent, "unselected"),
        }, "mcp": secrets.token_hex(24)}
        prepare_workspace = getattr(supplier, "prepare_workspace", None)
        if prepare_workspace is not None:
            prepare_workspace(workspace)
        attachment = root / "mcp.txt"
        attachment.write_text(fixtures["mcp"])
        audit = root / "mcp-audit.txt"
        config_path = workspace / ".vwacprc.json"
        native_start = root / "native-mcp-started"
        if iflow_native:
            # The provider's project config is distinct from the gateway's unselected MCP.
            native_settings = workspace / ".iflow/settings.json"
            native_settings.write_text(json.dumps({"mcpServers": {"native_probe": {
                "command": sys.executable,
                "args": ["-c", "from pathlib import Path; import sys; Path(sys.argv[1]).touch()",
                         str(native_start)],
            }}}))
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
            sid = None
            for phase in phases:
                report["stage"] = phase
                row = {"phase": phase}
                report["turns"].append(row)
                mounted = phase in ("mounted", "rejected")
                rejected = phase == "rejected"
                profile = {"enabled": True, "runtime": {"kind": "acp", "backend": agent},
                           "defaults": {"skills": {"mode": "fixed", "value": [selected] if mounted else []},
                                        "mcp": {"mode": "fixed", "value": ["coexistence"]
                                                if mounted and not glm_skills else []}}}
                if sid is None or not glm_skills:
                    request(url, "/global/config", {"agents": {AGENT_PROFILE: profile}}, method="PATCH")
                if sid is None or not glm_skills:
                    sid = json.loads(request(url, "/session" + query, {"title": "ACP coexistence " + phase}))["id"]
                row["session"] = sid
                if glm_skills:
                    # The attached default is already a local catalog skill.
                    # Record its starting revision, then explicitly revoke it.
                    row["mount_revision"] = glm_skill_revision(binary, root, env, workspace, sid, remove=not mounted)
                    if not mounted:
                        require(row["mount_revision"] == report["turns"][0]["mount_revision"] + 1,
                                "GLM removal did not follow the selected session revision")
                probe_options = {"skill_only": True, "restored": phase == "removed"} if glm_skills else {}
                model.probe = probe = supplier.Probe(fixtures, mounted, rejected, **probe_options)
                native_read_decision = {"native-rejected": "reject", "native-cancelled": "cancel"}.get(phase)
                if native_read_decision:
                    probe.native_read_decision = native_read_decision
                body = {"session_id": sid, "agent": AGENT_PROFILE, "acp_model_source": "session_model",
                        "model": model_id, "options": {"full_access": False, "cwd": str(workspace)},
                        "messages": [{"role": "user", "content": getattr(supplier, "PROMPT",
                            "Check the available read-only fixture resources.")}]}
                before = audit.read_text().splitlines() if audit.exists() else []
                stream, decisions, skill_approvals, live_access = interactive_turn(
                    url, query, body, probe, rejected, native_read_decision=native_read_decision)
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
                if iflow_native:
                    require(not native_start.exists(), "iFlow started an unprojected project MCP")
                require(after == before + [supplier.TRANSPORT] * row["mcp_calls"],
                        "MCP audit transport mismatch")
                if native_read_decision:
                    require(after == before and decisions == [], "negative native turn reached MCP")
                    require(skill_approvals == ["native", "native_read:" + native_read_decision],
                            "unexpected native read permission receipt")
                    require(fixtures["skills"]["native"]["attachment"] not in stream,
                            "blocked native read leaked its attachment to the gateway")
                    require(set(probe.calls) == {"native", "native_read"}, "native read was never requested")
                    if native_read_decision == "cancel":
                        require(row["done"] == 0 and row["errors"] == 1
                                and any(event.get("error") == "cancelled" for event in events),
                                "missing gateway cancellation terminal")
                        require(set(probe.completed) == {"native"}, "cancelled read returned to model")
                    else:
                        require(row["done"] == 1 and row["errors"] == 0 and probe.native_read_denied,
                                "missing correlated native read refusal")
                    continue
                if glm_skills:
                    expected = {"native", "native_read"}
                    if mounted:
                        expected.update(("selected", "selected_read"))
                    require(set(probe.completed) == expected, "incorrect GLM selected skill readbacks")
                    require(row["done"] == 1 and row["errors"] == 0
                            and after == before and decisions == [], "GLM skill-only turn failed")
                else:
                    verify_turn(row, mounted, rejected)
                if "skill_approvals" in row:
                    expected = ["native", "selected"] if mounted else ["native"]
                    if hasattr(probe, "expected_skill_approvals"):
                        expected = probe.expected_skill_approvals()
                    require(skill_approvals == expected, "missing or unexpected skill approval")
                # The resumed native turn can finish between access-status polls.
                row.update(readback(url, query, sid, agent, model_id, live_access,
                                    require_live_access=not (glm_skills and phase == "removed")))
            verify_native_plugins = getattr(supplier, "verify_native_plugins", None)
            if verify_native_plugins is not None:
                report["stage"] = "native_plugins"
                report["native_plugins"] = verify_native_plugins(spec, root, env, model, make_skill)
        if iflow_native:
            require(not probe.failure, probe.failure)
            require(not native_start.exists(), "iFlow started an unprojected project MCP during shutdown")
            require(not audit.exists() or not audit.read_text().splitlines(),
                    "unselected gateway MCP executed during native verification")
        sessions = {row["session"] for row in report["turns"]}
        require(len(sessions) == (1 if glm_skills else len(phases)), "unexpected session reuse")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--agent", choices=AGENTS, required=True)
    parser.add_argument("--gateway-bin", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--baseline-only", action="store_true",
                        help="verify only the native skill and read path")
    parser.add_argument("--iflow-native-approval", action="store_true",
                        help="verify Linux iFlow native approval, rejection and cancellation")
    parser.add_argument("--glm-skill-removal", action="store_true",
                        help="verify the selected Linux GLM skill view and removal in the same session")
    args = parser.parse_args()
    if args.glm_skill_removal and (args.agent != "glm-acp-agent" or platform.system() != "Linux"
                                   or args.baseline_only):
        parser.error("--glm-skill-removal requires Linux GLM and its mounted and removed phases")
    if args.iflow_native_approval and (args.agent != "iflow" or platform.system() != "Linux"
                                      or args.baseline_only):
        parser.error("--iflow-native-approval requires Linux iFlow and its three native phases")
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
        phases = ("baseline",) if args.baseline_only else PHASES
        if args.iflow_native_approval:
            phases = IFLOW_NATIVE_PHASES
        if args.glm_skill_removal:
            phases = GLM_SKILL_PHASES
        run(args.agent, binary, report, phases)
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
