#!/usr/bin/env python3
"""Exercise a non-interactive plan run with the pinned Vibe ACP adapter and a loopback model."""
import argparse
import json
import os
from pathlib import Path
import signal
import tempfile
from urllib.parse import urlencode

from acp_approval_model import CONTENTS, DONE, model_fixture, tool_inputs
from verify_acp_approval import MANIFEST, environment, gateway, interrupt, request, require
from verify_acp_model_source import cargo_slot, isolated_environment, locate


def add_plan_artifacts(fixture, workspace, outside):
    """Use the adapter's real file tool to generate a minimal, complete plan."""
    base_next_tool = fixture.next_tool
    task_file = "1-verify.md"
    files = {
        "plan.md": "# Isolated ACP plan\n",
        "prompt.md": "# Verify\n",
        "tasks.json": json.dumps({
            "plan_dir": ".",
            "tasks": [{"id": "verify", "title": "Verify", "file": task_file,
                       "batch": 1, "status": "未开始"}],
        }),
        task_file: "# Verify\n状态：已完成\n完成条件：检查标记文件\n"
                   "验证命令：cat note.txt\n验证结果：退出码：0\n",
    }

    def next_tool(tools, body):
        call = base_next_tool(tools, body)
        if call is not None:
            return call
        index = len(fixture.actions) - 5
        if index < 0 or index >= len(files):
            return None
        plan_dir, = (workspace / "shiju-plans").glob("task-*")
        name, content = list(files.items())[index]
        tool, arguments = tool_inputs(tools, 0, workspace, outside, "vibe")
        path_key = next(key for key, value in arguments.items() if value == str(workspace / "note.txt"))
        content_key = next(key for key, value in arguments.items() if value == CONTENTS[0])
        arguments.update({path_key: str(plan_dir / name), content_key: content})
        tool_id = f"toolu_background_plan_{index}"
        fixture.actions.append(tool)
        fixture.issued.add(tool_id)
        return {"id": tool_id, "type": "function",
                "function": {"name": tool, "arguments": json.dumps(arguments)}}

    fixture.next_tool = next_tool
    return 5 + len(files)


def verify(binary, session_model_runtime):
    source = "session_model" if session_model_runtime else "cli_local"
    entry = next(entry for entry in json.loads(MANIFEST.read_text())["entries"]
                 if entry["key"] == "vibe")
    with tempfile.TemporaryDirectory(prefix="shiju-acp-background-") as name:
        root = Path(name).resolve()
        env = environment(root)
        workspace = root / "workspace"
        workspace.mkdir()
        outside = root / "outside.txt"
        (workspace / "keep.txt").write_text("keep")
        with model_fixture(workspace, outside, "vibe") as fixture:
            fixture.reset("background")
            expected_calls = add_plan_artifacts(fixture, workspace, outside)
            spec = locate(entry, env)
            if session_model_runtime:
                # The gateway verifies the artifact against its checked-in runtime contract.
                installed = Path(env["VIBEWINDOW_CONFIG_DIR"]) / "acp-runtime"
                installed.mkdir()
                (installed / f"vibe-{entry['distribution']['release']}-model-source-v1").symlink_to(
                    session_model_runtime.resolve(strict=True), target_is_directory=True
                )
            else:
                native = root / "native"
                native.mkdir(mode=0o700)
                native_env = isolated_environment(entry, native, fixture, "cli-local")
                spec["env"].update(native_env)
                env.update(native_env)
            env["ACP_APPROVAL_FIXTURE_KEY"] = fixture.key
            config = {
                "workspace_dir": str(workspace),
                "agents": {"acp-vibe": {"enabled": True,
                                        "runtime": {"kind": "acp", "backend": "vibe"}}},
                "acp": {"vibe": spec},
                "providers": {"fixture": {
                    "name": "Approval fixture", "api": fixture.url + "/v1",
                    "adapter": "openai", "env": ["ACP_APPROVAL_FIXTURE_KEY"],
                    "models": {fixture.models[0]: {"status": "active", "temperature": True, "tool_call": True,
                                                  "limit": {"context": 100000, "output": 1024}}},
                }},
            }
            with gateway(binary, root, env) as url:
                capabilities = json.loads(request(url, "/acp-model-source/vibe"))
                require(capabilities["acp_model_source"] == source, "unexpected plan model source")
                request(url, "/global/config", config, method="PATCH")
                stream = request(url, "/task-plan-run/stream", {
                    "root": str(workspace), "query": "Run the isolated approval fixture",
                    "agent": "acp-vibe", "acp_agent": "vibe",
                    "model": "fixture/" + fixture.models[0],
                })
                query = "?" + urlencode({"directory": str(workspace)})
                pending = json.loads(request(url, "/permission" + query))
            events = [json.loads(line[6:]) for line in stream.splitlines() if line.startswith("data: {")]
            result = {
                "backend": "vibe", "release": entry["distribution"]["release"],
                "source": source, "actions": len(fixture.actions),
                "model_tool_results": len(fixture.results),
                "workspace_edit": (workspace / "note.txt").is_file()
                and (workspace / "note.txt").read_text() == CONTENTS[1],
                "outside_write": outside.exists(),
                "terminal_marker": (workspace / "terminal.txt").exists(),
                "deletion": not (workspace / "keep.txt").is_file(),
                "pending_approvals": len(pending),
                "events": sorted({event["type"] for event in events}),
                "terminal_event": events[-1] if events else None,
            }
            print(json.dumps(result, ensure_ascii=False, indent=2))
            require(result["workspace_edit"], "ordinary workspace edits did not complete")
            require(not any(result[key] for key in ("outside_write", "terminal_marker", "deletion")),
                    "a sensitive operation escaped approval")
            require(result["actions"] == expected_calls
                    and result["model_tool_results"] == expected_calls and not pending,
                    "background ACP did not finish all probes without pending approvals")
            require("plan_created" in result["events"] and events[-1]["type"] == "finished"
                    and DONE in stream, "plan run did not complete")


if __name__ == "__main__":
    signal.signal(signal.SIGTERM, interrupt)
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--gateway", required=True, type=Path)
    parser.add_argument("--session-model-runtime", type=Path,
                        help="verified Vibe 3.0.4 runtime directory for the session-model route")
    args = parser.parse_args()
    slot = Path(cargo_slot(os.environ))
    binary = args.gateway.resolve()
    require(binary == slot / "debug/shiju", "gateway must come from the owned Cargo slot")
    verify(binary, args.session_model_runtime)
