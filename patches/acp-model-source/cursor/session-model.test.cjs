const { test } = require('node:test');
const assert = require('node:assert/strict');
const { validateConfig, parseStream, history, shellArgs } = require('./session-model.cjs');
const config = { endpoint: 'http://127.0.0.1:1234/', model: 'selected', token: 'fixture', outputLimit: 1000 };
const event = (delta, finish_reason) => 'data: ' + JSON.stringify({ choices: [{ delta, finish_reason }] }) + '\n\n';
async function collect(text, tools) {
  // Byte-wise delivery includes split UTF-8 and SSE boundaries.
  const body = (async function* () { for (const byte of Buffer.from(text)) yield Buffer.from([byte]); })();
  const result = []; for await (const item of parseStream(body, tools)) result.push(item); return result;
}
test('binding rejects non-loopback and credential-bearing endpoints', () => {
  assert.equal(validateConfig(config), config);
  for (const endpoint of ['https://127.0.0.1:1234/', 'http://example.com:1234/', 'http://user@127.0.0.1:1234/', 'http://127.0.0.1:1234/a', 'http://127.0.0.1:1234/?x=1'])
    assert.throws(() => validateConfig({ ...config, endpoint }), /loopback/);
});
test('stream retains Chinese and reasoning and validates complete tool arguments', async () => {
  const chunks = await collect(event({ content: '你好', reasoning_content: '推理', tool_calls: [{ index: 0, id: 'call', type: 'function', function: { name: 'shell', arguments: '{"command":' } }] }) +
    event({ tool_calls: [{ index: 0, function: { arguments: '"pwd"}' } }] }, 'tool_calls') + 'data: [DONE]\n\n');
  assert.equal(chunks.at(-1).content, '你好'); assert.equal(chunks.at(-1).reasoning, '推理');
  assert.equal(shellArgs(chunks.at(-1).call, '/workspace').command, 'pwd');
});
test('truncated, failed, parallel, unknown and length-limited completions fail closed', async () => {
  for (const s of [event({ content: 'partial' }, 'stop'), event({}, 'length') + 'data: [DONE]\n\n',
    'data: {"error":{"message":"private fixture"}}\n\n',
    event({ tool_calls: [{ index: 1 }] }, 'tool_calls') + 'data: [DONE]\n\n',
    event({ tool_calls: [{ index: 0, id: 'x', function: { name: 'unknown', arguments: '{}' } }] }, 'tool_calls') + 'data: [DONE]\n\n'])
    await assert.rejects(collect(s), error => !error.message.includes('private fixture'));
});
test('history preserves reasoning and rejects incomplete native tools', () => {
  const toolCall = { toolCallId: 'x', tool: { case: 'shellToolCall', value: { args: { command: 'pwd' }, result: { result: { case: 'success' }, toJson: () => ({ success: { stdout: '/workspace' } }) } } } };
  const conversation = { turns: [{ turn: { case: 'agentConversationTurn', value: { userMessage: { text: 'hello' }, steps: [
    { message: { case: 'thinkingMessage', value: { text: 'reason' } } }, { message: { case: 'toolCall', value: toolCall } },
  ] } } }] };
  const messages = history(conversation);
  assert.equal(messages[1].reasoning_content, 'reason'); assert.equal(messages[2].tool_call_id, 'x');
  toolCall.tool.value.result = undefined;
  assert.throws(() => history(conversation), /incomplete/);
});
test('history replays the selected MCP tool and native result', () => {
  const args = {
    name: 'ruejs_graph_probe_rustcodegraph_query',
    toolName: 'rustcodegraph_query',
    providerIdentifier: 'ruejs_graph_probe',
    args: { search: 'createRueSiteApp' },
  };
  const result = { success: { content: [{ text: { text: 'app/app.tsx:32' } }] } };
  const toolCall = { toolCallId: 'graph', tool: { case: 'mcpToolCall', value: {
    args: { toJson: () => args },
    result: { result: { case: 'success' }, toJson: () => result },
  } } };
  const conversation = { turns: [{ turn: { case: 'agentConversationTurn', value: {
    userMessage: { text: 'Locate createRueSiteApp' },
    steps: [{ message: { case: 'toolCall', value: toolCall } }],
  } } }] };
  const messages = history(conversation);
  assert.deepEqual(messages[1].tool_calls, [{ id: 'graph', type: 'function', function: {
    name: args.name, arguments: JSON.stringify(args.args),
  } }]);
  assert.deepEqual(messages[2], { role: 'tool', tool_call_id: 'graph', content: JSON.stringify(result) });
});
test('stream accepts only mounted MCP tools with object arguments', async () => {
  const name = 'ruejs_graph_probe-rustcodegraph_query';
  const mounted = new Map([[name, { target: { name, toolName: 'rustcodegraph_query' } }]]);
  const completion = args => event({ tool_calls: [{
    index: 0, id: 'mcp-1', type: 'function', function: { name, arguments: args },
  }] }, 'tool_calls') + 'data: [DONE]\n\n';
  const args = '{"search":"createRueSiteApp"}';
  assert.equal((await collect(completion(args), mounted)).at(-1).call.name, name);
  await assert.rejects(collect(completion(args)), /unsupported returned tool/);
  await assert.rejects(collect(completion('[]'), mounted), /invalid MCP tool arguments/);
});
