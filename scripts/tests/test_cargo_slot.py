import importlib.util
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch

SPEC = importlib.util.spec_from_file_location('cargo_slot', Path(__file__).parents[1] / 'cargo_slot.py')
slots = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(slots)


class CargoSlotTests(unittest.TestCase):
    def test_nested_commands_reuse_owned_cache_and_release_on_failure(self):
        with tempfile.TemporaryDirectory() as folder, patch.dict(os.environ, {}, clear=True):
            root = Path(folder).resolve()
            locks = root / 'target/rust-slot-locks'
            locks.mkdir(parents=True)
            (locks / 'slot-1.lock').mkdir()
            with self.assertRaisesRegex(RuntimeError, 'command failed'):
                with slots.slot(root) as env:
                    expected = str(root / 'target/rust-slot-2')
                    self.assertEqual(env['CARGO_TARGET_DIR'], expected)
                    self.assertEqual(env['CARGO_BUILD_BUILD_DIR'], expected)
                    with patch.dict(os.environ, env, clear=True), slots.slot(root) as nested:
                        self.assertEqual(nested, env)
                    self.assertTrue((locks / 'slot-2.lock/owner.json').is_file())
                    raise RuntimeError('command failed')
            self.assertFalse((locks / 'slot-2.lock').exists())
            self.assertTrue((locks / 'slot-1.lock').is_dir())
            with slots.slot(root) as again:
                self.assertEqual(again['CARGO_TARGET_DIR'], expected)

    def test_manual_task_slot_is_reused_only_by_owner_process(self):
        with tempfile.TemporaryDirectory() as folder, patch.dict(os.environ, {}, clear=True):
            root = Path(folder).resolve()
            lock = root / 'target/rust-slot-locks/slot-4.lock'
            lock.mkdir(parents=True)
            (lock / 'owner').write_text(f'plan=voice PID={os.getpid()}')
            with patch.dict(os.environ, {'CARGO_TARGET_DIR': str(root / 'target/rust-slot-4')}):
                with slots.slot(root) as env:
                    self.assertEqual(env['CARGO_BUILD_BUILD_DIR'], env['CARGO_TARGET_DIR'])
                self.assertTrue(lock.exists())
                (lock / 'owner').write_text('pid=1')
                with self.assertRaisesRegex(RuntimeError, 'not owned'):
                    with slots.slot(root):
                        self.fail('joined another owner')

    def test_wrong_owner_cannot_join_slot(self):
        with tempfile.TemporaryDirectory() as folder, patch.dict(os.environ, {}, clear=True):
            root = Path(folder).resolve()
            with slots.slot(root) as env:
                env['SHIJU_CARGO_SLOT_TOKEN'] = 'wrong'
                with patch.dict(os.environ, env, clear=True), self.assertRaisesRegex(RuntimeError, 'owner'):
                    with slots.slot(root):
                        self.fail('joined wrong owner')

    def test_real_child_receives_paths_and_exit_status(self):
        with tempfile.TemporaryDirectory() as folder, patch.dict(os.environ, {}, clear=True):
            root = Path(folder).resolve()
            code = 'import os,json; open("env.json","w").write(json.dumps(dict(os.environ))); exit(7)'
            self.assertEqual(slots.run([sys.executable, '-c', code], root), 7)
            env = json.loads((root / 'env.json').read_text())
            self.assertEqual(env['CARGO_TARGET_DIR'], env['CARGO_BUILD_BUILD_DIR'])
            self.assertFalse(list((root / 'target/rust-slot-locks').iterdir()))

    def test_exited_leader_does_not_leave_descendant_using_released_slot(self):
        with tempfile.TemporaryDirectory() as folder, patch.dict(os.environ, {}, clear=True):
            root = Path(folder).resolve()
            code = ('import subprocess,sys,pathlib; '
                    'p=subprocess.Popen([sys.executable,"-c","import time; time.sleep(60)"]); '
                    'pathlib.Path("descendant.pid").write_text(str(p.pid))')
            self.assertEqual(slots.run([sys.executable, '-c', code], root), 0)
            pid = int((root / 'descendant.pid').read_text())
            status = subprocess.run(['ps', '-o', 'stat=', '-p', str(pid)], capture_output=True, text=True)
            self.assertTrue(not status.stdout.strip() or status.stdout.strip().startswith('Z'))
            self.assertFalse(list((root / 'target/rust-slot-locks').iterdir()))

    def test_permission_denied_during_exit_waits_until_group_disappears(self):
        with patch.object(slots.os, 'killpg', side_effect=PermissionError), \
                patch.object(slots, 'group_alive', side_effect=[True, False]), \
                patch.object(slots.time, 'sleep'):
            slots.signal_group(123, slots.signal.SIGKILL)

    def test_persistent_permission_denied_for_live_group_is_reported(self):
        with patch.object(slots.os, 'killpg', side_effect=PermissionError), \
                patch.object(slots, 'group_alive', return_value=True), \
                patch.object(slots.time, 'sleep'), self.assertRaises(PermissionError):
            slots.signal_group(123, slots.signal.SIGKILL)

    def test_sigterm_releases_lock_after_child_ends(self):
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder).resolve()
            script = ('import importlib.util,sys; from pathlib import Path; '
                      f's=importlib.util.spec_from_file_location("slots",{str(Path(slots.__file__))!r}); '
                      'm=importlib.util.module_from_spec(s); s.loader.exec_module(m); '
                      'sys.exit(m.run([sys.executable,"-c",'
                      '"import pathlib,time; pathlib.Path(\'ready\').touch(); time.sleep(60)"],Path(sys.argv[1])))')
            child = subprocess.Popen([sys.executable, '-c', script, str(root)])
            try:
                import time
                deadline = time.monotonic() + 10
                while not (root / 'ready').exists() and time.monotonic() < deadline:
                    time.sleep(.02)
                self.assertTrue((root / 'ready').exists())
                child.terminate()
                child.wait(timeout=15)
                self.assertFalse(list((root / 'target/rust-slot-locks').iterdir()))
            finally:
                if child.poll() is None:
                    child.kill()
                    child.wait()
