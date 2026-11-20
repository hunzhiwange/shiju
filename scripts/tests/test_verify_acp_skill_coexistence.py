import sys
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from acp_skill_coexistence import correlated_result, verify_turn
from acp_skill_coexistence.copilot import MCP_FUNCTION, Probe


class CoexistenceTests(unittest.TestCase):
    def test_rejects_missing_or_uncorrelated_results(self):
        for results in ([], [{"tool_use_id": "different", "content": "secret"}],
                        [{"tool_use_id": "expected", "content": ""}],
                        [{"tool_use_id": "expected", "content": "secret"}] * 2):
            with self.subTest(results=results), self.assertRaises(AssertionError):
                correlated_result(results, "expected", "secret")

    def test_accepts_only_matching_status(self):
        result = {"tool_use_id": "expected", "is_error": True, "content": "denied"}
        self.assertEqual(correlated_result([result], "expected", rejected=True), result)
        with self.assertRaises(AssertionError):
            correlated_result([result], "expected", "denied")

    def test_rejects_zero_downstream_calls(self):
        evidence = {"calls": {kind: str(i) for i, kind in enumerate(
            ("native", "native_read", "selected", "selected_read", "mcp"))},
            "model_requests": 6, "done": 1, "errors": 0, "mcp_calls": 0, "mcp_decisions": ["once"]}
        with self.assertRaisesRegex(AssertionError, "downstream MCP count"):
            verify_turn(evidence, mounted=True, rejected=False)
        evidence["mcp_calls"] = 1
        verify_turn(evidence, mounted=True, rejected=False)

    def test_terminal_mcp_rejection_requires_all_skill_continuations(self):
        evidence = {"calls": {kind: str(i) for i, kind in enumerate(
            ("native", "native_read", "selected", "selected_read", "mcp"))},
            "model_requests": 5, "done": 1, "errors": 0, "mcp_calls": 0, "mcp_decisions": ["reject"]}
        with self.assertRaisesRegex(AssertionError, "tool continuations"):
            verify_turn(evidence, mounted=True, rejected=True)
        evidence["terminal_mcp_rejection"] = True
        verify_turn(evidence, mounted=True, rejected=True)
        evidence["model_requests"] = 4
        with self.assertRaisesRegex(AssertionError, "tool continuations"):
            verify_turn(evidence, mounted=True, rejected=True)

    def test_copilot_requires_same_call_id_and_explicit_rejection(self):
        probe = Probe({}, mounted=True, rejected=True)
        probe.pending_mcp = True
        probe.calls["mcp"] = "requested"
        event = {"type": "chat.tool", "kind": "complete", "status": "failed",
                 "call_id": "foreign", "payload": {"title": MCP_FUNCTION,
                                                  "error": "The user rejected this tool call."}}
        self.assertFalse(probe.observe_terminal([event]))
        event["call_id"] = "requested"
        self.assertFalse(probe.observe_terminal([dict(event, status="completed")]))
        self.assertTrue(probe.observe_terminal([event]))
        self.assertEqual(probe.completed, {"mcp": "requested"})
