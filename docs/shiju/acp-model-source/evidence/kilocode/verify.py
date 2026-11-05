#!/usr/bin/env python3
"""Kilocode XDG isolation recipe for the task-10 production acceptance runner.

Only fixture setup is extended: the real native binary and production Rust ACP
entry, lifecycle assertions and HTTP observations remain the shared runner's.
"""
import copy
import io
import json
import threading
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[5]
sys.path.insert(0, str(ROOT / "scripts"))
import verify_acp_model_source as runner

native_environment = runner.isolated_environment


class KilocodeFixture(runner.ModelFixture):
    """Retain only a title-helper boolean, never native model request payloads."""
    def __init__(self):
        super().__init__()
        context = threading.local()

        class Records(list):
            def __deepcopy__(self, memo):
                return copy.deepcopy(list(self), memo)

            def append(self, row):
                row["auxiliary"] = getattr(context, "title", False)
                super().append(row)

        self.records = Records()
        original = self.server.RequestHandlerClass

        class Handler(original):
            def do_POST(self):
                body = self.rfile.read(int(self.headers.get("Content-Length", "0")))
                try:
                    messages = json.loads(body).get("messages", [])
                    context.title = not json.loads(body).get("tools") and all(
                        any(message.get("role") == role and
                            "title" in json.dumps(message.get("content")).lower()
                            for message in messages)
                        for role in ("system", "user"))
                except (ValueError, AttributeError, TypeError):
                    context.title = False
                stream = self.rfile
                self.rfile = io.BytesIO(body)
                try:
                    super().do_POST()
                finally:
                    self.rfile = stream
                    context.title = False

        self.server.RequestHandlerClass = Handler


successful = runner.successful


def successful_with_helper(evidence, flow):
    flow["requires_auxiliary"] = True
    return successful(evidence, flow)


runner.ModelFixture = KilocodeFixture
runner.successful = successful_with_helper


def isolated_environment(entry, directory, fixture, mode):
    env = native_environment(entry, directory, fixture, mode)
    # Kilo independently resolves credentials/history, configuration and caches.
    # A config-only root would leave auth.json and the database in the real HOME.
    for key in ("XDG_DATA_HOME", "XDG_STATE_HOME", "XDG_CACHE_HOME"):
        path = directory / key
        path.mkdir(mode=0o700)
        env[key] = str(path)
    return env


execute_cargo = runner.execute_cargo


def offline_cargo(command, *, env, timeout=1800):
    env = dict(env, KILO_DISABLE_MODELS_FETCH="1", KILO_DISABLE_AUTOUPDATE="1")
    return execute_cargo(command, env=env, timeout=timeout)


runner.execute_cargo = offline_cargo
runner.isolated_environment = isolated_environment
if __name__ == "__main__":
    raise SystemExit(runner.main())
