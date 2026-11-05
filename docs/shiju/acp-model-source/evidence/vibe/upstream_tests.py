#!/usr/bin/env python3
"""Exercise the installed upstream configuration and ACP parser, never mock them."""
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
import tomli_w


def binding(endpoint="http://127.0.0.1:1/v1"):
    model = {"name": "selected-model", "alias": "session-selected", "provider": "session-provider"}
    return {"active_model": "session-selected", "allowed_models": ["selected-model"],
            "providers": [{"name": "session-provider", "api_base": endpoint,
                           "api_key_env_var": "VW_VIBE_RELAY_TOKEN", "api_style": "openai"}],
            "models": [model], "compaction_model": model,
            "utility_models": {"title": "session-selected", "smart_approve": "session-selected"}}


class InstalledUpstream(unittest.TestCase):
    def run_config(self, source, *, bound=True, endpoint="http://127.0.0.1:1/v1", permissions=0o600, token=True, reference=None):
        with tempfile.TemporaryDirectory(prefix="vibe-binding-test-") as root:
            home = Path(root, "native")
            home.mkdir(mode=0o700)
            path = home / "session-model.toml"
            path.write_text(tomli_w.dumps(binding(endpoint)))
            path.chmod(permissions)
            (home / "config.toml").write_text(tomli_w.dumps(binding()))
            env = {key: os.environ[key] for key in ("PATH", "HOME", "TMPDIR") if key in os.environ}
            env.update(VIBE_HOME=str(home), TMPDIR=root, VW_VIBE_RELAY_TOKEN="offline-only")
            if reference is not None:
                env["VW_ACP_LAUNCH_REFERENCE"] = reference
            if not token:
                env.pop("VW_VIBE_RELAY_TOKEN")
            if bound:
                env["_SHIJU_VIBE_SESSION_MODEL_CONFIG"] = str(path)
            prelude = '''import asyncio
from vibe.core.config.harness_files import init_harness_files_manager
from vibe.core.config.default_orchestrator import build_default_orchestrator
from vibe.core.agents.models import LEAN
from vibe.core.agents.registry import apply_profile_overrides
init_harness_files_manager("user", "project")
async def main():
    orchestrator = await build_default_orchestrator()
'''
            code = prelude + "\n".join("    " + line for line in source.splitlines()) + "\nasyncio.run(main())\n"
            return subprocess.run([sys.executable, "-c", code], env=env, text=True,
                                  capture_output=True, timeout=30)

    def test_profile_cannot_replace_primary_compaction_or_utility_routes(self):
        result = self.run_config('''apply_profile_overrides(orchestrator, LEAN.overrides, profile_name=LEAN.name)
c = orchestrator.config
assert c.get_active_model().name == "selected-model"
assert c.get_compaction_model().name == "selected-model"
assert c.vision_model is None
assert c.utility_models.title == "session-selected"
assert c.utility_models.smart_approve == "session-selected"
assert c.get_provider_for_model(c.get_active_model()).api_base == "http://127.0.0.1:1/v1"
assert len(c.providers) == 1
assert len(c.models) == 1''')
        self.assertEqual(result.returncode, 0, result.stderr[-1500:])

    def test_cli_local_retains_original_profile_behavior(self):
        result = self.run_config('''apply_profile_overrides(orchestrator, LEAN.overrides, profile_name=LEAN.name)
assert orchestrator.config.get_active_model().name == "labs-leanstral-1-5"''', bound=False)
        self.assertEqual(result.returncode, 0, result.stderr[-1500:])

    def test_organization_model_policy_is_not_weakened(self):
        result = self.run_config('''from vibe.core.config.layers.admin import admin_layer
admin_layer(orchestrator).load_managed_toml('active_model = "organization-model"')
try:
    await orchestrator.reload_layer("admin")
except ValueError as error:
    assert "organization model policy conflicts" in str(error)
else:
    raise AssertionError("organization policy bypassed")''')
        self.assertEqual(result.returncode, 0, result.stderr[-1500:])

    def test_remote_binding_is_rejected(self):
        result = self.run_config('pass', endpoint="https://example.invalid/v1")
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("requires loopback relay", result.stderr)

    def test_insecure_binding_permissions_are_rejected(self):
        result = self.run_config('pass', permissions=0o644)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("insecure private session model binding", result.stderr)

    def test_missing_relay_token_cannot_use_native_account(self):
        result = self.run_config('pass', token=False)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("session model relay credential is missing", result.stderr)

    def test_history_directory_is_stable_across_private_homes(self):
        result = self.run_config('''import os, shutil
from pathlib import Path
first_home = Path(os.environ["VIBE_HOME"])
history = Path(orchestrator.config.session_logging.save_dir)
assert not history.is_relative_to(first_home)
assert history.parent.name == "a" * 64
assert history.name == "sessions"
assert history.stat().st_mode & 0o777 == 0o700
(history / "checkpoint").write_text("history-without-credentials")
second_home = Path(os.environ["TMPDIR"]) / "second-native"
second_home.mkdir(mode=0o700)
for name in ("config.toml", "session-model.toml"):
    shutil.copy2(first_home / name, second_home / name)
os.environ["VIBE_HOME"] = str(second_home)
os.environ["_SHIJU_VIBE_SESSION_MODEL_CONFIG"] = str(second_home / "session-model.toml")
orchestrator.rebuild()
assert Path(orchestrator.config.session_logging.save_dir) == history
assert (history / "checkpoint").read_text() == "history-without-credentials"''', reference="vw-session-model:" + "a" * 64)
        self.assertEqual(result.returncode, 0, result.stderr[-1500:])


if __name__ == "__main__":
    unittest.main()
