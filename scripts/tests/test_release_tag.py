"""Check release tagging against the remote default branch without GitHub writes."""

from pathlib import Path
import subprocess
import tempfile
import unittest

SCRIPT = Path(__file__).resolve().parents[1] / 'release_tag.sh'


class ReleaseTagTests(unittest.TestCase):
    def git(self, cwd, *args):
        return subprocess.run(['git', '-C', str(cwd), *args], check=True,
                              capture_output=True, text=True).stdout.strip()

    def fixture(self, directory):
        root = Path(directory)
        bare, local = root / 'remote.git', root / 'checkout'
        subprocess.run(['git', 'init', '--bare', '--initial-branch=shiju', str(bare)],
                       check=True, capture_output=True)
        subprocess.run(['git', 'clone', str(bare), str(local)], check=True, capture_output=True)
        self.git(local, 'config', 'user.name', 'Test')
        self.git(local, 'config', 'user.email', 'test@example.test')
        (local / 'README').write_text('fixture')
        self.git(local, 'add', 'README')
        self.git(local, 'commit', '-m', 'initial')
        self.git(local, 'push', 'origin', 'HEAD')
        return local

    def test_tags_remote_default_branch_without_main(self):
        with tempfile.TemporaryDirectory() as directory:
            local = self.fixture(directory)
            result = subprocess.run(['bash', str(SCRIPT), 'v0.5.7'], cwd=local,
                                    capture_output=True, text=True)
            self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
            self.assertEqual(self.git(local, 'cat-file', '-t', 'v0.5.7'), 'tag')
            self.assertEqual(self.git(local, 'rev-parse', 'v0.5.7^{}'),
                             self.git(local, 'rev-parse', 'HEAD'))
            self.assertEqual(self.git(local, 'ls-remote', 'origin', 'refs/tags/v0.5.7'), '')

    def test_rejects_unpushed_commit(self):
        with tempfile.TemporaryDirectory() as directory:
            local = self.fixture(directory)
            self.git(local, 'commit', '--allow-empty', '-m', 'unpushed')
            result = subprocess.run(['bash', str(SCRIPT), 'v0.5.7'], cwd=local,
                                    capture_output=True, text=True)
            self.assertNotEqual(result.returncode, 0)
            self.assertIn('origin/shiju', result.stderr)
            self.assertEqual(self.git(local, 'tag', '--list'), '')


if __name__ == '__main__':
    unittest.main()
