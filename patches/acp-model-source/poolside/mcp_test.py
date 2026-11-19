import unittest
from pathlib import Path
from tempfile import TemporaryDirectory

from mcp_bridge import Mounts


class ApprovalTest(unittest.TestCase):
    def test_approval_is_argument_bound_single_use_and_revoked(self):
        with TemporaryDirectory() as tmp, Mounts(Path(tmp), Path(tmp) / "settings.yaml") as mounts:
            mounts.catalogs["graph"] = {"query": {}}
            mounts.begin("session")
            update = {"sessionUpdate": "tool_call", "toolCallId": "call",
                      "title": "graph__query", "rawInput": {"limit": 1}}
            mounts.observe("session", update)
            permission = {"sessionId": "session", "toolCall": {
                "toolCallId": "call", "title": "mcp:graph:query"},
                "options": [{"kind": "allow_once", "optionId": "allow-once"},
                            {"kind": "allow_always", "optionId": "always"}]}
            self.assertIsNone(mounts.claim("graph", "query", {"limit": 1}))
            self.assertTrue(mounts.permission(1, permission))
            self.assertEqual([o["optionId"] for o in permission["options"]], ["allow-once"])
            mounts.decide(1, {"outcome": {"outcome": "selected", "optionId": "allow-once"}})
            self.assertIsNone(mounts.claim("graph", "query", {"limit": True}))
            self.assertIsNotNone(mounts.claim("graph", "query", {"limit": 1}))
            self.assertIsNone(mounts.claim("graph", "query", {"limit": 1}))
            mounts.observe("session", update)
            self.assertTrue(mounts.permission(2, permission))
            mounts.end()
            mounts.decide(2, {"outcome": {"outcome": "selected", "optionId": "allow-once"}})
            mounts.begin("another-session")
            self.assertIsNone(mounts.claim("graph", "query", {"limit": 1}))
