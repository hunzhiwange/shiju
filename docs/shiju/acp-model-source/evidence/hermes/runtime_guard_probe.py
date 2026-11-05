#!/usr/bin/env python3
"""Real CLI must reject a resolved route conflicting with its private binding."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import selectors
import subprocess
import sys
import tempfile
import time

ROOT = Path(__file__).resolve().parents[5]
sys.path.insert(0, str(ROOT / "scripts"))
from acp_model_source_fixture import ModelFixture


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--cli", required=True, type=Path)
    args = parser.parse_args()
    with tempfile.TemporaryDirectory(prefix="hermes-binding-probe-") as directory, ModelFixture() as fixture:
        root = Path(directory)
        route = {"provider": "custom", "default": fixture.models[0],
                 "base_url": fixture.url + "/v1", "api_key": fixture.key,
                 "api_mode": "chat_completions", "context_length": 131072}
        config = root / "config.yaml"
        config.write_text(json.dumps({"model": route, "fallback_providers": []}))
        config.chmod(0o600)
        (root / ".env").write_text("")
        env = {k: os.environ[k] for k in ("PATH", "HOME") if k in os.environ}
        env.update(HERMES_HOME=str(root), TMPDIR=str(root),
                   CUSTOM_BASE_URL="http://127.0.0.1:1/v1",
                   VW_ACP_LAUNCH_REFERENCE="vw-session-model:" + hashlib.sha256(str(root).encode()).hexdigest())
        process = subprocess.Popen([str(args.cli.absolute()), "acp", "--session-model-binding"],
                                   env=env, cwd=root, stdin=subprocess.PIPE,
                                   stdout=subprocess.PIPE, stderr=subprocess.DEVNULL, text=True)
        selector = selectors.DefaultSelector()
        selector.register(process.stdout, selectors.EVENT_READ)

        def rpc(identifier, method, params):
            process.stdin.write(json.dumps({"jsonrpc": "2.0", "id": identifier,
                                            "method": method, "params": params}) + "\n")
            process.stdin.flush()
            end = time.monotonic() + 30
            while time.monotonic() < end:
                if not selector.select(1):
                    if process.poll() is not None:
                        raise AssertionError("real CLI exited before response")
                    continue
                line = process.stdout.readline()
                if not line:
                    raise AssertionError("real CLI closed its protocol stream")
                result = json.loads(line)
                if result.get("id") == identifier and "method" not in result:
                    return result
            raise AssertionError("real CLI response timed out")

        try:
            initialized = rpc(1, "initialize", {"protocolVersion": 1, "clientCapabilities": {}})
            assert initialized["result"]["agentInfo"]["version"] == "0.19.0"
            created = rpc(2, "session/new", {"cwd": str(root), "mcpServers": []})
            assert "error" in created, "conflicting resolved endpoint was accepted"
            assert not fixture.snapshot()["requests"]
            print(json.dumps({"version": "0.19.0", "conflicting_route_rejected": True,
                              "generation_requests": 0}))
        finally:
            process.terminate()
            try:
                process.wait(timeout=5)
            except subprocess.TimeoutExpired:
                process.kill()
                process.wait()
            selector.close()


if __name__ == "__main__":
    main()
