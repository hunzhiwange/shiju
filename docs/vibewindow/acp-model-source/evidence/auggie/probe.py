#!/usr/bin/env python3
"""Reproduce Auggie's upstream gates; this is not model-source acceptance."""
import argparse
import hashlib
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
import json
import os
from pathlib import Path
import platform
import signal
import subprocess
import tempfile
import threading

ROOT = Path(__file__).resolve().parents[5]
PACKAGES = {
    "stable": ("0.36.0", "augment.mjs", "6229e1685a5c4f9305ad333d76cd1a05c4315e6a9c8af3bbc75170b524005306"),
    "preview": ("1.2.0-prerelease.202611031510", "auggie-v2.mjs", "e2369eb798e79bd85f4de65ee0eea841e6eb22bf6df6de679734947bd6a7cb83"),
}


def execute(args, env, cwd, stdin=""):
    process = subprocess.Popen(args, env=env, cwd=cwd, stdin=subprocess.PIPE,
                               stdout=subprocess.PIPE, stderr=subprocess.PIPE,
                               text=True, start_new_session=True)
    try:
        stdout, stderr = process.communicate(stdin, timeout=25)
        return process.returncode, stdout + stderr
    finally:
        # Own all children even on timeout or a normal CLI exit.
        try:
            os.killpg(process.pid, signal.SIGKILL)
        except ProcessLookupError:
            pass
        process.wait()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--stable-dir", type=Path, default=ROOT / "target/auggie-inspection/package")
    parser.add_argument("--preview-dir", type=Path, default=ROOT / "target/auggie-inspection/preview/package")
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    if os.name != "posix":
        parser.error("child process containment is verified only on POSIX")
    requests = []

    class Handler(BaseHTTPRequestHandler):
        def do_POST(self):
            requests.append({"method": "POST", "path": self.path})
            self.send_error(500, "No model response in an upstream-gate probe")

        def do_GET(self):
            requests.append({"method": "GET", "path": self.path})
            self.send_error(500, "No model response in an upstream-gate probe")

        def log_message(self, *_):
            pass

    server = ThreadingHTTPServer(("127.0.0.1", 0), Handler)
    thread = threading.Thread(target=server.serve_forever, daemon=True)
    thread.start()
    endpoint = f"http://127.0.0.1:{server.server_port}/v1"
    report = {"evidence_kind": "real-cli-upstream-gate", "platform": platform.system(),
              "architecture": platform.machine(), "model_source_verified": False, "cases": []}
    try:
        for channel, package in (("stable", args.stable_dir), ("preview", args.preview_dir)):
            version, entry, digest = PACKAGES[channel]
            package = package.resolve()
            metadata = json.loads((package / "package.json").read_text())
            assert metadata["name"] == "@augmentcode/auggie" and metadata["version"] == version
            cli = package / entry
            assert hashlib.sha256(cli.read_bytes()).hexdigest() == digest, "CLI differs from pinned package"
            with tempfile.TemporaryDirectory(prefix="auggie-upstream-gate-") as raw:
                root = Path(raw)
                cache = root / "cache"
                cache.mkdir(mode=0o700)
                env = {key: os.environ[key] for key in ("PATH", "HOME", "TMPDIR") if key in os.environ}
                env.update(AUGMENT_DISABLE_AUTO_UPDATE="1", PI_OFFLINE="1", PROBE_MARKER=str(root / "marker.json"))
                code, output = execute(["node", str(cli), "--augment-cache-dir", str(cache), "--version"], env, root)
                assert code == 0 and version in output
                extension = root / "provider.mjs"
                extension.write_text("import fs from 'node:fs';\nexport default function(pi) {\n"
                                     "fs.writeFileSync(process.env.PROBE_MARKER, JSON.stringify({registerProvider: typeof pi.registerProvider}));\n"
                                     "pi.registerProvider('shiju-test', " + json.dumps({
                                         "baseUrl": endpoint, "apiKey": "offline-probe-only", "api": "openai-completions",
                                         "models": [{"id": "probe-model", "name": "probe-model", "reasoning": False,
                                                     "input": ["text"], "cost": {"input": 0, "output": 0, "cacheRead": 0, "cacheWrite": 0},
                                                     "contextWindow": 32000, "maxTokens": 1024}]}) + ");\n}\n")
                base = ["node", str(cli), "--augment-cache-dir", str(cache)]
                if channel == "stable":
                    cases = [("acp-extension", ["--acp", "--extension", str(extension)])]
                else:
                    base += ["--offline", "--no-cloud", "--no-extensions"]
                    cases = [("acp-extension", ["--extension", str(extension), "--model", "shiju-test/probe-model", "--acp"]),
                             ("print-extension", ["--extension", str(extension), "--model", "shiju-test/probe-model", "--print", "Return ok"])]
                for name, tail in cases:
                    before = len(requests)
                    code, output = execute(base + tail, env, root, json.dumps({
                        "jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {
                            "protocolVersion": 1, "clientCapabilities": {},
                            "clientInfo": {"name": "shiju-offline-gate", "version": "1"}}}) + "\n")
                    error = "unknown option '--extension'" if channel == "stable" else "Extension provider registration is not supported"
                    assert code == 1 and error in output, "Upstream behavior changed; review before enabling driver"
                    marker = root / "marker.json"
                    loaded = json.loads(marker.read_text()) if marker.exists() else None
                    assert loaded == (None if channel == "stable" else {"registerProvider": "function"})
                    assert len(requests) == before
                    report["cases"].append({"version": version, "case": name, "exit_code": code,
                                            "extension_api": loaded, "error": error, "fixture_requests": 0})
                if channel == "preview":
                    # Probe Pi's documented file form as an alternative, not an Auggie contract.
                    provider = {"providers": {"shiju-test": {"baseUrl": endpoint, "apiKey": "offline-probe-only",
                                "api": "openai-completions", "models": [{"id": "probe-model", "name": "probe-model",
                                "reasoning": False, "input": ["text"], "contextWindow": 32000, "maxTokens": 1024,
                                "cost": {"input": 0, "output": 0, "cacheRead": 0, "cacheWrite": 0}}]}}}
                    for name in ("models.json", "agent/models.json"):
                        path = cache / name
                        path.parent.mkdir(exist_ok=True)
                        path.write_text(json.dumps(provider))
                        path.chmod(0o600)
                    env["PI_CODING_AGENT_DIR"] = str(cache)
                    code, output = execute(base + ["--model", "shiju-test/probe-model", "--print", "Return ok"], env, root)
                    assert code == 1 and "No models available" in output and not requests
                    report["cases"].append({"version": version, "case": "pi-models-json-alternative",
                                            "exit_code": code, "error": "No models available", "fixture_requests": 0})
        report["fixture_requests"] = requests
        report["probe_completed"] = True
        args.output.parent.mkdir(parents=True, exist_ok=True)
        args.output.write_text(json.dumps(report, indent=2) + "\n")
        print(f"Upstream gate reproduced: {len(report['cases'])} cases; no model-source acceptance claimed")
    finally:
        server.shutdown()
        server.server_close()
        thread.join()


if __name__ == "__main__":
    main()
