#!/usr/bin/env python3
"""Run skill-specific ignored entries through the real production ACP request path."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import platform
import re
import tempfile
import subprocess

from acp_skill_mount_fixture import LIFECYCLE, SkillMountFixture
from verify_acp_model_source import ROOT, Unavailable, cargo_slot, locate, isolated_environment, execute_cargo

AGENTS = tuple("codex claude qwen gemini codebuddy goose vibe hermes snow copilot kilocode kimi kiro opencode openclaw pi auggie cursor droid qoder trae iflow".split())


def select_entry(manifest, agent):
    entries = [entry for entry in manifest["entries"] if entry.get("key") == agent]
    if agent not in AGENTS or len(entries) != 1:
        raise ValueError("manifest must identify exactly one known agent")
    return entries[0]


def accepted(evidence, flow, tests, exit_code):
    return (exit_code == 0 and tests == 1 and evidence.get("errors") == 0
            and evidence.get("model_requests", 0) > 0 and evidence.get("tool_requests", 0) >= 3
            and evidence.get("tool_results", 0) >= 3 and evidence.get("discovery_events", 0) > 0
            and evidence.get("first_prompt_clean") is True and evidence.get("attachment_read") is True
            and evidence.get("script_executed") is True
            and all(flow.get(key) is True for key in LIFECYCLE))


def run(args):
    mode = getattr(args, "mode", "session-model")
    report = {"agent": args.agent, "status": "unverified", "passed": False,
              "platform": platform.system() + "-" + platform.machine(),
              "mount_source_mode": "session-directory", "model_source": mode, "effective_tests": 0, "version": None, "sha256": None,
              "model_requests": 0, "tool_requests": 0, "tool_results": 0,
              "lifecycle": {key: False for key in LIFECYCLE}}
    try:
        slot = cargo_slot(os.environ)
        report["cargo_target_dir"] = slot
        entry = select_entry(json.loads(args.manifest.read_text()), args.agent)
        report["version"] = entry["version"]
        # Reuse only fixed identity/configuration recipes, never old verdicts or evidence.
        identity = select_entry(json.loads((ROOT / "docs/shiju/acp-model-source/cli-versions.json").read_text()), args.agent)
        if identity["distribution"].get("release") != entry["version"]:
            raise Unavailable("skill and launch manifests pin different adapter releases")
        if args.agent == "goose":
            # The release tag has v; the fixed native --version prints 1.57.0.
            identity["distribution"]["release"] = entry["version"].removeprefix("v")
        with tempfile.TemporaryDirectory(prefix="acp-skill-mount-") as directory:
            root = Path(directory)
            root.chmod(0o700)
            env = {key: os.environ[key] for key in ("PATH", "HOME", "USERPROFILE", "TMPDIR", "SystemRoot") if key in os.environ}
            env.update(CARGO_TARGET_DIR=slot, CARGO_BUILD_BUILD_DIR=slot,
                       RUST_MIN_STACK="33554432", VIBEWINDOW_CONFIG_DIR=str(root / "config"))
            with SkillMountFixture(root) as fixture:
                fixture.kilocode_native = args.agent == "kilocode"
                fixture.copilot_native = args.agent == "copilot"
                fixture.snow_native = args.agent == "snow"
                fixture.hermes_native = args.agent == "hermes"
                fixture.vibe_native = args.agent == "vibe"
                fixture.goose_native = args.agent == "goose"
                fixture.claude_native = args.agent == "claude"
                fixture.qwen_native = args.agent == "qwen"
                fixture.gemini_native = args.agent == "gemini"
                fixture.codebuddy_native = args.agent == "codebuddy"
                native = root / "native"
                native.mkdir(mode=0o700)
                if args.agent in ("qwen", "gemini", "codebuddy", "goose", "vibe", "hermes", "snow", "copilot", "kilocode"):
                    fixture.forbidden_skill = "unselected-" + fixture.name
                    for base in (root / (".github" if args.agent == "copilot" else ".kilo" if args.agent == "kilocode" else "." + args.agent) / "skills", native / (".gemini/skills" if args.agent == "gemini" else "config/skills" if args.agent == "goose" else "kilo/skills" if args.agent == "kilocode" else "skills")):
                        decoy = base / fixture.forbidden_skill
                        decoy.mkdir(parents=True)
                        (decoy / "SKILL.md").write_text(
                            "---\nname: " + fixture.forbidden_skill + "\ndescription: Must remain isolated\n---\nUNSELECTED_BODY\n")
                isolation = isolated_environment(identity, native, fixture, mode)
                if args.agent == "goose":
                    # Keep native background session titles out of the skill protocol fixture.
                    isolation["GOOSE_DISABLE_SESSION_NAMING"] = "true"
                if args.agent == "kilocode":
                    for key in ("XDG_DATA_HOME", "XDG_STATE_HOME", "XDG_CACHE_HOME"):
                        directory = native / key
                        directory.mkdir(mode=0o700)
                        isolation[key] = str(directory)
                    isolation.update(KILO_DISABLE_MODELS_FETCH="1", KILO_DISABLE_AUTOUPDATE="1")
                env.update(isolation)
                if args.agent == "codebuddy":
                    env.update(CODEBUDDY_REPL_ENABLED="0", CODEBUDDY_DEFER_TOOL_LOADING="false")
                    isolation.update(CODEBUDDY_REPL_ENABLED="0", CODEBUDDY_DEFER_TOOL_LOADING="false")
                if args.agent == "codex":
                    from acp_skill_mount_codex import locate_codex
                    config = locate_codex(identity, env)
                    report["engine_version"] = identity["underlying"]["release"]
                else:
                    config = locate(identity, env)
                if args.agent in ("claude", "qwen", "gemini", "codebuddy", "goose", "vibe", "hermes", "snow", "copilot", "kilocode") and mode == "cli-local":
                    config["env"] = isolation
                if args.agent == "goose":
                    config["env"]["GOOSE_DISABLE_SESSION_NAMING"] = "true"
                if args.agent == "codebuddy":
                    plugin = root / "unselected-plugin"
                    (plugin / ".codebuddy-plugin").mkdir(parents=True)
                    (plugin / ".codebuddy-plugin/plugin.json").write_text(json.dumps({"name": "unselected-plugin", "version": "1.0.0", "skills": "./skills/"}))
                    decoy = plugin / "skills" / fixture.forbidden_skill
                    decoy.mkdir(parents=True)
                    (decoy / "SKILL.md").write_text("---\nname: " + fixture.forbidden_skill + "\ndescription: Must remain isolated\n---\nUNSELECTED_PLUGIN_BODY\n")
                    if mode == "cli-local":
                        config["args"] += ["--plugin-dir", str(plugin)]
                if args.agent == "kilocode":
                    config["args"] = ["acp", "--pure"]
                binary = Path(config["command"]).resolve()
                digest = hashlib.sha256(binary.read_bytes()).hexdigest()
                known = {row.get("sha256") for row in entry.get("evidence", [])}
                report.update(version=entry["version"], sha256=digest)
                if digest not in known:
                    raise Unavailable("installed CLI hash is absent from reviewed skill artifacts")
                runtime = root / "entry.json"
                runtime.write_text(json.dumps({"agent": args.agent, "config": config,
                    "root": str(root), "fixture": fixture.url, "key": fixture.key,
                    "skill": fixture.name, "prompt": fixture.prompt, "mode": mode}))
                runtime.chmod(0o600)
                flow_file = root / "flow.json"
                env.update(VW_ACP_SKILL_ACCEPTANCE_MANIFEST=str(runtime),
                           VW_ACP_SKILL_ACCEPTANCE_FLOW=str(flow_file))
                command = ["cargo", "test", "-p", "vw-agent", "--lib", "acp_skill_mount_real_" + args.agent,
                           "--", "--ignored", "--nocapture", "--test-threads=1"]
                result = execute_cargo(command, env=env)
                log = result.stdout.decode(errors="replace")
                diagnostics = [line.split("SKILL_FAILURE: ", 1)[1] for line in result.stderr.decode(errors="replace").splitlines() if "SKILL_FAILURE: " in line]
                stderr_lines = result.stderr.decode(errors="replace").splitlines()
                build_errors = [re.sub(r'https?://[^\s"]+|/[^\s"]+|[A-Za-z0-9_=-]{24,}', '[redacted]', line)[:400]
                                for line in stderr_lines if line.startswith("error")][:8]
                for index, line in enumerate(stderr_lines):
                    if "panicked at" in line and index + 1 < len(stderr_lines):
                        diagnostics.append(re.sub(r'https?://[^\s"]+|/[^\s"]+|[A-Za-z0-9_=-]{24,}', '[redacted]', stderr_lines[index + 1])[:400])
                tests = 1 if re.search(r"running 1 test\b", log) else 0
                one_passed = bool(re.search(r"test result: ok\. 1 passed; 0 failed; 0 ignored;", log))
                flow = json.loads(flow_file.read_text()) if flow_file.exists() else {}
                evidence = fixture.snapshot()
                passed = (one_passed and accepted(evidence, flow, tests, result.returncode)
                          and (args.agent not in ("claude", "qwen", "gemini", "codebuddy", "goose", "vibe", "hermes", "snow", "copilot", "kilocode") or evidence.get("native_skill_invocations", 0) > 0))
                report.update(evidence, passed=passed, status="passed" if passed else "failed",
                              reason="all skill assertions passed" if passed else "production skill lifecycle or observations incomplete",
                              effective_tests=tests, cargo_exit_code=result.returncode, command=command, diagnostics=diagnostics, build_errors=build_errors,
                              lifecycle={key: flow.get(key) is True for key in LIFECYCLE})
    except Unavailable as error:
        report["reason"] = str(error)
    except (OSError, ValueError, KeyError, TypeError, subprocess.TimeoutExpired, InterruptedError):
        report.update(status="failed", reason="invalid acceptance input or execution; raw supplier logs withheld")
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(report, ensure_ascii=False, indent=2) + "\n")
    return 0 if report["passed"] else 1


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--agent", required=True)
    parser.add_argument("--mode", choices=("session-model", "cli-local"), default="session-model")
    parser.add_argument("--manifest", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    return run(parser.parse_args())


if __name__ == "__main__":
    raise SystemExit(main())
