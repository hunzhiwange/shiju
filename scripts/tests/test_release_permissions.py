"""Verify probe cleanup and failure handling without GitHub writes."""

import os
from pathlib import Path
import subprocess
import tempfile
import unittest

SCRIPT = Path(__file__).resolve().parents[1] / 'check_release_permissions.sh'


class ReleasePermissionsTests(unittest.TestCase):
    def run_probe(self, failure):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            gh = root / 'gh'
            gh.write_text('''#!/bin/bash
set -eu
printf '%s\\n' "$*" >> "$CALL_LOG"
case "$*" in
  'api --paginate '*)
    [[ "$FAILURE" != list ]] || exit 1
    if [[ "$FAILURE" == existing ]]; then echo v0.5.6; fi ;;
  *git/ref/tags/*) [[ "$FAILURE" != tag ]] ;;
  'api --method POST '*)
    [[ "$FAILURE" != create ]] || exit 1
    echo 123 ;;
  'release upload '*) [[ "$FAILURE" != upload ]] ;;
  'api --method DELETE '*) [[ "$FAILURE" != cleanup ]] ;;
  *) exit 99 ;;
esac
''')
            gh.chmod(0o755)
            log = root / 'calls'
            env = dict(os.environ, PATH=f"{root}:{os.environ['PATH']}",
                       GH_REPO='owner/repo', FAILURE=failure, CALL_LOG=str(log))
            result = subprocess.run(['bash', str(SCRIPT), 'v0.5.6'], env=env,
                                    capture_output=True, text=True)
            return result, log.read_text()

    def test_success_uploads_and_deletes_only_created_release(self):
        result, calls = self.run_probe('none')
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn('release upload v0.5.6', calls)
        self.assertIn('api --method DELETE repos/owner/repo/releases/123', calls)
        self.assertNotIn('target_commitish', calls)

    def test_upload_failure_still_cleans_up(self):
        result, calls = self.run_probe('upload')
        self.assertNotEqual(result.returncode, 0)
        self.assertIn('api --method DELETE repos/owner/repo/releases/123', calls)

    def test_missing_tag_or_create_failure_never_deletes_existing_release(self):
        for failure in ('tag', 'create', 'list', 'existing'):
            with self.subTest(failure=failure):
                result, calls = self.run_probe(failure)
                self.assertNotEqual(result.returncode, 0)
                self.assertNotIn('DELETE', calls)
                self.assertNotIn('release upload', calls)

    def test_cleanup_failure_is_reported(self):
        result, _ = self.run_probe('cleanup')
        self.assertNotEqual(result.returncode, 0)
        self.assertIn('remove it manually', result.stderr)


if __name__ == '__main__':
    unittest.main()
