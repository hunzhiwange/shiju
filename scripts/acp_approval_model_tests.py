import unittest
import json
from pathlib import Path
from tempfile import TemporaryDirectory
from urllib.request import Request, urlopen

from acp_approval_model import CONTENTS, matches_workspace_edit, model_fixture, shell_command, tool_inputs


class ApprovalModelTests(unittest.TestCase):
    def setUp(self):
        self.workspace = Path("/fixture/workspace")
        self.outside = Path("/fixture/outside.txt")
        self.tools = [
            {"name": "Write", "input_schema": {"properties": {"file_path": {}, "content": {}}}},
            {"name": "Bash", "input_schema": {"properties": {"command": {}, "description": {}}}},
        ]

    def test_uses_advertised_file_tool_for_exact_workspace_edit(self):
        name, arguments = tool_inputs(self.tools, 1, self.workspace, self.outside)
        self.assertEqual(name, "Write")
        self.assertEqual(arguments["content"], CONTENTS[1])
        self.assertTrue(matches_workspace_edit(arguments, self.workspace))

    def test_outside_and_sensitive_tools_never_match_edit_approval(self):
        for step in (2, 3, 4):
            _, arguments = tool_inputs(self.tools, step, self.workspace, self.outside)
            self.assertFalse(matches_workspace_edit(arguments, self.workspace))
        command = shell_command(1, self.workspace, self.outside)
        self.assertTrue(matches_workspace_edit({"command": command}, self.workspace))
        self.assertFalse(matches_workspace_edit({"command": command + " && true"}, self.workspace))

    def test_missing_tool_schema_is_not_treated_as_an_executable_tool(self):
        self.assertIsNone(tool_inputs([{"name": "write_file"}], 0, self.workspace, self.outside))

    def test_pinned_native_shell_names_use_their_declared_cwd(self):
        for name in ("execute_bash", "developer__shell"):
            tools = [{"name": name, "parameters": {"properties": {"command": {}, "cwd": {}}}}]
            chosen, arguments = tool_inputs(tools, 1, self.workspace, self.outside)
            self.assertEqual(chosen, name)
            self.assertEqual(arguments["cwd"], str(self.workspace))
            self.assertTrue(matches_workspace_edit(arguments, self.workspace))

    def test_kiro_control_plane_reuses_the_loopback_fixture(self):
        with model_fixture(self.workspace, self.outside) as fixture:
            fixture.kiro_controls = True
            request = Request(fixture.url, b"{}", {
                "Authorization": "Bearer " + fixture.key,
                "x-amz-target": "KiroControlPlaneBearerService.ListAvailableModels",
            })
            with urlopen(request, timeout=2) as response:
                result = json.load(response)
            self.assertEqual(result["defaultModel"]["modelId"], fixture.models[0])

    def test_qwen_reads_existing_file_before_the_second_write(self):
        with TemporaryDirectory() as name:
            workspace = Path(name)
            note = workspace / "note.txt"
            note.write_text(CONTENTS[0])
            tools = self.tools + [{"name": "read_file"}]
            with model_fixture(workspace, self.outside, "qwen") as fixture:
                fixture.reset("approval_1", 1)
                read = fixture.next_tool(tools, {})
                self.assertEqual(read["function"]["name"], "read_file")
                self.assertEqual(json.loads(read["function"]["arguments"]), {"file_path": str(note)})
                self.assertEqual(fixture.actions, [])
                write = fixture.next_tool(tools, {})
                self.assertEqual(write["function"]["name"], "Write")
                self.assertEqual(fixture.actions, ["Write"])


if __name__ == "__main__":
    unittest.main()
