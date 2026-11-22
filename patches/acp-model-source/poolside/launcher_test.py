import unittest
from pathlib import Path
from tempfile import TemporaryDirectory
from types import SimpleNamespace
from unittest.mock import Mock, patch

from launcher import StagedWrites, bridge, project_skills


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


class BridgeReviewTest(unittest.TestCase):
    def test_staged_write_uses_host_review_and_discards_native_copy(self):
        with TemporaryDirectory() as workspace:
            root = Path(workspace).resolve()
            path = str(root / "result.txt")
            for allowed in (True, False):
                with self.subTest(allowed=allowed):
                    review = dict(sessionId="session", toolCall={"toolCallId": "poolside/sandbox_review"},
                                  options=[{"optionId": "sandbox-discard"}])
                    updates = [
                        ("host", dict(id=1, method="session/prompt", params=dict(sessionId="session"))),
                        ("pool", dict(method="session/update", params=dict(sessionId="session", update={
                            "sessionUpdate": "tool_call", "toolCallId": "write", "status": "completed",
                            "content": [dict(type="diff", path=path, newText="staged")],
                        }))),
                        ("pool", dict(id=2, method="session/request_permission", params=review)),
                        ("host", dict(id="shiju-poolside-write-1", **(
                            {"result": {}} if allowed else {"error": {"code": -32000}}
                        ))),
                        ("pool", dict(id=1, result={"stopReason": "end_turn"})),
                    ]
                    process = SimpleNamespace(stdin=object())
                    with patch("launcher.frames", return_value=iter(updates)), patch("launcher.send") as output:
                        bridge(process, root, Mock(http=None))
                    sent = [call.args for call in output.call_args_list]
                    writes = [frame for _, frame in sent if frame.get("method") == "fs/write_text_file"]
                    self.assertEqual([write["params"] for write in writes], [
                        dict(sessionId="session", path=path, content="staged"),
                    ])
                    native_replies = [frame for stream, frame in sent
                                      if stream is process.stdin and frame.get("id") == 2]
                    self.assertEqual([reply["result"]["outcome"]["optionId"] for reply in native_replies],
                                     ["sandbox-discard"])
                    self.assertIn("result" if allowed else "error", output.call_args.args[1])
                    self.assertFalse(Path(path).exists())


if __name__ == "__main__":
    unittest.main()
