#!/usr/bin/env python3
"""Original acceptance with Hermes' external model-service discovery surface.

Only the model fixture is adapted: two real files, and unsupported local-server
fingerprint GETs return 404. POST routing, authentication and all production
acceptance predicates remain unchanged. No synthetic ACP agent or tool results.
"""
import argparse
import io
import json
import time
import os
from pathlib import Path
import sys
from urllib.parse import urlsplit

ROOT = Path(__file__).resolve().parents[5]
sys.path.insert(0, str(ROOT / "scripts"))
import verify_acp_model_source as acceptance

DISCOVERY = {"/api/v1/models", "/api/tags", "/v1/props", "/props", "/version"}
OBSERVATIONS = {"unsupported_discovery_gets": 0, "generation_probes": []}
START = time.monotonic()


class HermesFixture(acceptance.ModelFixture):
    def __init__(self):
        super().__init__()
        original = self.server.RequestHandlerClass

        class Handler(original):
            def do_GET(self):
                # Hermes fingerprints loopback services before loading model
                # metadata. A 404 is the correct unsupported-protocol response,
                # not an unauthenticated model generation or tool result.
                if urlsplit(self.path).path in DISCOVERY:
                    OBSERVATIONS["unsupported_discovery_gets"] += 1
                    return self.reply(404, {})
                return super().do_GET()

            def do_POST(self):
                if urlsplit(self.path).path == "/v1/chat/completions":
                    length = int(self.headers.get("Content-Length", "0"))
                    if 0 < length <= 2_000_000:
                        data = self.rfile.read(length)
                        self.rfile = io.BytesIO(data)
                        try:
                            body = json.loads(data)
                            messages = body.get("messages", [])
                            OBSERVATIONS["generation_probes"].append({
                                "seconds": round(time.monotonic() - START, 3),
                                "cancel": "CANCEL_PROBE" in json.dumps(next((m for m in reversed(messages) if m.get("role") == "user"), {})),
                                "tool_messages": sum(m.get("role") == "tool" for m in messages),
                            })
                        except (ValueError, TypeError, KeyError):
                            pass
                return super().do_POST()

        self.server.RequestHandlerClass = Handler

    def __setattr__(self, name, value):
        super().__setattr__(name, value)
        if name == "read_path" and Path(value).is_absolute():
            first = Path(value)
            assert first.parent.name.startswith("acp-model-acceptance-")
            second = first.with_name("challenge-second.txt")
            with second.open("x") as output:
                output.write(self.challenge)
            second.chmod(0o600)
            self.second_read_path = str(second)


if __name__ == "__main__":
    parser = argparse.ArgumentParser(add_help=False)
    parser.add_argument("--test-binary", type=Path)
    options, remaining = parser.parse_known_args()
    sys.argv = [sys.argv[0], *remaining]
    binary_command = []
    if options.test_binary is not None:
        binary = options.test_binary.resolve()
        slot = Path(acceptance.cargo_slot(os.environ))
        if (binary.parent != slot / "debug/deps" or not binary.name.startswith("vw_agent-")
                or not os.access(binary, os.X_OK)):
            raise SystemExit("cached test binary must belong to the currently held Cargo slot")
        original_execute = acceptance.execute_cargo

        def execute_binary(command, *, env, timeout=300):
            assert command[:6] == ["cargo", "test", "-p", "vw-agent", "--lib", "--all-features"]
            assert command[6] in acceptance.TESTS.values() and command[7] == "--"
            binary_command[:] = [str(binary), command[6], *command[8:]]
            return original_execute(binary_command, env=env, timeout=timeout)

        acceptance.execute_cargo = execute_binary
    acceptance.ModelFixture = HermesFixture
    result = acceptance.main()
    output = Path(sys.argv[sys.argv.index("--output") + 1])
    report = json.loads(output.read_text())
    if options.test_binary is not None:
        report["evidence_kind"] = "cached-production-binary"
        if "cargo_exit_code" in report:
            report["binary_exit_code"] = report.pop("cargo_exit_code")
        report["command"] = binary_command
    report["fixture_adaptation"] = {"two_independent_files": True, **OBSERVATIONS}
    output.write_text(json.dumps(report, ensure_ascii=False, indent=2) + "\n")
    raise SystemExit(result)
