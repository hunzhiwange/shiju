#!/usr/bin/env python3
"""Probe the pinned official CLI, never replace it with a simulated ACP agent."""
import hashlib
import json
import os
from pathlib import Path
import select
import subprocess
import sys
import tempfile
import time

ROOT = Path(__file__).resolve().parents[5]
sys.path.insert(0, str(ROOT / "scripts"))
from acp_model_source_fixture import ModelFixture

EXPECTED = "d4dd02d40f18f3a784699fc37ae0c55d676a4704c37a3b91ab96541b0db673a1"


def rpc(child, identifier, method, params):
    child.stdin.write((json.dumps({"jsonrpc": "2.0", "id": identifier,
                                  "method": method, "params": params}) + "\n").encode())
    child.stdin.flush()
    deadline = time.monotonic() + 30
    while time.monotonic() < deadline:
        if not select.select([child.stdout], [], [], 1)[0]:
            continue
        line = child.stdout.readline()
        if not line:
            raise RuntimeError("CLI exited before ACP response")
        reply = json.loads(line)
        if reply.get("id") == identifier:
            return reply
    raise RuntimeError("ACP response timed out")


def run(binary):
    with binary.open("rb") as file:
        digest = hashlib.sha256()
        for chunk in iter(lambda: file.read(1024 * 1024), b""):
            digest.update(chunk)
    if digest.hexdigest() != EXPECTED:
        raise RuntimeError("pinned Darwin-arm64 official binary digest mismatch")
    report = {"version": "2.5.0", "platform": "Darwin-arm64", "sha256": EXPECTED}
    for scenario in ("missing-credential", "unknown-model"):
        with tempfile.TemporaryDirectory(prefix="kimi-boundary-") as directory, ModelFixture() as fixture:
            root = Path(directory)
            default = "unknown" if scenario == "unknown-model" else "selected"
            config = ('default_model = "' + default + '"\ntelemetry = false\n'
                      '[providers.fixture]\ntype = "openai"\nbase_url = ' + json.dumps(fixture.url + '/v1') + '\n'
                      'api_key_env = "VW_KIMI_MISSING_TOKEN"\n'
                      '[models.selected]\nprovider = "fixture"\nmodel = "acp-fixture-primary"\n'
                      'max_context_size = 128000\ncapabilities = ["tool_use"]\n')
            (root / "config.toml").write_text(config)
            os.chmod(root / "config.toml", 0o600)
            env = {key: os.environ[key] for key in ("PATH", "HOME", "TMPDIR") if key in os.environ}
            env.update(KIMI_CODE_HOME=directory, KIMI_DISABLE_TELEMETRY="1", KIMI_CODE_NO_AUTO_UPDATE="1",
                       OPENAI_API_KEY="offline-native-account-must-not-be-used",
                       KIMI_API_KEY="offline-native-account-must-not-be-used")
            if scenario == "unknown-model":
                env["VW_KIMI_MISSING_TOKEN"] = fixture.key
            child = subprocess.Popen([str(binary), "acp"], stdin=subprocess.PIPE, stdout=subprocess.PIPE,
                                     stderr=subprocess.DEVNULL, env=env, bufsize=0)
            try:
                initialized = rpc(child, 1, "initialize", {"protocolVersion": 1, "clientCapabilities": {}})
                assert "result" in initialized
                result = rpc(child, 2, "session/new", {"cwd": directory, "mcpServers": []})
                error = result.get("error", {})
                message = json.dumps(error)
                assert error, "invalid model configuration unexpectedly created a session"
                if scenario == "missing-credential":
                    assert error["code"] == -32000 and "Authentication required" in message
                else:
                    assert error["code"] in (-32000, -32602, -32603)
                observed = fixture.snapshot()
                assert not observed["requests"] and observed["errors"] == 0
                report[scenario] = {"rejected": True, "error_code": error["code"], "requests": 0}
            finally:
                if child.poll() is None:
                    child.terminate()
                child.wait(timeout=10)
                child.stdin.close()
                child.stdout.close()
    return report


if __name__ == "__main__":
    print(json.dumps(run(Path(sys.argv[1]).resolve()), indent=2))
