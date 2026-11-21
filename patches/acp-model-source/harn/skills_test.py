"""Check that Harn stages only bounded workspace skill manifests."""
from pathlib import Path
import runpy
import shutil
import tempfile
import unittest


ROOT = Path(__file__).resolve().parent


class SkillSnapshot(unittest.TestCase):
    def setUp(self):
        private = tempfile.TemporaryDirectory()
        self.addCleanup(private.cleanup)
        self.root = Path(private.name)
        shutil.copyfile(ROOT / 'launcher.py', self.root / 'launcher.py')
        shutil.copyfile(ROOT.parent / 'cortex-code/mcp_bridge.py', self.root / 'shiju_stdio.py')
        self.snapshot = runpy.run_path(str(self.root / 'launcher.py'))['snapshot_native_skills']
        self.workspace = self.root / 'workspace'
        self.native = self.workspace / '.harn/skills'
        self.native.mkdir(parents=True)
        self.output = self.root / 'native'

    def test_manifests_and_detached_signatures_only(self):
        bundle = self.native / 'project'
        bundle.mkdir()
        (bundle / 'SKILL.md').write_text('manifest')
        (bundle / 'SKILL.md.sig').write_text('signature')
        (bundle / 'attachment').write_text('attachment')
        self.snapshot(self.workspace, self.output)
        self.assertEqual(sorted(p.name for p in (self.output / 'project').iterdir()),
                         ['SKILL.md', 'SKILL.md.sig'])

    def test_skips_linked_bundle_and_refuses_linked_manifest(self):
        external = self.root / 'external'
        external.mkdir()
        manifest = external / 'SKILL.md'
        manifest.write_text('outside')
        (self.native / 'linked').symlink_to(external, target_is_directory=True)
        self.snapshot(self.workspace, self.output)
        self.assertFalse(list(self.output.iterdir()))
        self.output.rmdir()
        bundle = self.native / 'project'
        bundle.mkdir()
        (bundle / 'SKILL.md').symlink_to(manifest)
        with self.assertRaises(OSError):
            self.snapshot(self.workspace, self.output)


if __name__ == '__main__':
    unittest.main()
