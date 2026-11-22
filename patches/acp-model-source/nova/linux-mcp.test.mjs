import assert from 'node:assert/strict';
import { once } from 'node:events';
import { createServer } from 'node:http';
import test from 'node:test';
import { projectedOperation, selectProjectedServers } from './linux-mcp.mjs';

const endpoint = 'http://127.0.0.1:12345/';
const modelToken = 'a'.repeat(64);
const definition = {
  type: 'http',
  name: 'selected',
  url: endpoint + 'shiju-vtcode-mcp/selected',
  headers: [{ name: 'Authorization', value: 'Bearer ' + 'b'.repeat(64) }],
};
const select = (servers, reference = 'launch') =>
  selectProjectedServers(servers, endpoint, modelToken, reference);

test('only the owned projection with its distinct credential is accepted', () => {
  assert.equal(select([definition])[0].url, definition.url);
  for (const invalid of [
    { ...definition, type: 'stdio' },
    { ...definition, url: 'http://127.0.0.1:12346/shiju-vtcode-mcp/selected' },
    { ...definition, url: definition.url + '?extra' },
    { ...definition, headers: [{ name: 'Authorization', value: 'Bearer ' + modelToken }] },
  ]) assert.throws(() => select([definition, invalid]));
  assert.throws(() => select([definition, definition]));
  assert.throws(() => select([definition], ''));
});

test('projected invocation preserves its call identity and does not retry a refusal', async (t) => {
  const requests = [];
  const listener = createServer(async (request, response) => {
    const chunks = [];
    for await (const chunk of request) chunks.push(chunk);
    requests.push({ path: request.url, headers: request.headers, body: JSON.parse(Buffer.concat(chunks)) });
    response.writeHead(403).end();
  });
  t.after(() => listener.close());
  listener.listen(0, '127.0.0.1');
  await once(listener, 'listening');
  const base = `http://127.0.0.1:${listener.address().port}/`;
  const url = base + 'shiju-vtcode-mcp/selected';
  const [server] = selectProjectedServers([{ ...definition, url }], base, modelToken, 'launch');
  const request = {
    server, operation: 'call', name: 'read_probe', arguments: { value: 'probe' }, callId: 'model-call',
  };
  await assert.rejects(projectedOperation(request, new AbortController().signal), /rejected/);
  await assert.rejects(projectedOperation(request, AbortSignal.abort()), /abort/i);
  assert.equal(requests.length, 1);
  assert.equal(requests[0].path, '/shiju-vtcode-mcp/selected');
  assert.equal(requests[0].headers.authorization, definition.headers[0].value);
  assert.deepEqual(requests[0].body.params, {
    name: request.name, arguments: request.arguments, _meta: { 'shiju.nova.toolCallId': 'model-call' },
  });
});
