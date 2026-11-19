import unittest
from pathlib import Path
from tempfile import TemporaryDirectory

from mcp import StakpakMounts


class StakpakApprovalTest(unittest.TestCase):
    def test_observed_call_and_single_use_are_required(self):
        with TemporaryDirectory() as tmp, StakpakMounts(Path(tmp), Path(tmp)) as mounts:
            mounts.titles['Graph Query'] = ('graph', 'graph_query')
            arguments = {'path': 'docs/a.md', 'limit': 1}
            title = 'Graph Query docs/a.md'
            update = {'sessionUpdate': 'tool_call', 'toolCallId': 'call', 'title': title,
                      'rawInput': {**arguments, 'abs_path': '/workspace/docs/a.md'}}
            permission = {'sessionId': 'session', 'toolCall': {
                'toolCallId': 'call', 'title': title, 'rawInput': arguments},
                'options': [{'kind': 'allow_once', 'optionId': 'once'},
                            {'kind': 'allow_always', 'optionId': 'always'}]}
            mounts.begin('session')
            self.assertFalse(mounts.permission(1, permission))
            mounts.observe('other-session', update)
            self.assertFalse(mounts.permission(1, permission))
            mounts.observe('session', {**update, 'rawInput': {**arguments, 'limit': 2}})
            self.assertFalse(mounts.permission(1, permission))

            mounts.observe('session', update)
            self.assertTrue(mounts.permission(1, permission))
            self.assertEqual([o['optionId'] for o in permission['options']], ['once'])
            mounts.decide(1, {'outcome': {'outcome': 'selected', 'optionId': 'once'}})
            self.assertIsNone(mounts.claim('graph', 'graph_query', {**arguments, 'limit': True}))
            self.assertIsNotNone(mounts.claim('graph', 'graph_query', arguments))
            self.assertIsNone(mounts.claim('graph', 'graph_query', arguments))

            mounts.observe('session', update)
            self.assertTrue(mounts.permission(2, permission))
            mounts.end()
            mounts.decide(2, {'outcome': {'outcome': 'selected', 'optionId': 'once'}})
            mounts.begin('next-session')
            self.assertIsNone(mounts.claim('graph', 'graph_query', arguments))


    def test_native_permission_uses_the_original_options_and_grants_no_mcp_call(self):
        with TemporaryDirectory() as tmp, StakpakMounts(Path(tmp), Path(tmp)) as mounts:
            arguments = {'path': 'docs/a.md', 'file_text': 'fixture'}
            title = 'Creating docs/a.md'
            options = [{'kind': 'allow_once', 'optionId': 'once'},
                       {'kind': 'reject_once', 'optionId': 'reject'}]
            update = {'sessionUpdate': 'tool_call', 'toolCallId': 'native', 'title': title,
                      'rawInput': {**arguments, 'abs_path': '/workspace/docs/a.md'}}
            permission = {'sessionId': 'session', 'toolCall': {
                'toolCallId': 'native', 'title': title, 'rawInput': arguments}, 'options': options}
            mounts.begin('session')
            mounts.observe('session', update)
            self.assertTrue(mounts.permission(1, permission))
            self.assertIs(permission['options'], options)
            mounts.decide(1, {'outcome': {'outcome': 'selected', 'optionId': 'once'}})
            self.assertIsNone(mounts.claim('graph', 'create', arguments))


if __name__ == '__main__':
    unittest.main()
