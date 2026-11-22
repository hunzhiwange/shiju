import assert from 'node:assert/strict';
import { test } from 'node:test';
import { pathToFileURL } from 'node:url';

const bridge = process.env.SHIJU_AMP_LINUX_BRIDGE;

test('Amp Linux rejects stdio before configuring or reloading a Worker', {
  skip: !bridge && 'point SHIJU_AMP_LINUX_BRIDGE at the generated Linux control module',
}, async () => {
  const { LocalAmp, selectedStdio } = await import(pathToFileURL(bridge));
  const oldView = process.env.SHIJU_AMP_SKILL_VIEW;
  process.env.SHIJU_AMP_SKILL_VIEW = '[]';
  try {
    const servers = [{ name: 'selected', command: '/bin/false', args: [], env: [] }];
    const missingIsolation = /Amp Linux requires an isolated MCP transport/;
    assert.throws(() => selectedStdio(servers), missingIsolation);
    assert.deepEqual(Object.keys(selectedStdio([])), []);

    const closed = [];
    const local = new LocalAmp({ transport: { closeSession: async id => closed.push(id) } });
    const session = { queue: [], mcpConfig: {}, processStarted: true };
    local.tokens.set('old-token', { sessionId: 'session' });
    await assert.rejects(local.reloadSelection(session, { sessionId: 'session', mcpServers: servers }),
      missingIsolation);
    assert.deepEqual(closed, []);

    await local.reloadSelection(session, { sessionId: 'session', mcpServers: [] });
    assert.deepEqual(closed, ['session']);
    assert.equal(local.tokens.size, 0);
    assert.equal(session.processStarted, false);
    assert.deepEqual(Object.keys(session.mcpConfig), []);
  } finally {
    if (oldView === undefined) delete process.env.SHIJU_AMP_SKILL_VIEW;
    else process.env.SHIJU_AMP_SKILL_VIEW = oldView;
  }
});
