"""Integrity checks use synthetic material; no benchmark answers are embedded."""
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

from suite.catalog import build_catalog, validate_ids


class CatalogTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.task = self.root / 'tasks/example'
        for name, content in {
            'instruction.md': '<!-- canary synthetic -->\n\nExample\n',
            'task.toml': '[metadata]\ncategory="debugging"\n[agent]\ntimeout_sec=17\n[verifier]\ntimeout_sec=23\n[environment]\ncpus=2\nmemory_mb=4096\ndocker_image="example:1"\n',
            'environment/Dockerfile': 'FROM scratch\n',
            'environment/data.bin': 'fixture',
            'tests/test.sh': 'true\n',
            'solution/solve.sh': 'true\n',
        }.items():
            path = self.task / name
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text(content)
        self.git('init', '-q')
        self.git('add', '.')
        self.git('-c', 'user.name=Fixture', '-c', 'user.email=fixture@example.invalid', 'commit', '-qm', 'fixture')
        self.revision = self.git('rev-parse', 'HEAD').strip()

    def git(self, *args):
        return subprocess.check_output(['git', '-C', str(self.root), *args], text=True)

    def test_dataset_root_metadata_is_verified_without_becoming_tasks(self):
        metadata = self.root / 'tasks/dataset.toml'
        metadata.write_text('name="fixture"\n')
        self.git('add', '.')
        self.git('-c', 'user.name=Fixture', '-c', 'user.email=fixture@example.invalid', 'commit', '-qm', 'metadata')
        revision = self.git('rev-parse', 'HEAD').strip()
        catalog = build_catalog(self.root, revision)
        self.assertEqual(catalog['task_count'], 1)
        self.assertIn('dataset.toml', catalog['root_files'])
        metadata.write_text('name="changed"\n')
        with self.assertRaises(ValueError):
            build_catalog(self.root, revision)

    def test_cli_requires_all_arguments(self):
        result = subprocess.run([sys.executable, "-m", "suite.catalog"], capture_output=True)
        self.assertEqual(result.returncode, 2)

    def test_mode_tampering_fails(self):
        (self.task / "tests/test.sh").chmod(0o755)
        with self.assertRaises(ValueError):
            build_catalog(self.root, self.revision)

    def test_real_metadata_and_determinism(self):
        catalog = build_catalog(self.root, self.revision)
        self.assertEqual(catalog, build_catalog(self.root, self.revision))
        task = catalog['tasks'][0]
        self.assertEqual(task['agent']['timeout_sec'], 17)
        self.assertEqual(task['verifier']['timeout_sec'], 23)
        self.assertNotEqual(task['instruction_sha256'], task['effective_instruction_sha256'])
        self.assertEqual(len(task['files']), 6)

    def test_missing_solution_fails(self):
        (self.task / "solution/solve.sh").unlink()
        with self.assertRaises(ValueError):
            build_catalog(self.root, self.revision)

    def test_symlink_substitution_fails(self):
        path = self.task / "environment/data.bin"
        path.unlink()
        path.symlink_to("../instruction.md")
        with self.assertRaises(ValueError):
            build_catalog(self.root, self.revision)

    def test_missing_tests_fails(self):
        (self.task / 'tests/test.sh').unlink()
        with self.assertRaises(ValueError):
            build_catalog(self.root, self.revision)

    def test_missing_data_fails(self):
        (self.task / 'environment/data.bin').unlink()
        with self.assertRaises(ValueError):
            build_catalog(self.root, self.revision)

    def test_wrong_revision_fails(self):
        with self.assertRaises(ValueError):
            build_catalog(self.root, '0' * 40)

    def test_instruction_or_config_tampering_fails(self):
        for name in ('instruction.md', 'task.toml'):
            path = self.task / name
            original = path.read_bytes()
            path.write_bytes(original + b'\n# changed\n')
            with self.assertRaises(ValueError):
                build_catalog(self.root, self.revision)
            path.write_bytes(original)

    def test_duplicate_ids_fail(self):
        with self.assertRaises(ValueError):
            validate_ids(['example', 'example'])

    def test_extra_file_fails(self):
        (self.task / 'extra').write_text('unexpected')
        with self.assertRaises(ValueError):
            build_catalog(self.root, self.revision)
