"""Run the pinned Pi adapter directly to verify both skill catalogs and stdio approval."""

import json
from pathlib import Path
import secrets
import tempfile

from acp_skill_coexistence import correlated_result
from acp_skill_coexistence.pi import Handler, MCP_TOOL, MODEL, Probe, launch, mcp_server
from verify_acp_approval import environment
from verify_acp_integration import Process, require
from verify_acp_skill_coexistence import make_skill, model_server


class ResumedProbe(Probe):
    def catalog(self, body, tools):
        # Loaded transcripts legitimately keep tool results from earlier turns.
        super().catalog(dict(body, messages=[message for message in body["messages"]
                                             if message["role"] == "system"]), tools)


class RestrictedProbe(Probe):
    def __init__(self, fixtures, mounted, rejected, forbidden_path):
        super().__init__(fixtures, mounted, rejected)
        self.forbidden_path = forbidden_path
        self.boundary_call = None
        self.boundary_checked = False

    def response(self, body):
        tools = {tool["function"]["name"]: tool["function"] for tool in body.get("tools", [])}
        if self.forbidden_path is None or not tools:
            return super().response(body)
        if self.boundary_call is None:
            self.catalog(body, tools)
            response = self.read(tools, "boundary", self.forbidden_path)
            self.boundary_call = self.pending[1]
            return response
        results = [dict(message, tool_use_id=message["tool_call_id"]) for message in body["messages"]
                   if message["role"] == "tool"]
        result = str(correlated_result(results, self.boundary_call)["content"])
        require("EPERM" in result or "operation not permitted" in result.lower(),
                "Pi sandbox did not reject the foreign file")
        self.boundary_checked = True
        return {"content": "BOUNDARY_CHECKED"}, "stop"


def main():
    rows = []
    with tempfile.TemporaryDirectory(prefix="pi-direct-") as directory:
        root = Path(directory).resolve()
        env = environment(root)
        spec, artifact = launch(env)
        entry = Path(spec["command"])
        engine = entry.parents[2] / "@mariozechner/pi-coding-agent/dist/cli.js"
        workspace = root / "workspace"
        skills = {kind: make_skill(workspace / path, kind) for kind, path in
                  (("native", ".pi/skills"), ("unselected", ".shiju/skills"),
                   ("disabled", ".pi/skills"), ("foreign", ".agents/skills"))}
        skills["selected"] = make_skill(root / "external-skills", "selected")
        side_effect = workspace / "extension-loaded"
        ext = workspace / ".pi/extensions/forbidden.mjs"
        ext.parent.mkdir()
        ext.write_text("import { writeFileSync } from 'node:fs';\nwriteFileSync("
                       + json.dumps(str(side_effect)) + ", 'loaded');\nexport default function() {}")
        (workspace / ".pi/settings.json").write_text(json.dumps({
            "skills": ["-skills/" + skills["disabled"]["name"] + "/SKILL.md"]}))
        (workspace / ".pi/skills/escape").symlink_to(skills["foreign"]["path"], target_is_directory=True)
        view = root / "shiju-session-skills-fixture" / "skills"
        view.mkdir(parents=True)
        (view / skills["selected"]["name"]).symlink_to(skills["selected"]["path"], target_is_directory=True)
        # Pi must keep its first/native definition of a duplicate skill name.
        duplicate = view / "native-collision"
        duplicate.mkdir()
        (duplicate / "SKILL.md").write_text("---\nname: " + skills["native"]["name"]
                                          + "\ndescription: collision fixture\n---\n")
        attachment = root / "attachment"
        attachment.write_text(secrets.token_hex(24))
        audit = root / "audit"
        key = secrets.token_hex(24)
        sessions = root / "sessions"
        sessions.mkdir()
        agent = root / "agent"
        agent.mkdir()
        bound_state = root / "acp"
        with model_server(Handler, key) as model, mcp_server(workspace, attachment, audit) as mcp:
            (agent / "models.json").write_text(json.dumps({"providers": {"session-provider": {
                "baseUrl": f"http://127.0.0.1:{model.server_port}/v1",
                "api": "openai-completions", "apiKey": "VW_PI_RELAY_TOKEN", "models": [
                    {"id": MODEL, "reasoning": False, "input": ["text"],
                     "contextWindow": 16000, "maxTokens": 1000}]}}}))
            (agent / "settings.json").write_text(json.dumps({
                "defaultProvider": "session-provider", "defaultModel": MODEL, "sessionDir": str(sessions),
                "quietStartup": False, "enableSkillCommands": True}))
            env.update(PI_CODING_AGENT_DIR=str(agent), PI_ACP_STATE_DIR=str(bound_state),
                       PI_ACP_PI_COMMAND=str(engine), PI_OFFLINE="1", PI_SKIP_VERSION_CHECK="1",
                       VW_PI_RELAY_TOKEN=key, VW_PI_RESTRICTED="1")
            foreign = root / "foreign.txt"
            foreign.write_text(secrets.token_hex(24))
            escape = workspace / "escape.txt"
            escape.symlink_to(foreign)
            for phase in ("baseline", "mounted", "removed", "rejected", "load_removed",
                          "cancelled", "foreign_read", "symlink_read"):
                mounted = phase in ("mounted", "rejected", "cancelled")
                cancelled = phase == "cancelled"
                forbidden = {"foreign_read": foreign, "symlink_read": escape}.get(phase)
                rejected = phase == "rejected"
                selected = str(view) if mounted else ""
                proc = Process([str(entry)], workspace, dict(env, SHIJU_PI_SKILL_ROOT=selected), root / "log")
                fixtures = {"skills": skills, "mcp": attachment.read_text()}
                probe = (ResumedProbe(fixtures, mounted, rejected) if phase == "load_removed"
                         else RestrictedProbe(fixtures, mounted, rejected, forbidden))
                model.probe = probe
                approvals = []

                def approve(frame):
                    if frame.get("method") != "session/request_permission":
                        return
                    args = frame["params"]["toolCall"]
                    require(args["title"] == MCP_TOOL and args["rawInput"] == {}, "unexpected approval")
                    approvals.append(args["toolCallId"])
                    if cancelled:
                        proc.send("session/cancel", {"sessionId": sid}, notification=True)
                        return
                    proc.child.stdin.write(json.dumps({"jsonrpc": "2.0", "id": frame["id"], "result": {
                        "outcome": {"outcome": "selected", "optionId": "reject" if rejected else "allow"}}}) + "\n")
                    proc.child.stdin.flush()

                try:
                    proc.call("initialize", {"protocolVersion": 1, "clientCapabilities": {}})
                    params = {"cwd": str(workspace), "mcpServers": [mcp] if mounted else []}
                    if phase == "load_removed":
                        sid = rows[1]["session"]
                        proc.call("session/load", {**params, "sessionId": sid})
                    else:
                        new, _ = proc.call("session/new", params)
                        sid = new["sessionId"]
                        catalog = new["_meta"]["piAcp"]["startupInfo"]
                        for kind, skill in skills.items():
                            visible = kind == "native" or (kind == "selected" and mounted)
                            require((skill["name"] in catalog) == visible, "ACP catalog mismatch: " + kind)
                    before = audit.read_text().splitlines() if audit.exists() else []
                    result, _ = proc.call("session/prompt", {"sessionId": sid, "prompt": [
                        {"type": "text", "text": "Check available read-only resources."}]}, approve)
                    after = audit.read_text().splitlines() if audit.exists() else []
                    require(not probe.failure, probe.failure)
                    require(not side_effect.exists(), "Pi auto-loaded an extension")
                    require(result["stopReason"] == ("cancelled" if cancelled else "end_turn"),
                            "unexpected ACP stop")
                    if forbidden:
                        require(probe.boundary_checked, "no sandbox read result")
                        probe.completed["denied_read"] = probe.boundary_call
                    else:
                        expected = {"native", "native_read"}
                        if mounted:
                            expected.update(("selected", "selected_read"))
                            if not cancelled:
                                expected.add("mcp")
                        require(set(probe.completed) == expected, "incomplete correlated results")
                    if cancelled:
                        require(probe.pending[0] == "mcp" and approvals == [probe.pending[1]],
                                "cancelled approval ID mismatch")
                    else:
                        require(approvals == ([probe.completed["mcp"]] if mounted else []), "approval ID mismatch")
                    require(after == before + (["stdio"] if mounted and not rejected and not cancelled else []),
                            "unexpected stdio count")
                    commands = [f["params"]["update"]["availableCommands"] for f in proc.received
                                if f.get("method") == "session/update"
                                and f["params"]["update"]["sessionUpdate"] == "available_commands_update"]
                    require(commands, "ACP slash catalog missing")
                    names = {command["name"] for command in commands[-1]}
                    for kind, skill in skills.items():
                        visible = kind == "native" or (kind == "selected" and mounted)
                        require(("skill:" + skill["name"] in names) == visible, "ACP slash catalog mismatch: " + kind)
                    rows.append({"phase": phase, "session": sid, "calls": probe.completed,
                                 "stdio_calls": len(after) - len(before), "stop": result["stopReason"]})
                finally:
                    proc.close()
    print(json.dumps({"artifact": artifact, "turns": rows}, indent=2))


if __name__ == "__main__":
    main()
