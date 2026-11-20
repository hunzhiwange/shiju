import { test } from 'node:test';
import assert from 'node:assert/strict';
import { Readable } from 'node:stream';
import { gzipSync } from 'node:zlib';
import { LocalAmp, selectedStdio } from './local.mjs';

test('Amp accepts only uniquely named, explicit stdio commands', () => {
  const server = { name: 'selected_graph', command: '/bin/graph', args: ['serve'], env: [] };
  assert.deepEqual(Object.entries(selectedStdio([server])), [
    ['selected_graph', { command: '/bin/graph', args: ['serve'], env: {} }],
  ]);
  for (const selection of [
    [server, server], [{ ...server, type: 'http' }], [{ ...server, name: 'selected-graph' }],
    [{ ...server, command: 'graph' }],
  ]) {
    assert.throws(() => selectedStdio(selection), /Amp requires distinct stdio MCP servers/);
  }
});

test('Amp reads compressed native thread uploads inside the existing body limit', async () => {
  const local = new LocalAmp({});
  local.url = 'http://127.0.0.1:9';
  const asRequest = (url, body) => Object.assign(Readable.from([gzipSync(body)]), {
    url, headers: { 'content-encoding': 'gzip' },
  });
  const payload = { method: 'uploadThread', params: { thread: { content: 'x'.repeat(32000) } } };
  assert.deepEqual(await local.body(asRequest('/api/internal?uploadThread', JSON.stringify(payload))), payload);
  await assert.rejects(local.body(asRequest('/shiju/tool', '{}')), /Unsupported Amp control encoding/);
  await assert.rejects(local.body(asRequest('/api/internal?uploadThread', 'x'.repeat(16 * 1024 * 1024 + 1))), {
    code: 'ERR_BUFFER_TOO_LARGE',
  });
});

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

async function withLocalAmp(run) {
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
  const session = { cwd: directory, active: { controller: new AbortController() } };
  const local = new LocalAmp({ sessions: new Map([['session', session]]), client: {} });
  try {
    await local.start();
    const env = local.environment('session');
    const post = async (route, body) => {
      const response = await fetch(`${local.url}${route}`, {
        method: 'POST', headers: { authorization: `Bearer ${env.AMP_API_KEY}`, 'content-type': 'application/json' },
        body: JSON.stringify(body),
      });
      return { status: response.status, body: await response.json() };
    };
    const model = async body => {
      const response = await post('/api/provider/anthropic/v1/messages', {
        model: 'native-opus', max_tokens: 2048, messages: [], ...body,
      });
      return { ...response, received };
    };
    await run({ local, session, post, model });
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
}

test('Amp removes its native summarized thinking display before the model relay', () => withLocalAmp(async ({ model }) => {
  const response = await model({ thinking: { type: 'adaptive', display: 'summarized' } });
  assert.equal(response.status, 200);
  assert.equal(response.received.model, 'selected-model');
  assert.deepEqual(response.received.thinking, { type: 'adaptive' });
  const unknown = await model({ thinking: { type: 'adaptive', display: 'unknown' } });
  assert.equal(unknown.status, 422);
  assert.equal(unknown.received.thinking.display, 'unknown');
}));

test('Amp advertises routed native tools alongside selected MCP in the streaming catalog', () => withLocalAmp(async ({ session, model }) => {
  session.mcpConfig = { selected_graph: {} };
  const selected = { name: 'mcp__selected_graph__query' };
  const read = { name: 'Read', input_schema: { properties: { path: {}, read_range: {}, unsupported: {} } } };
  const title = { name: 'set_title' };
  const tools = [read, title, selected, { name: 'mcp__other_graph__query' }, { name: 'Glob' }];
  const response = await model({ stream: true, tools });
  assert.equal(response.status, 200);
  assert.deepEqual(response.received.tools.map(tool => tool.name), ['Read', selected.name]);
  assert.deepEqual(response.received.tools[0].input_schema, {
    properties: { path: {}, read_range: {} }, additionalProperties: false,
  });
  const auxiliary = await model({ tools });
  assert.equal(auxiliary.status, 200);
  assert.deepEqual(auxiliary.received.tools, [title, selected]);
}));

test('Amp applies native and MCP permissions independently in a mounted session', () => withLocalAmp(async ({ local, session, post, model }) => {
  session.mcpConfig = { selected_graph: {} };
  const approvals = [];
  const reads = [];
  local.agent.client.requestPermission = async ({ toolCall }) => {
    approvals.push(toolCall.toolCallId);
    return { outcome: { outcome: 'selected', optionId: toolCall.toolCallId.endsWith('allow') ? 'allow_once' : 'reject_once' } };
  };
  local.agent.client.readTextFile = async request => {
    reads.push(request);
    return { content: 'read by host' };
  };

  const name = 'mcp__selected_graph__query';
  assert.equal((await model({ stream: true, tools: [{ name }] })).status, 200);
  const native = (id, tool = 'Read') => post('/shiju/tool', {
    sessionId: 'session', id, name: tool, input: { path: 'fixture.txt' },
  });
  const mcp = (id, tool = name) => post('/shiju/mcp-permission', {
    sessionId: 'session', id, name: tool, input: {},
  });
  assert.deepEqual(await native('native-allow'), { status: 200, body: { output: '1: read by host' } });
  assert.equal((await mcp('mcp-deny')).status, 500);
  assert.equal((await native('native-deny')).status, 500);
  assert.equal((await mcp('mcp-allow')).status, 200);
  assert.equal((await native('unrouted-allow', 'Glob')).status, 500);
  assert.equal((await mcp('undeclared-allow', 'mcp__selected_graph__other')).status, 500);
  assert.deepEqual(approvals, ['native-allow', 'mcp-deny', 'native-deny', 'mcp-allow']);
  assert.deepEqual(reads, [{ sessionId: 'session', path: `${session.cwd}/fixture.txt`, line: 1, limit: 1000 }]);
}));

test('Amp scans project packages and only the validated session links', async () => {
  const { mkdtemp, mkdir, writeFile, symlink, realpath, rm } = await import('node:fs/promises');
  const { tmpdir } = await import('node:os');
  const { join } = await import('node:path');
  const { scanSkills } = await import('./local.mjs');
  const root = await realpath(await mkdtemp(join(tmpdir(), 'amp-skills-')));
  const cwd = process.cwd();
  const previous = process.env.SHIJU_AMP_SKILL_VIEW;
  const project = join(root, 'project');
  const native = join(project, '.agents/skills');
  const sources = join(root, 'sources');
  const view = join(root, 'view');
  const add = async (parent, name, disabled = false) => {
    const dir = join(parent, name);
    await mkdir(dir, { recursive: true });
    await writeFile(join(dir, 'SKILL.md'), JSON.stringify({
      frontmatter: { name, description: 'fixture', 'disable-model-invocation': disabled },
      body: 'local fixture',
    }));
    return dir;
  };

  try {
    await add(native, 'native');
    await add(native, 'disabled', true);
    const selected = await add(sources, 'selected');
    const other = await add(sources, 'other');
    await add(root, 'ancestor');
    await mkdir(view);
    const link = join(view, 'selected');
    await symlink(selected, link);
    await symlink(other, join(native, 'other'));
    await mkdir(join(native, 'external'));
    await symlink(join(other, 'SKILL.md'), join(native, 'external', 'SKILL.md'));
    process.chdir(project);
    process.env.SHIJU_AMP_SKILL_VIEW = JSON.stringify([[link, selected]]);
    assert.deepEqual((await scanSkills(JSON.parse)).map(({ name, baseDir }) => [name, baseDir]), [
      ['native', join(native, 'native')], ['selected', link],
    ]);
    await rm(link);
    await symlink(other, link);
    assert.deepEqual((await scanSkills(JSON.parse)).map(({ name }) => name), ['native']);
    process.env.SHIJU_AMP_SKILL_VIEW = 'null';
    assert.deepEqual(await scanSkills(JSON.parse), []);
  } finally {
    process.chdir(cwd);
    await rm(root, { recursive: true, force: true });
    if (previous === undefined) delete process.env.SHIJU_AMP_SKILL_VIEW;
    else process.env.SHIJU_AMP_SKILL_VIEW = previous;
  }
});

test('Amp skill activation needs a declared tool and separate host approval', () => withLocalAmp(async ({ local, post, model }) => {
  const previous = process.env.SHIJU_AMP_SKILL_VIEW;
  process.env.SHIJU_AMP_SKILL_VIEW = '[]';
  const decisions = [];
  local.agent.client.requestPermission = async ({ toolCall }) => {
    decisions.push(toolCall.toolCallId);
    return { outcome: { outcome: 'selected',
      optionId: toolCall.toolCallId === 'allowed' ? 'allow_once' : 'reject_once' } };
  };
  const skill = (id, input = { name: 'native' }) => post('/shiju/skill-permission', {
    sessionId: 'session', id, name: 'skill', input,
  });
  try {
    assert.equal((await skill('unadvertised')).status, 500);
    const response = await model({ stream: true, tools: [{ name: 'skill' }] });
    assert.deepEqual(response.received.tools, [{ name: 'skill' }]);
    assert.equal((await skill('forged', { name: '../outside' })).status, 500);
    assert.equal((await skill('denied')).status, 500);
    assert.equal((await skill('allowed')).status, 200);
    assert.deepEqual(decisions, ['denied', 'allowed']);
  } finally {
    if (previous === undefined) delete process.env.SHIJU_AMP_SKILL_VIEW;
    else process.env.SHIJU_AMP_SKILL_VIEW = previous;
  }
}));
