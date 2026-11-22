import unittest
from pathlib import Path
from tempfile import TemporaryDirectory

from mcp import StakpakMounts


class LinuxStakpakApprovalTest(unittest.TestCase):
    def test_dispatch_requires_the_approved_session_call_and_arguments(self):
        with TemporaryDirectory() as tmp, StakpakMounts(Path(tmp), Path(tmp)) as mounts:
            mounts.titles["Read Probe"] = ("fixture", "read_probe")
            mounts.begin("session")
            arguments = {"limit": 1}

            def ask(call, request):
                tool_call = {"title": "Read Probe", "toolCallId": call, "rawInput": arguments}
                mounts.observe("session", {**tool_call, "sessionUpdate": "tool_call"})
                self.assertTrue(mounts.permission(request, {
                    "sessionId": "session", "toolCall": tool_call,
                    "options": [{"kind": "allow_once", "optionId": "once"}],
                }))

            allow = {"outcome": {"outcome": "selected", "optionId": "once"}}
            ask("approved", 1)
            ask("denied", 2)
            mounts.decide(1, allow)
            mounts.decide(2, {"outcome": {"outcome": "cancelled"}})

            def claim(session, call, params=arguments):
                return mounts.claim("fixture", "read_probe", params, {
                    "dev.shiju/acp-session-id": session,
                    "dev.shiju/acp-tool-call-id": call,
                })

            self.assertIsNone(claim("session", "denied"))
            self.assertIsNone(claim("other-session", "approved"))
            self.assertIsNone(claim("session", "approved", {"limit": 2}))
            self.assertIsNotNone(claim("session", "approved"))
            self.assertIsNone(claim("session", "approved"))

            ask("revoked", 3)
            ask("late", 4)
            mounts.decide(3, allow)
            mounts.end()
            mounts.decide(4, allow)
            mounts.begin("session")
            self.assertIsNone(claim("session", "revoked"))
            self.assertIsNone(claim("session", "late"))

            ask("expired", 5)
            mounts.decide(5, allow)
            from unittest.mock import patch
            with patch("mcp.monotonic", return_value=float("inf")):
                self.assertIsNone(claim("session", "expired"))


if __name__ == "__main__":
    unittest.main()
