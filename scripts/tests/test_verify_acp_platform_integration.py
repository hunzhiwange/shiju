"""Regression checks for the Linux ACP integration evidence."""
import sys
from pathlib import Path
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import verify_acp_integration as integration


class PlatformIntegrationTests(unittest.TestCase):
    def test_rejection_cannot_reach_external_service(self):
        with self.assertRaisesRegex(AssertionError, "attached MCP audit"):
            integration.verify_platform_audit((0, 0), (1, 1), mounted=True, rejected=True)
        integration.verify_platform_audit((0, 0), (1, 0), mounted=True, rejected=True)

    def test_removal_must_clear_catalog_and_leave_service_unused(self):
        with self.assertRaisesRegex(AssertionError, "removed MCP"):
            integration.verify_platform_catalog(
                {integration.ATTACHED_MCP, integration.NATIVE_MCP}, mounted=False
            )
        with self.assertRaisesRegex(AssertionError, "attached MCP audit"):
            integration.verify_platform_audit((1, 1), (2, 2), mounted=False)
        integration.verify_platform_catalog({integration.NATIVE_MCP}, mounted=False)
        integration.verify_platform_audit((1, 1), (2, 1), mounted=False)

    def test_continuation_requires_current_call_id(self):
        messages = [
            {"role": "tool", "tool_call_id": "old", "content": "previous"},
            {"role": "user", "content": "current"},
            {"role": "assistant", "tool_calls": [{"id": "current-call"}]},
            {"role": "tool", "tool_call_id": "old", "content": "wrong turn"},
        ]
        with self.assertRaisesRegex(AssertionError, "continuation"):
            integration.platform_receipts(messages, "current", ["current-call"])
        messages[-1]["tool_call_id"] = "current-call"
        self.assertEqual(
            integration.platform_receipts(messages, "current", ["current-call"]),
            [messages[-1]],
        )
        with self.assertRaisesRegex(AssertionError, "continuation"):
            integration.platform_receipts(messages + [messages[-1]], "current", ["current-call"])


if __name__ == "__main__":
    unittest.main()
