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
