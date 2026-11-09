import unittest
from pathlib import Path

from acp_approval_model import CONTENTS, matches_workspace_edit, shell_command, tool_inputs


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


if __name__ == "__main__":
    unittest.main()
