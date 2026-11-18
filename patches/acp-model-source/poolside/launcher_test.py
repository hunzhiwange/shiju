import unittest
from pathlib import Path
from tempfile import TemporaryDirectory

from launcher import StagedWrites


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


if __name__ == "__main__":
    unittest.main()
