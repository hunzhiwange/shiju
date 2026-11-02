# Repository-authored MIT fixture. Only the caller-selected sandbox output is written.
import json
import sys
from pathlib import Path
for line in sys.stdin:
    m = json.loads(line)
    if 'id' not in m:
        continue
    method = m['method']
    if method == 'initialize':
        r = {'protocolVersion': '2024-11-05', 'capabilities': {'tools': {}}, 'serverInfo': {'name': 'writer', 'version': '1'}}
    elif method == 'tools/list':
        r = {'tools': [{'name': 'write', 'inputSchema': {'type': 'object', 'properties': {'output': {'type': 'string'}}, 'required': ['output']}}]}
    elif method == 'tools/call':
        Path(m['params']['arguments']['output']).write_text('original package executed\n')
        r = {'content': [{'type': 'text', 'text': 'written'}]}
    else:
        r = {}
    print(json.dumps({'jsonrpc': '2.0', 'id': m['id'], 'result': r}), flush=True)
