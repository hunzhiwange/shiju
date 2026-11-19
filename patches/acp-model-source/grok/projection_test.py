import io
import json
import os
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

from projection import Catalog, Selection, encode, project_request, project_stream, publish, read_catalog

SERVER = 'example'
TOOL = {'name': 'lookup', 'description': 'Look up a symbol', 'inputSchema': {'type': 'object'}}
NATIVE = [{'type': 'function', 'function': {'name': name}} for name in ('read_file', 'search_tool', 'use_tool')]


class ProjectionTests(unittest.TestCase):
    def test_unmounted_catalog_keeps_native_tools(self):
        body = {'tools': NATIVE, 'messages': []}
        self.assertEqual(project_request(body, {}), {'read_file': None})
        self.assertEqual([t['function']['name'] for t in body['tools']], ['read_file'])

    def test_direct_mapping_preserves_approval_dispatch_and_history(self):
        body = {'tools': NATIVE, 'messages': []}
        mapping = project_request(body, {SERVER: [TOOL]})
        self.assertEqual([t['function']['name'] for t in body['tools']], ['read_file', 'lookup'])
        stream = io.BytesIO(
            frame({'tool_calls': [{'index': 0, 'id': 'call_1', 'function': {'name': 'look', 'arguments': '{"sym'}}]})
            + frame({'tool_calls': [{'index': 0, 'function': {'name': 'up', 'arguments': 'bol":"hello"}'}}]})
            + frame({}, 'tool_calls') + b'data: [DONE]\n\n'
        )
        result = list(project_stream(stream, mapping))
        calls = [call for line in result if line.startswith(b'data: {')
                 for choice in json.loads(line[6:])['choices']
                 for call in choice['delta'].get('tool_calls', [])]
        self.assertEqual(len(calls), 1)
        self.assertEqual(calls[0]['id'], 'call_1')
        function = calls[0]['function']
        self.assertEqual(function['name'], 'use_tool')
        self.assertEqual(json.loads(function['arguments']),
                         {'tool_name': 'example__lookup', 'tool_input': {'symbol': 'hello'}})
        request = {'tools': NATIVE, 'messages': [{'role': 'assistant', 'tool_calls': calls}]}
        project_request(request, {SERVER: [TOOL]})
        self.assertEqual(request['messages'][0]['tool_calls'][0]['function'],
                         {'name': 'lookup', 'arguments': '{"symbol":"hello"}'})

    def test_unselected_dispatch_is_rejected(self):
        data = frame({'tool_calls': [{'index': 0, 'id': 'call_1',
                                      'function': {'name': 'use_tool', 'arguments': '{}'}}]})
        with self.assertRaises(ValueError):
            list(project_stream(io.BytesIO(data + frame({}, 'tool_calls')), {}))

    def test_catalog_uses_complete_pages_and_revokes_on_change(self):
        with tempfile.TemporaryDirectory() as folder:
            path = Path(folder) / 'tools.json'
            catalog = Catalog(path)
            catalog.request({'id': 1, 'method': 'tools/list'})
            catalog.response({'id': 1, 'result': {'tools': [TOOL], 'nextCursor': 'page-2'}})
            self.assertFalse(path.exists())
            catalog.request({'id': 2, 'method': 'tools/list', 'params': {'cursor': 'page-2'}})
            catalog.response({'id': 2, 'result': {'tools': []}})
            self.assertEqual(read_catalog(path), [TOOL])
            catalog.response({'method': 'notifications/tools/list_changed'})
            self.assertFalse(path.exists())

    def test_catalog_honors_qualified_policy_and_deny_precedence(self):
        with tempfile.TemporaryDirectory() as folder:
            path = Path(folder) / 'tools.json'
            policy = {'allow': ['mcp__example__lookup', 'other'], 'deny': ['mcp__example__other']}
            catalog = Catalog(path, policy, SERVER)
            self.assertFalse(catalog.admits('other'))
            self.assertFalse(catalog.admits('unknown'))
            self.assertTrue(catalog.admits('lookup'))
            catalog.request({'id': 1, 'method': 'tools/list'})
            result = {'id': 1, 'result': {'tools': [TOOL, {'name': 'other'}]}}
            catalog.response(result)
            self.assertEqual(result['result']['tools'], [TOOL])
            self.assertEqual(read_catalog(path), [TOOL])

    def test_revoked_stream_cannot_dispatch(self):
        with self.assertRaisesRegex(ValueError, 'revoked'):
            list(project_stream(io.BytesIO(
                frame({'tool_calls': [{'index': 0, 'id': 'call_1',
                                      'function': {'name': 'lookup', 'arguments': '{}'}}]})
                + frame({}, 'tool_calls')
            ), {'lookup': 'example__lookup'}, authorize=lambda _: False))

    def test_stdio_filters_policy_and_removes_relay_token(self):
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder)
            path = root / 'catalog.json'
            publish(path.with_suffix('.policy'), {'server': SERVER, 'policy': {'deny': ['denied']}})
            peer = root / 'peer.py'
            peer.write_text("""
import json
import os
import sys
assert 'SHIJU_GROK_RELAY_KEY' not in os.environ
for line in sys.stdin:
    req = json.loads(line)
    assert req['method'] == 'tools/list', req
    print(json.dumps({'id': req['id'], 'result': {'tools': [
        {'name': 'lookup', 'inputSchema': {}},
        {'name': 'denied', 'inputSchema': {}},
    ]}}), flush=True)
""")
            request = b'\n'.join(map(encode, [
                {'id': 1, 'method': 'tools/list'},
                {'id': 2, 'method': 'tools/call', 'params': {'name': 'denied'}},
            ])) + b'\n'
            result = subprocess.run([
                sys.executable, '-I', str(Path(__file__).with_name('projection.py')),
                'mcp', str(path), sys.executable, str(peer),
            ], input=request, capture_output=True, timeout=5, check=True,
                env=dict(os.environ, SHIJU_GROK_RELAY_KEY='fixture-only'))
            replies = {msg['id']: msg for msg in map(json.loads, result.stdout.splitlines())}
            self.assertEqual([tool['name'] for tool in replies[1]['result']['tools']], ['lookup'])
            self.assertIn('error', replies[2])
            self.assertFalse(path.exists())

    def test_loading_without_servers_revokes_the_previous_catalog(self):
        with tempfile.TemporaryDirectory() as folder:
            selection = Selection(Path(folder), Path('projection.py'))
            selection.bind({'cwd': folder, 'mcpServers': [
                {'name': SERVER, 'command': 'mcp', 'args': []},
            ]})
            original = selection.snapshot
            publish(original[1][SERVER], [TOOL])
            self.assertTrue(selection.authorizes(original, 'example__lookup'))
            selection.bind({'cwd': folder, 'mcpServers': []})
            self.assertFalse(selection.authorizes(original, 'example__lookup'))
            self.assertEqual(selection.current(selection.snapshot), (folder, {}))
            selection.close()
            self.assertEqual(selection.current(selection.snapshot), (None, {}))

    def test_resource_only_server_has_an_empty_catalog(self):
        with tempfile.TemporaryDirectory() as folder:
            path = Path(folder) / 'tools.json'
            catalog = Catalog(path)
            catalog.request({'id': 1, 'method': 'initialize'})
            catalog.response({'id': 1, 'result': {'capabilities': {'resources': {}}}})
            self.assertEqual(read_catalog(path), [])

    def test_long_colliding_tool_names_get_distinct_aliases(self):
        name = 'lookup_' + 'a' * 60
        tool = dict(TOOL, name=name)
        body = {'tools': NATIVE, 'messages': []}
        mapping = project_request(body, {'first': [tool], 'second': [tool]})
        aliases = {alias: target for alias, target in mapping.items() if target}
        self.assertEqual(set(aliases.values()), {f'first__{name}', f'second__{name}'})
        self.assertEqual(len(aliases), 2)
        self.assertTrue(all(len(alias) <= 64 for alias in aliases))

    def test_catalog_shares_no_state_between_selections(self):
        with tempfile.TemporaryDirectory() as folder:
            a, b = (Path(folder) / name for name in ('first', 'second'))
            publish(a, [TOOL])
            self.assertEqual(read_catalog(a), [TOOL])
            self.assertIsNone(read_catalog(b))


def frame(delta, finish=None):
    return b'data: ' + encode({'choices': [{'index': 0, 'delta': delta, 'finish_reason': finish}]}) + b'\n\n'


if __name__ == '__main__':
    unittest.main()
