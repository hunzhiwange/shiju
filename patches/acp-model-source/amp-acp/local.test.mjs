import { test } from 'node:test';
import assert from 'node:assert/strict';
import { LocalAmp } from './local.mjs';

for (const invalidate of ['cancel', 'replace']) {
  test(`Amp reaps a terminal created after turn ${invalidate}`, async () => {
    const active = { controller: new AbortController() };
    const session = { cwd: '/tmp', active };
    const calls = [];
    const agent = {
      sessions: new Map([['session', session]]),
      client: {
        requestPermission: async () => ({ outcome: { outcome: 'selected', optionId: 'allow_once' } }),
        createTerminal: async () => {
          if (invalidate === 'cancel') active.controller.abort();
          else session.active = { controller: new AbortController() };
          return {
            kill: async () => { calls.push('kill'); },
            release: async () => { calls.push('release'); },
            waitForExit: async () => { calls.push('wait'); },
          };
        },
      },
    };
    await assert.rejects(
      new LocalAmp(agent).tool({ sessionId: 'session', id: 'tool', name: 'Bash', input: { cmd: 'true' } }),
      /turn is no longer active/,
    );
    assert.deepEqual(calls, ['kill', 'release']);
  });
}

// The legacy Opus display default is not a preference for the selected model.
test('Amp removes its native summarized thinking display before the model relay', async () => {
  const { createServer } = await import('node:http');
  const { mkdtemp, rm } = await import('node:fs/promises');
  const { tmpdir } = await import('node:os');
  const directory = await mkdtemp(`${tmpdir()}/amp-thinking-`);
  const keys = ['SHIJU_AMP_RELAY_URL', 'SHIJU_AMP_MODEL', 'SHIJU_AMP_BINDING', 'SHIJU_AMP_RELAY_TOKEN', 'SHIJU_AMP_REASONING', 'AMP_ACP_STATE_DIR'];
  const previous = Object.fromEntries(keys.map(key => [key, process.env[key]]));
  let received;
  const upstream = createServer(async (req, res) => {
    const chunks = [];
    for await (const chunk of req) chunks.push(chunk);
    received = JSON.parse(Buffer.concat(chunks).toString());
    res.writeHead(received.thinking?.display === undefined ? 200 : 422, { 'content-type': 'application/json' });
    res.end('{}');
  });
  await new Promise(resolve => upstream.listen(0, '127.0.0.1', resolve));
  Object.assign(process.env, {
    SHIJU_AMP_RELAY_URL: `http://127.0.0.1:${upstream.address().port}/`,
    SHIJU_AMP_MODEL: 'selected-model', SHIJU_AMP_BINDING: 'test-binding',
    SHIJU_AMP_RELAY_TOKEN: 'test-token', SHIJU_AMP_REASONING: 'true', AMP_ACP_STATE_DIR: directory,
  });
  const local = new LocalAmp({ sessions: new Map([['session', { active: { controller: new AbortController() } }]]) });
  try {
    await local.start();
    const env = local.environment('session');
    const response = await fetch(`${local.url}/api/provider/anthropic/v1/messages`, {
      method: 'POST', headers: { authorization: `Bearer ${env.AMP_API_KEY}`, 'content-type': 'application/json' },
      body: JSON.stringify({ model: 'native-opus', max_tokens: 2048, thinking: { type: 'adaptive', display: 'summarized' }, messages: [] }),
    });
    await response.text();
    assert.equal(response.status, 200);
    assert.equal(received.model, 'selected-model');
    assert.deepEqual(received.thinking, { type: 'adaptive' });
    const unknown = await fetch(`${local.url}/api/provider/anthropic/v1/messages`, {
      method: 'POST', headers: { authorization: `Bearer ${env.AMP_API_KEY}`, 'content-type': 'application/json' },
      body: JSON.stringify({ model: 'native-opus', max_tokens: 2048, thinking: { type: 'adaptive', display: 'unknown' }, messages: [] }),
    });
    await unknown.text();
    assert.equal(unknown.status, 422);
    assert.equal(received.thinking.display, 'unknown');
  } finally {
    local.close();
    upstream.closeAllConnections();
    await new Promise(resolve => upstream.close(resolve));
    for (const key of keys) {
      if (previous[key] === undefined) delete process.env[key];
      else process.env[key] = previous[key];
    }
    await rm(directory, { recursive: true, force: true });
  }
});
