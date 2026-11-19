"""Exercise the OMP relay's host approval and mount lifecycle."""
import asyncio
from pathlib import Path
import unittest

from mcp_bridge import Session


class Peer:
    def __init__(self, definition, workspace):
        self.executed = []
        self.closed = False

    async def list_tools(self):
        for name in ('query', 'denied'):
            yield {'name': name, 'inputSchema': {'type': 'object'}}

    async def close(self):
        self.closed = True

    async def call(self, name, arguments):
        self.executed.append((name, arguments))
        return {'content': [{'type': 'text', 'text': 'result'}]}


class ApprovalTests(unittest.IsolatedAsyncioTestCase):
    async def asyncSetUp(self):
        workspace = Path.cwd().resolve()
        self.session = Session(workspace, Peer, lambda policy, _, name:
                               name not in policy['_meta']['shiju.cortex.toolPolicy']['deny'])
        self.requests = asyncio.Queue()
        self.session.emit = self.requests.put_nowait
        await self.session.stage({'params': {'cwd': str(workspace), 'mcpServers': [{
            'name': 'graph', '_meta': {'shiju.omp.toolPolicy': {'deny': ['denied']}},
        }]}}, 12345)
        self.path, self.mount = next(iter(self.session.mounts.items()))
        self.session.session = 'native-session'
        self.session.turn = 'prompt'
        await self.session.mcp(self.path, {'method': 'notifications/initialized'})

    def call(self):
        return asyncio.create_task(self.session.mcp(self.path, {
            'method': 'tools/call', 'id': 1, 'params': {'name': 'query', 'arguments': {'name': 'main'}},
        }))

    @staticmethod
    def decision(request, option):
        return {'id': request['id'], 'result': {'outcome': {'outcome': 'selected', 'optionId': option}}}

    async def test_one_call_requires_one_approval(self):
        approved = self.call()
        request = await asyncio.wait_for(self.requests.get(), 1)
        self.assertEqual(request['params']['sessionId'], 'native-session')
        self.assertEqual(request['params']['toolCall']['rawInput'], {'name': 'main'})
        self.assertEqual(self.mount.peer.executed, [])
        decision = self.decision(request, 'allow_once')
        self.assertTrue(self.session.decision(decision))
        status, response = await approved
        self.assertEqual(status, 200)
        self.assertEqual(response['result']['content'][0]['text'], 'result')

        denied = self.call()
        other = await asyncio.wait_for(self.requests.get(), 1)
        self.assertTrue(self.session.decision(decision))
        self.session.decision(self.decision(other, 'reject_once'))
        self.assertTrue((await denied)[1]['result']['isError'])
        self.assertEqual(self.mount.peer.executed, [('query', {'name': 'main'})])

    async def test_policy_applies_to_catalog_and_calls(self):
        _, response = await self.session.mcp(self.path, {'method': 'tools/list', 'id': 1})
        self.assertEqual([tool['name'] for tool in response['result']['tools']], ['query'])
        _, response = await self.session.mcp(self.path, {
            'method': 'tools/call', 'id': 2, 'params': {'name': 'denied'},
        })
        self.assertTrue(response['result']['isError'])
        self.assertTrue(self.requests.empty())

    async def test_late_approval_after_cancel_cannot_execute(self):
        task = self.call()
        request = await asyncio.wait_for(self.requests.get(), 1)
        await self.session.revoke()
        self.session.decision(self.decision(request, 'allow_once'))
        self.assertTrue(task.cancelled())
        self.assertTrue(self.mount.peer.closed)
        self.assertEqual(self.mount.peer.executed, [])

    async def test_removal_invalidates_mount_url(self):
        await self.session.clear()
        self.assertEqual(await self.session.mcp(self.path, {'method': 'tools/list', 'id': 1}), (404, None))


if __name__ == '__main__':
    unittest.main()
