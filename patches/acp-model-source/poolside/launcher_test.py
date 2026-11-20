import unittest
from pathlib import Path
from tempfile import TemporaryDirectory

from launcher import StagedWrites, project_skills


class StagedWritesTest(unittest.TestCase):
    def test_only_completed_text_diffs_inside_workspace_are_published(self):
        with TemporaryDirectory() as workspace:
            root = Path(workspace).resolve()
            staged = StagedWrites(root)
            def call(id_, path, status, text="content"):
                staged.observe(dict(
                    sessionUpdate="tool_call", toolCallId=id_, status="pending",
                    content=[dict(type="diff", path=str(path), newText=text)],
                ))
                staged.observe(dict(
                    sessionUpdate="tool_call_update", toolCallId=id_, status=status,
                ))
            call("allowed", root / "allowed.txt", "completed")
            call("denied", root / "denied.txt", "failed")
            call("outside", root.parent / "outside.txt", "completed")
            self.assertEqual(staged.take(), {str(root / "allowed.txt"): "content"})
            self.assertEqual(staged.take(), {})

    def test_later_edits_replace_earlier_content_and_symlinks_are_rejected(self):
        with TemporaryDirectory() as workspace:
            root = Path(workspace).resolve()
            staged = StagedWrites(root)
            (root / "link").symlink_to(root.parent)
            for index, (path, text) in enumerate([
                (root / "file", "before"), (root / "file", ""), (root / "link/escape", "escape")
            ]):
                staged.observe(dict(
                    sessionUpdate="tool_call", toolCallId=str(index), status="completed",
                    content=[dict(type="diff", path=str(path), newText=text)],
                ))
            self.assertEqual(staged.take(), {str(root / "file"): ""})


class SkillProjectionTest(unittest.TestCase):
    def test_projects_only_selected_files_without_following_nested_links(self):
        with TemporaryDirectory() as directory:
            root = Path(directory).resolve()
            lease = root / "shiju-session-skills-fixture"
            lease.mkdir(mode=0o700)
            view = lease / "skills"
            view.mkdir()
            for name in ("selected", "other"):
                skill = root / name
                skill.mkdir()
                (skill / "SKILL.md").write_text(name)
            (view / "selected").symlink_to(root / "selected")
            destination = root / "projected"
            project_skills(str(view), destination)
            self.assertEqual(list(destination.iterdir()), [destination / "selected"])
            self.assertEqual((destination / "selected/SKILL.md").read_text(), "selected")
            (root / "selected/escape").symlink_to(root / "other/SKILL.md")
            with self.assertRaises(OSError):
                project_skills(str(view), root / "rejected")

    def test_empty_selection_needs_no_private_mapping(self):
        with TemporaryDirectory() as directory:
            destination = Path(directory) / "projected"
            project_skills("", destination)
            self.assertFalse(destination.exists())


if __name__ == "__main__":
    unittest.main()
