"""Focus on Junie's native approval binding and mount removal."""
import asyncio
from pathlib import Path
import runpy
import unittest

FILES = Path(__file__).resolve().parent
BRIDGE = runpy.run_path(str(FILES / 'mcp_bridge.py'))
STDIO = runpy.run_path(str(FILES.parent / 'cortex-code/mcp_bridge.py'))


class Peer:
    def __init__(self, _definition, _workspace):
        self.closed = False
        self.calls = []

    async def list_tools(self):
        for name in ('query', 'node'):
            yield {'name': name, 'inputSchema': {'type': 'object'}}

    async def call(self, name, arguments):
        self.calls.append((name, arguments))
        return {'content': [{'type': 'text', 'text': 'result'}]}

    async def close(self):
        self.closed = True


class BridgeTests(unittest.IsolatedAsyncioTestCase):
    async def asyncSetUp(self):
        workspace = FILES
        self.session = BRIDGE['Session'](workspace, Peer, STDIO['allowed_tool'])
        request = {'params': {'cwd': str(workspace), 'mcpServers': [{
            'name': 'probe', '_meta': {'shiju.junie.toolPolicy': {'allow': ['query', 'node'], 'deny': ['node']}}
        }]}}
        await self.session.stage(request, 12345)
        self.path, self.mount = next(iter(self.session.mounts.items()))
        self.mount.initialized = True
        self.session.session, self.session.turn = 'first', 3

    def request_approval(self):
        frame = {'id': 5, 'params': {'sessionId': 'first', 'toolCall': {
            'title': 'probe/query', '_meta': {'is_mcp_tool_call': True},
            'content': [{'type': 'content', 'content': {'type': 'text', 'text': '{"search":"a"}'}}],
        }, 'options': [{'kind': 'allow_once', 'optionId': 'yes'},
                       {'kind': 'reject_once', 'optionId': 'no'},
                       {'kind': 'allow_always', 'optionId': 'always'}]}}
        self.session.approval(frame)
        self.assertEqual([o['optionId'] for o in frame['params']['options']], ['yes', 'no'])

    def decision(self, option):
        self.session.decision({'id': 5, 'result': {
            'outcome': {'outcome': 'selected', 'optionId': option}}})

    async def call(self, search='a'):
        _, result = await self.session.mcp(self.path, {'id': 1, 'method': 'tools/call',
            'params': {'name': 'query', 'arguments': {'search': search}}})
        return result

    async def test_approval_is_argument_bound_and_single_use(self):
        self.assertEqual(set(self.mount.tools), {'query'})
        self.request_approval()
        self.decision('yes')
        self.assertIn('error', await self.call('other'))
        self.assertIn('result', await self.call())
        self.assertIn('error', await self.call())
        self.assertEqual(self.mount.peer.calls, [('query', {'search': 'a'})])

    async def test_reject_and_cancel_revoke_pending_execution(self):
        for option in ('no', 'always'):
            self.request_approval()
            self.decision(option)
            self.assertIn('error', await self.call())
        self.request_approval()
        await self.session.revoke()
        self.decision('yes')
        self.assertIn('error', await self.call())
        self.assertEqual(self.mount.peer.calls, [])

    async def test_revoke_cancels_an_active_call(self):
        started = asyncio.Event()

        async def run_tool(_name, _arguments):
            started.set()
            await asyncio.Future()

        self.mount.peer.call = run_tool
        self.request_approval()
        self.decision('yes')
        running = asyncio.create_task(self.call())
        await started.wait()
        await self.session.revoke()
        self.assertTrue(running.cancelled())
        self.assertIn('error', await self.call())

    async def test_removed_mount_closes_and_invalidates_url(self):
        await self.session.clear()
        self.assertTrue(self.mount.peer.closed)
        self.assertEqual(await self.session.mcp(self.path, {'id': 1, 'method': 'tools/list'}), (404, None))


if __name__ == '__main__':
    unittest.main()
