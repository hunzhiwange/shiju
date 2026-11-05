#!/usr/bin/env python3
"""Run the unchanged production harness with Gemini's private fixed-version recipe."""
import argparse
import json
import os
from pathlib import Path
import sys

ROOT = Path(__file__).resolve().parents[5]
sys.path.insert(0, str(ROOT / "scripts"))
import verify_acp_model_source as harness

parser = argparse.ArgumentParser()
parser.add_argument("--mode", choices=("session-model", "cli-local"), required=True)
parser.add_argument("--output", type=Path, required=True)
parser.add_argument("--cached-binary", type=Path)
args = parser.parse_args()
actual = []
execute_original = harness.execute_cargo

def diagnosed(command, *, env, timeout=1800):
    env = dict(env)
    env["VW_ACP_ACCEPTANCE_DIAGNOSTICS"] = str(args.output.resolve().with_suffix(".diagnostics.json"))
    return execute_original(command, env=env, timeout=timeout)

harness.execute_cargo = diagnosed
if args.cached_binary:
    slot = Path(harness.cargo_slot(os.environ))
    binary = args.cached_binary.resolve()
    if slot not in binary.parents or not binary.is_file() or not os.access(binary, os.X_OK):
        raise SystemExit("binary must belong to the currently held Cargo slot")
    sources = [ROOT / "crates/vw-agent/src/session/llm/acp" / path for path in (
        "request.rs", "model_source/acceptance_tests.rs", "model_source/drivers/gemini.rs",
        "model_source/drivers/gemini_tests.rs")]
    if any(source.stat().st_mtime > binary.stat().st_mtime for source in sources):
        raise SystemExit("cached binary predates Gemini implementation or production acceptance")
    execute = harness.execute_cargo

    def cached(command, *, env, timeout=300):
        actual[:] = [str(binary), command[6], *command[8:]]
        return execute(actual, env=env, timeout=timeout)

    harness.execute_cargo = cached
os.environ["PATH"] = str(ROOT / "target/acp-gemini-runtime/bin") + os.pathsep + os.environ["PATH"]
result = harness.run(argparse.Namespace(agent="gemini", mode=args.mode,
    manifest=Path(__file__).with_name("manifest.json"), output=args.output, offline_fixture=True))
if args.cached_binary:
    report = json.loads(args.output.read_text())
    report["evidence_kind"] = "cached-production-binary"
    if "cargo_exit_code" in report:
        report["binary_exit_code"] = report.pop("cargo_exit_code")
    report["command"] = actual
    args.output.write_text(json.dumps(report, ensure_ascii=False, indent=2) + "\n")
raise SystemExit(result)
