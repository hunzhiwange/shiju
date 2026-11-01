# Synthetic local protocol fixture; repository license. No network or credentials.
import json
import pathlib
import sys
for line in sys.stdin:
    request = json.loads(line)
    if 'id' not in request:
        continue
    method = request['method']
    if method == 'initialize':
        result = {'protocolVersion': '2025-06-18', 'capabilities': {'tools': {}, 'resources': {}}, 'serverInfo': {'name': 'weather', 'version': '1'}}
    elif method == 'tools/list':
        result = {'tools': [{'name': 'forecast', 'inputSchema': {'type': 'object'}, '_meta': {'ui': {'resourceUri': 'ui://weather/view'}}}]}
    elif method == 'tools/call':
        result = {'content': [{'type': 'text', 'text': 'Weather ready'}], 'structuredContent': {'temperature': 12}}
    elif method == 'resources/read':
        result = json.loads(pathlib.Path(__file__).with_name('resource.json').read_text())
    else:
        result = {}
    print(json.dumps({'jsonrpc': '2.0', 'id': request['id'], 'result': result}), flush=True)
