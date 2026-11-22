"""Run with the reviewed, Linux-patched Python 3.12 virtual environment."""
import asyncio
import json
import os
import sys
from types import SimpleNamespace
import unittest
from unittest.mock import AsyncMock, Mock, patch

from acp.schema import HttpHeader, McpServerHttp, McpServerStdio
from acp_adapter.server import HermesACPAgent
from tools import mcp_tool
from tools.approval import reset_current_observability_context, set_current_observability_context


@unittest.skipUnless(sys.platform == "linux", "Linux managed overlay")
class ManagedMcpScopeTests(unittest.IsolatedAsyncioTestCase):
    def setUp(self):
        env = patch.dict(os.environ, {
            "VW_ACP_LAUNCH_REFERENCE": "vw-session-model:" + "0" * 64,
            "CUSTOM_BASE_URL": "http://127.0.0.1:12345/v1",
        })
        env.start()
        self.addCleanup(env.stop)
        self.agent = HermesACPAgent(session_manager=object())
        self.state = SimpleNamespace(session_id="session")
        self.server = McpServerHttp(
            name="probe", type="http",
            url="http://127.0.0.1:12345/shiju-vtcode-mcp/probe",
            headers=[HttpHeader(name="Authorization", value="Bearer fixture")],
        )

    def test_scope_is_invalidated_by_removal(self):
        self.assertTrue(self.agent._bind_managed_mcp_scope(self.state.session_id, [self.server]))
        self.agent._managed_mcp_ready = True
        self.assertFalse(self.agent._bind_managed_mcp_scope(self.state.session_id, [self.server]))
        with self.assertRaisesRegex(RuntimeError, "restart"):
            self.agent._bind_managed_mcp_scope(self.state.session_id, [])
        self.assertFalse(self.agent._managed_mcp_ready)
        with self.assertRaises(RuntimeError):
            self.agent._bind_managed_mcp_scope(self.state.session_id, [self.server])

    async def test_direct_stdio_is_rejected_before_registration(self):
        server = McpServerStdio(name="probe", command="/fixture/mcp", args=[], env=[])
        with patch("tools.mcp_tool.register_mcp_servers") as register:
            with self.assertRaisesRegex(RuntimeError, "host MCP relay"):
                await self.agent._register_session_mcp_servers(self.state, [server])
            register.assert_not_called()

    async def test_prompt_after_scope_change_is_refused(self):
        await self.agent._register_session_mcp_servers(self.state, [])
        with self.assertRaises(RuntimeError):
            await self.agent._register_session_mcp_servers(SimpleNamespace(session_id="other"), [])
        result = await self.agent.prompt(prompt=[], session_id=self.state.session_id)
        self.assertEqual(result.stop_reason, "refusal")


@unittest.skipUnless(sys.platform == "linux", "Linux managed overlay")
class ManagedMcpCallTests(unittest.TestCase):
    def setUp(self):
        env = patch.dict(os.environ, {"VW_ACP_LAUNCH_REFERENCE": "vw-session-model:" + "0" * 64})
        env.start()
        self.addCleanup(env.stop)
        self.call_tool = AsyncMock(return_value=SimpleNamespace(isError=False, content=[]))
        server = SimpleNamespace(session=SimpleNamespace(call_tool=self.call_tool),
                                 _rpc_lock=asyncio.Lock())
        transport = patch.object(mcp_tool, "_get_connected_server_for_call", return_value=server)
        transport.start()
        self.addCleanup(transport.stop)
        loop = patch.object(mcp_tool, "_run_on_mcp_loop", side_effect=lambda command, **_: asyncio.run(command()))
        loop.start()
        self.addCleanup(loop.stop)
        self.handler = mcp_tool._make_tool_handler("probe", "read_probe", 2)
        self.approve = Mock(return_value=True)
        self.scope_current = Mock(return_value=True)
        token = mcp_tool._acp_tool_approval.set((self.approve, self.scope_current))
        self.addCleanup(mcp_tool._acp_tool_approval.reset, token)

    def test_original_identity_is_sent_after_approval(self):
        token = set_current_observability_context(tool_call_id="model-call")
        self.addCleanup(reset_current_observability_context, token)
        self.handler({"path": "fixture"})
        self.approve.assert_called_once_with("mcp__probe__read_probe", {"path": "fixture"}, "model-call")
        self.call_tool.assert_awaited_once_with("read_probe", arguments={"path": "fixture"},
                                                meta={"shijuCallId": "model-call"})

    def test_missing_identity_denies_without_execution(self):
        token = set_current_observability_context()
        self.addCleanup(reset_current_observability_context, token)
        self.assertIn("denied", json.loads(self.handler({}))["error"])
        self.approve.assert_not_called()
        self.call_tool.assert_not_awaited()

    def test_scope_change_after_approval_denies_without_execution(self):
        token = set_current_observability_context(tool_call_id="model-call")
        self.addCleanup(reset_current_observability_context, token)
        self.scope_current.side_effect = [True, False]
        self.assertIn("revoked", json.loads(self.handler({}))["error"])
        self.call_tool.assert_not_awaited()


if __name__ == "__main__":
    unittest.main()
