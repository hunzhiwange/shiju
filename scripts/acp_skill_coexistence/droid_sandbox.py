"""Run the Droid 1.9.0 coexistence candidate through isolated native ACP."""

import argparse
from contextlib import ExitStack
import json
from pathlib import Path
import platform
import secrets
from tempfile import TemporaryDirectory
from urllib.parse import urlsplit

from acp_skill_coexistence import droid
from acp_skill_coexistence.hermes_sandbox import stdio_relay
from verify_acp_approval import environment
from verify_acp_integration import Process, require
from verify_acp_skill_coexistence import PHASES, make_skill, model_server


def policy(entry, root, workspace, home, scratch, skill_roots, settings, ports):
    def literal(path):
        return f"(literal {json.dumps(str(path))})"

    def subpath(path):
        return f"(subpath {json.dumps(str(path))})"

    reads = " ".join(subpath(path) for path in (home, scratch, *skill_roots))
    outbound = "\n".join(f'(allow network-outbound (remote ip "localhost:{port}"))'
                         for port in ports)
    return f"""(version 1)
(allow default)
(deny file-read-data)
(allow file-read-data (literal "/") (subpath "/System") (subpath "/usr/lib") (subpath "/usr/share"))
(deny file-read-data (subpath "/System/Volumes/Data"))
(allow file-read-data (literal "/dev/null") (literal "/dev/urandom")
 (literal "/private/etc/hosts") (literal "/private/etc/localtime")
 {literal(entry)} {literal(root)} {literal(workspace)} {literal(settings)} {reads})
(deny file-write*)
(allow file-write* {subpath(home)} {subpath(scratch)} (literal "/dev/null"))
(deny process-exec)
(allow process-exec {literal(entry)})
(deny network*)
{outbound}
"""


def run_phase(phase, report):
    mounted = phase in ("mounted", "rejected")
    rejected = phase == "rejected"
    with TemporaryDirectory(prefix="shiju-droid-sandbox-") as directory, ExitStack() as stack:
        root = Path(directory).resolve()
        env = environment(root)
        spec, report["artifact"] = droid.launch(env)
        entry = Path(spec["command"]).resolve()
        workspace = root / "workspace"
        home = root / "shiju-acp-launch-fixture"
        home.mkdir(mode=0o700)
        native = workspace / droid.NATIVE_DIR
        skills = {
            "native": make_skill(native, "native"),
            "selected": make_skill(workspace / ".shiju-harness/skills", "selected"),
            "unselected": make_skill(workspace / ".shiju-harness/skills", "unselected"),
            "disabled": make_skill(native, "disabled"),
        }
        project_settings = workspace / ".factory/settings.json"
        project_settings.write_text(json.dumps({"disabledSkills": [skills["disabled"]["name"]]}))
        reads = [native]
        if mounted:
            view = root / "shiju-session-skills-fixture/skills"
            view.parent.mkdir(mode=0o700)
            view.mkdir()
            (view / skills["selected"]["name"]).symlink_to(skills["selected"]["path"])
            (home / ".factory").mkdir()
            (home / droid.NATIVE_DIR).symlink_to(view)
            reads.extend((view.parent, skills["selected"]["path"]))

        attachment, audit = root / "attachment.txt", root / "mcp.audit"
        value = secrets.token_hex(24)
        attachment.write_text(value)
        server = stack.enter_context(stdio_relay(attachment, audit)) if mounted else None
        model = stack.enter_context(model_server(droid.Handler, secrets.token_hex(24)))
        probe = model.probe = droid.Probe(
            {"skills": skills, "mcp": value}, mounted, rejected, observe_stdio=False)
        ports = [model.server_port]
        if server is not None:
            ports.append(urlsplit(server["url"]).port)
        private_env = dict(env, HOME=str(home), PATH="/usr/bin:/bin",
                           FACTORY_HOME_OVERRIDE=str(home), FACTORY_AIRGAP_ENABLED="true",
                           FACTORY_DISABLE_DYNAMIC_CONFIG="true", FACTORY_DISABLE_KEYRING="true",
                           NO_PROXY="127.0.0.1,localhost")
        model_id = "custom:Session-Selected-0"
        settings = home / "settings.json"
        settings.write_text(json.dumps({
            "customModels": [{"model": droid.MODEL, "displayName": "Session Selected",
                              "baseUrl": f"http://127.0.0.1:{model.server_port}/v1",
                              "apiKey": model.key, "provider": "generic-chat-completion-api",
                              "maxOutputTokens": 1024}],
            "model": model_id, "specModel": model_id,
            "hooksDisabled": True, "cloudSessionSync": False, "ideAutoConnect": False,
        }))
        sandbox = policy(entry, root, workspace, home, Path(env["TMPDIR"]), reads,
                         project_settings, ports)
        command = ["/usr/bin/sandbox-exec", "-p", sandbox, str(entry), *spec["args"],
                   "--settings", str(settings), "--model", model_id, "--spec-model", model_id]
        process = Process(command, home, private_env, root / "droid.stderr")
        stack.callback(process.close)
        report["stage"] = "initialize"
        process.call("initialize", {"protocolVersion": 1, "clientCapabilities": {}})
        report["stage"] = "session/new"
        session, _ = process.call("session/new", {
            "cwd": str(workspace), "mcpServers": [server] if server is not None else [],
        })
        session_id = session["sessionId"]
        report.update(session=session_id, mode=session["modes"]["currentModeId"])
        require(report["mode"] == "auto-medium", "unexpected Droid native mode")
        approvals = []

        def approve(frame):
            if "id" not in frame or "method" not in frame:
                return
            require(frame["method"] == "session/request_permission" and mounted and not approvals,
                    "unexpected Droid permission request")
            params = frame["params"]
            call = params["toolCall"]
            require(params["sessionId"] == session_id and probe.pending == ("mcp", droid.MCP_TOOL, {})
                    and call["title"] == droid.MCP_TITLE and call["rawInput"] == {}
                    and call["_meta"]["factory"] == {"source": "mcp", "server": "coexistence",
                                                    "tool": "read_probe"},
                    "unexpected Droid MCP approval target")
            kind = "reject_once" if rejected else "allow_once"
            choices = [option["optionId"] for option in params["options"] if option["kind"] == kind]
            require(len(choices) == 1, "missing or ambiguous Droid permission choice")
            approvals.append({"call_id": call["toolCallId"], "decision": kind})
            response = {"jsonrpc": "2.0", "id": frame["id"], "result": {
                "outcome": {"outcome": "selected", "optionId": choices[0]}}}
            process.child.stdin.write(json.dumps(response) + "\n")
            process.child.stdin.flush()

        try:
            report["stage"] = "session/prompt"
            result, events = process.call("session/prompt", {
                "sessionId": session_id,
                "prompt": [{"type": "text", "text": "Check the available read-only fixture resources."}],
            }, approve)
        finally:
            report.update(model_requests=probe.requests, calls=probe.completed, approvals=approvals,
                          mcp_calls=len(audit.read_text().splitlines()) if audit.exists() else 0)
        report["stop"] = result["stopReason"]
        require(probe.failure is None, "Droid model fixture rejected the conversation")
        require(report["stop"] == ("cancelled" if rejected else "end_turn"), "unexpected ACP terminal")
        require(len(approvals) == int(mounted), "unexpected ACP approval count")
        require(report["mcp_calls"] == int(mounted and not rejected), "unexpected downstream call count")
        expected = {"native", "native_read"}
        if mounted:
            expected.update(("selected", "selected_read", "mcp"))
            approval_id = approvals[0]["call_id"]
            if rejected:
                updates = [event["params"]["update"] for event in events
                           if event.get("method") == "session/update"]
                failures = [update for update in updates if update.get("toolCallId") == approval_id
                            and update.get("status") == "failed"]
                require(len(failures) == 1
                        and "Tool execution cancelled by user" in json.dumps(failures[0]["content"])
                        and value not in json.dumps(failures[0]["content"]), "Droid denial missing")
                probe.completed["mcp"] = approval_id
            require(probe.completed.get("mcp") == approval_id, "MCP result and approval IDs differ")
        require(set(probe.completed) == expected and len(set(probe.completed.values())) == len(expected),
                "required correlated Droid tool calls missing")
        require(probe.requests == len(expected) + int(not rejected), "unexpected model continuation count")
        report.pop("stage")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    report = {"platform": f"{platform.system()}-{platform.machine()}", "passed": False, "turns": []}
    try:
        for phase in PHASES:
            turn = {"phase": phase}
            report["turns"].append(turn)
            run_phase(phase, turn)
        require(len({turn["session"] for turn in report["turns"]}) == len(PHASES),
                "Droid reused an ACP session")
        report["passed"] = True
    except Exception as error:
        # ACP envelopes can include raw tool results and local paths.
        report["failure"] = type(error).__name__
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(report, indent=2) + "\n")
    return int(not report["passed"])


if __name__ == "__main__":
    raise SystemExit(main())
