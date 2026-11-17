"""Native Harn smoke: natural-language input must reach provider dispatch, not the parser."""
import json
import os
from pathlib import Path
import selectors
import shutil
import subprocess
import tempfile
import unittest


class NaturalLanguagePipeline(unittest.TestCase):
    def test_prompt_reaches_provider_without_compiling_user_text(self):
        binary = os.environ['HARN_TEST_BINARY']
        pipeline = Path(__file__).with_name('agent.harn').resolve()
        self.assertTrue(pipeline.is_file(), 'reviewed natural-language pipeline is missing')
        with tempfile.TemporaryDirectory() as directory:
            copied = Path(directory) / 'agent.harn'
            shutil.copy2(pipeline, copied)
            pipeline = copied
            env = {'PATH': os.environ['PATH'], 'HOME': directory,
                   'XDG_CONFIG_HOME': directory, 'XDG_DATA_HOME': directory,
                   'XDG_CACHE_HOME': directory}
            process = subprocess.Popen(
                [binary, 'serve', 'acp', '--session-environment', 'isolated', str(pipeline)],
                cwd=directory, env=env, stdin=subprocess.PIPE, stdout=subprocess.PIPE,
                stderr=subprocess.PIPE, text=True, bufsize=1)
            selector = selectors.DefaultSelector()
            selector.register(process.stdout, selectors.EVENT_READ)
            events = []
            def call(number, method, params):
                process.stdin.write(json.dumps({'jsonrpc': '2.0', 'id': number,
                                                'method': method, 'params': params}) + '\n')
                process.stdin.flush()
                for _ in range(60):
                    if not selector.select(1):
                        continue
                    line = process.stdout.readline()
                    self.assertTrue(line, 'Harn exited before responding')
                    response = json.loads(line)
                    events.append(response)
                    if 'method' in response and 'id' in response:
                        process.stdin.write(json.dumps({'jsonrpc':'2.0','id':response['id'],'error':{'code':-32000,'message':'not authorized by native test'}})+'\n')
                        process.stdin.flush()
                    if 'method' not in response and response.get('id') == number:
                        return response
                self.fail('native ACP request timed out')
            try:
                self.assertIn('result', call(1, 'initialize', {'protocolVersion': 1, 'clientCapabilities': {}}))
                session = call(2, 'session/new', {'cwd': directory, 'mcpServers': []})['result']['sessionId']
                result = call(3, 'session/prompt', {'sessionId': session, 'prompt': [
                    {'type': 'text', 'text': '你好。只回复 OK；不要创建文件。'}]})
                serialized = json.dumps(result)
                self.assertNotIn('Compilation error', serialized)
                self.assertNotIn('Unexpected character', serialized)
                # No credentials are supplied: dispatch must fail explicitly, never fall back.
                self.assertIn('error', result)
                self.assertTrue(any(event.get('params', {}).get('outcome', {}).get('kind') == 'provider_error' for event in events), 'missing credentials must fail at the provider boundary')
            finally:
                process.stdin.close()
                try:
                    process.wait(timeout=5)
                except subprocess.TimeoutExpired:
                    process.terminate()
                    process.wait(timeout=5)
                selector.close()
                process.stdout.close()
                process.stderr.close()


if __name__ == '__main__':
    unittest.main()
