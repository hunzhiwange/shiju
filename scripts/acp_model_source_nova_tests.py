"""Pinned Nova model binding: exercise patched supplier code against a local peer."""
import pathlib, shutil, subprocess, tempfile, unittest
ROOT = pathlib.Path(__file__).resolve().parents[1]
class NovaPatchTests(unittest.TestCase):
    def test_rejects_unreviewed_supplier_without_mutation(self):
        with tempfile.TemporaryDirectory() as temp:
            root = pathlib.Path(temp); (root / 'dist').mkdir()
            entry = root / 'dist/cli.js'; entry.write_text('unreviewed')
            p = subprocess.run(['python3', str(ROOT / 'patches/acp-model-source/nova/apply.py'), str(root)], capture_output=True)
            self.assertNotEqual(p.returncode, 0)
            self.assertIn(b'Unreviewed Nova', p.stderr)
            self.assertEqual(entry.read_text(), 'unreviewed')
    def test_binding_rejects_remote_endpoint_and_exposes_only_selected_model(self):
        import json
        hook = ROOT / 'patches/acp-model-source/nova/session-model.mjs'
        with tempfile.TemporaryDirectory() as temp:
            binding = pathlib.Path(temp) / 'model.json'
            binding.write_text(json.dumps({'model': 'host-model', 'endpoint': 'https://remote.invalid/'})); binding.chmod(0o600)
            import os
            env = dict(os.environ, SHIJU_NOVA_MODEL_CONFIG=str(binding), SHIJU_NOVA_RELAY_TOKEN='fixture')
            cmd = ['node', '--input-type=module', '-e', f"import {{model}} from {json.dumps(hook.as_uri())}; console.log(JSON.stringify(model()))"]
            p = subprocess.run(cmd, env=env, capture_output=True)
            self.assertNotEqual(p.returncode, 0)
            binding.write_text(json.dumps({'model': 'host-model', 'endpoint': 'http://127.0.0.1:12345/'}))
            p = subprocess.run(cmd, env=env, capture_output=True)
            self.assertEqual(p.returncode, 0, p.stderr)
            self.assertEqual(json.loads(p.stdout)['id'], 'host-model')
            binding.chmod(0o644)
            self.assertNotEqual(subprocess.run(cmd, env=env, capture_output=True).returncode, 0)
    def test_state_isolation_rejects_symlink_and_public_directory(self):
        import os, json
        hook = ROOT / 'patches/acp-model-source/nova/session-model.mjs'
        with tempfile.TemporaryDirectory() as temp:
            root = pathlib.Path(temp); state = root / 'state'; state.mkdir(mode=0o700)
            env = dict(os.environ, SHIJU_NOVA_STATE_DIRECTORY=str(state))
            cmd = ['node', '--input-type=module', '-e', f"import {json.dumps(hook.as_uri())}"]
            self.assertEqual(subprocess.run(cmd, env=env, capture_output=True).returncode, 0)
            state.chmod(0o755)
            self.assertNotEqual(subprocess.run(cmd, env=env, capture_output=True).returncode, 0)
            state.chmod(0o700); link = root / 'link'; link.symlink_to(state)
            env['SHIJU_NOVA_STATE_DIRECTORY'] = str(link)
            self.assertNotEqual(subprocess.run(cmd, env=env, capture_output=True).returncode, 0)
    def test_project_runtime_stays_outside_workspace(self):
        import os, json
        hook = ROOT / 'patches/acp-model-source/nova/session-model.mjs'
        with tempfile.TemporaryDirectory() as temp:
            root = pathlib.Path(temp); state = root / 'state'; state.mkdir(mode=0o700)
            workspace = root / 'workspace'; workspace.mkdir()
            env = dict(os.environ, SHIJU_NOVA_STATE_DIRECTORY=str(state))
            cmd = ['node', '--input-type=module', '-e', f"import {{projectDirectory}} from {json.dumps(hook.as_uri())}; console.log(projectDirectory({json.dumps(str(workspace))}));"]
            p = subprocess.run(cmd, env=env, capture_output=True, text=True)
            self.assertEqual(p.returncode, 0, p.stderr)
            output = pathlib.Path(p.stdout.strip())
            self.assertEqual(output.parent, state)
            self.assertEqual(list(workspace.iterdir()), [])
            self.assertEqual(subprocess.run(cmd, env=env, capture_output=True).returncode, 0)
            output.rmdir(); output.symlink_to(workspace)
            self.assertNotEqual(subprocess.run(cmd, env=env, capture_output=True).returncode, 0)
if __name__ == '__main__': unittest.main()
