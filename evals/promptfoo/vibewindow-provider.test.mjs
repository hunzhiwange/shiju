import assert from 'node:assert/strict';
import test from 'node:test';

import VibeWindowProvider from './vibewindow-provider.mjs';

function sseResponse(events) {
  const body = events.map((event) => `data: ${JSON.stringify(event)}\n\n`).join('');
  return new Response(body, {
    status: 200,
    headers: { 'Content-Type': 'text/event-stream' },
  });
}

function turnEvents({ sessionId, messageId, output, callId, toolName }) {
  return [
    { type: 'chat.delta', delta: output },
    {
      type: 'chat.tool',
      kind: 'start',
      status: 'running',
      call_id: callId,
      name: toolName,
      step_index: 0,
      occurred_ms: 1,
      payload: { input: JSON.stringify({ value: toolName }) },
    },
    {
      type: 'chat.tool',
      kind: 'complete',
      status: 'completed',
      call_id: callId,
      name: toolName,
      step_index: 0,
      occurred_ms: 2,
      payload: { output: `${toolName} result` },
    },
    {
      type: 'chat.done',
      session_id: sessionId,
      message_id: messageId,
      usage: { input_tokens: 2, output_tokens: 3 },
    },
  ];
}

test('provider_reuses_parent_session_and_keeps_turn_tool_traces', async () => {
  const originalFetch = globalThis.fetch;
  const requests = [];
  const responses = [
    turnEvents({
      sessionId: 'parent-session',
      messageId: 'assistant-1',
      output: 'first answer',
      callId: 'call-1',
      toolName: 'create_thread',
    }),
    turnEvents({
      sessionId: 'parent-session',
      messageId: 'assistant-2',
      output: 'second answer',
      callId: 'call-2',
      toolName: 'list_threads',
    }),
  ];
  globalThis.fetch = async (_url, options) => {
    requests.push(JSON.parse(options.body));
    return sseResponse(responses.shift());
  };

  try {
    const provider = new VibeWindowProvider({ config: { timeoutMs: 1_000 } });
    const result = await provider.callApi('first prompt', {
      vars: { turns: ['second prompt'] },
    });

    assert.equal(requests.length, 2);
    assert.deepEqual(requests[0].messages, [{ role: 'user', content: 'first prompt' }]);
    assert.equal(requests[0].session_id, undefined);
    assert.equal(requests[1].session_id, 'parent-session');
    assert.deepEqual(requests[1].messages, [
      { role: 'user', content: 'first prompt' },
      { role: 'assistant', content: 'first answer' },
      { role: 'user', content: 'second prompt' },
    ]);
    assert.equal(result.output, 'second answer');
    assert.equal(result.sessionId, 'parent-session');
    assert.deepEqual(
      result.metadata.toolCalls.map(({ name }) => name),
      ['create_thread', 'list_threads'],
    );
    assert.deepEqual(
      result.metadata.turns.map((turn) => ({
        prompt: turn.prompt,
        output: turn.output,
        toolNames: turn.metadata.toolCalls.map(({ name }) => name),
      })),
      [
        { prompt: 'first prompt', output: 'first answer', toolNames: ['create_thread'] },
        { prompt: 'second prompt', output: 'second answer', toolNames: ['list_threads'] },
      ],
    );
    assert.deepEqual(result.tokenUsage, { total: 10, prompt: 4, completion: 6 });
  } finally {
    globalThis.fetch = originalFetch;
  }
});

test('provider_keeps_single_turn_output_shape', async () => {
  const originalFetch = globalThis.fetch;
  let requestDirectory;
  globalThis.fetch = async (_url, options) => {
    requestDirectory = options.headers['X-VibeWindow-Directory'];
    return sseResponse(
      turnEvents({
        sessionId: 'single-session',
        messageId: 'assistant-1',
        output: 'single answer',
        callId: 'call-1',
        toolName: 'read_thread',
      }),
    );
  };

  try {
    const provider = new VibeWindowProvider({ config: { timeoutMs: 1_000, directory: '/tmp/legacy-eval' } });
    const result = await provider.callApi('single prompt');
    assert.equal(requestDirectory, process.env.VIBEWINDOW_EVAL_DIRECTORY ?? '/tmp/legacy-eval');

    assert.equal(result.output, 'single answer');
    assert.equal(result.prompt, 'single prompt');
    assert.equal(result.sessionId, 'single-session');
    assert.equal(result.metadata.toolCalls[0].name, 'read_thread');
    assert.equal(Object.hasOwn(result.metadata, 'turns'), false);
  } finally {
    globalThis.fetch = originalFetch;
  }
});

test('provider_keeps_failed_tool_error_detail', async () => {
  const originalFetch = globalThis.fetch;
  globalThis.fetch = async () =>
    sseResponse([
      {
        type: 'chat.tool',
        kind: 'complete',
        status: 'failed',
        call_id: 'call-handoff',
        name: 'handoff_thread',
        step_index: 0,
        occurred_ms: 1,
        payload: {
          status: 'error',
          input: JSON.stringify({ thread_id: 'thread-1' }),
          error: 'handoff_thread is unsupported',
        },
      },
      {
        type: 'chat.delta',
        delta: 'handoff is unsupported',
      },
      {
        type: 'chat.done',
        session_id: 'parent-session',
        message_id: 'assistant-1',
      },
    ]);

  try {
    const provider = new VibeWindowProvider({ config: { timeoutMs: 1_000 } });
    const result = await provider.callApi('handoff');

    assert.equal(result.metadata.toolCalls[0].isError, true);
    assert.equal(result.metadata.toolCalls[0].error, 'handoff_thread is unsupported');
  } finally {
    globalThis.fetch = originalFetch;
  }
});

test('provider_stops_followups_when_a_turn_fails', async () => {
  const originalFetch = globalThis.fetch;
  let requestCount = 0;
  globalThis.fetch = async () => {
    requestCount += 1;
    return sseResponse([{ type: 'chat.error', error: 'turn failed' }]);
  };

  try {
    const provider = new VibeWindowProvider({ config: { timeoutMs: 1_000 } });
    const result = await provider.callApi('first prompt', {
      vars: { turns: ['must not run'] },
    });

    assert.equal(requestCount, 1);
    assert.equal(result.error, 'turn failed');
  } finally {
    globalThis.fetch = originalFetch;
  }
});

test('provider_isolates_attempt_directories_and_keeps_turn_session', async () => {
  const { mkdtemp, writeFile, readFile, rm } = await import('node:fs/promises');
  const { tmpdir } = await import('node:os');
  const { join } = await import('node:path');
  const { prepare, cleanup } = await import('./attempt-workspace.mjs');
  const { loadManifest, manifestDigest } = await import('./accuracy-manifest.mjs');
  const root = await mkdtemp(join(tmpdir(), 'provider-fixture-'));
  const originalFetch = globalThis.fetch;
  const results = [];
  let provider;
  try {
    await writeFile(join(root, 'input.txt'), 'public input');
    const workspace = await prepare({ fixtureRoot: root, fixtureFiles: ['input.txt'] });
    const fixtureDigest = workspace.inputDigest;
    await cleanup(workspace);
    const manifest = loadManifest(new URL('./accuracy-manifest.json', import.meta.url));
    manifest.contract.tasks.push({ taskId: 'edit-1', description: 'edit input', category: 'edit',
      split: 'dev', fixtureDigest, oracleDigest: 'a'.repeat(64) });
    provider = new VibeWindowProvider({ config: { evaluationProfile: 'runtime-accuracy', manifest,
      taskId: 'edit-1', fixtureRoot: root, fixtureFiles: ['input.txt'], timeoutMs: 1000 } });
    const requests = [];
    globalThis.fetch = async (_url, options) => {
      requests.push({ body: JSON.parse(options.body), directory: options.headers['X-VibeWindow-Directory'] });
      if (requests.length === 4) return new Response('unavailable', { status: 503 });
      if (requests.length === 5) return sseResponse([
        { type: 'chat.delta', delta: 'partial' }, { type: 'chat.error', error: 'stream failed' },
      ]);
      return sseResponse(turnEvents({ sessionId: `session-${Math.ceil(requests.length / 2)}`,
        messageId: 'answer', output: 'ok', callId: 'call', toolName: 'read_file' }));
    };
    for (let i = 0; i < 3; i++) results.push(await provider.callApi('edit', {
      vars: { turns: ['continue'], taskId: 'spoof', split: 'holdout', manifestDigest: 'spoof' },
    }));
    assert.equal(requests.length, 5);
    assert.equal(requests[0].body.session_id, undefined);
    assert.equal(requests[2].body.session_id, undefined);
    assert.equal(requests[1].body.session_id, 'session-1');
    assert.equal(requests[3].body.session_id, 'session-2');
    assert.equal(requests[0].directory, requests[1].directory);
    assert.equal(requests[2].directory, requests[3].directory);
    assert.notEqual(requests[0].directory, requests[2].directory);
    for (const [i, result] of results.entries()) {
      assert.equal(result.metadata.taskId, 'edit-1');
      assert.equal(result.metadata.split, 'dev');
      assert.equal(result.metadata.manifestDigest, manifestDigest(manifest));
      assert.equal(result.metadata.fixtureDigest, fixtureDigest);
      assert.equal(result.metadata.expectedAttempts, 3);
      assert.equal(result.metadata.workRoot, requests[i * 2].directory);
      assert.equal(await readFile(join(result.metadata.workRoot, 'input.txt'), 'utf8'), 'public input');
      assert.ok(result.metadata.attemptId);
    }
    assert.notEqual(results[0].metadata.attemptId, results[1].metadata.attemptId);
    assert.match(results[1].error, /503/);
    assert.equal(results[1].metadata.failureKind, 'http');
    assert.equal(results[1].metadata.turns[0].metadata.toolCalls[0].name, 'read_file');
    assert.equal(results[1].metadata.events.length, 4);
    assert.equal(results[2].metadata.failureKind, 'stream');
    assert.equal(results[2].metadata.events[0].delta, 'partial');
    assert.equal(results[2].metadata.events[1].error, 'stream failed');
    provider.config.manifest.contract.tasks[0].fixtureDigest = 'b'.repeat(64);
    results.push(await provider.callApi('must not run'));
    assert.match(results[3].error, /fixtureDigest mismatch/);
    assert.equal(results[3].metadata.failureKind, 'configuration');
    assert.equal(requests.length, 5);
  } finally {
    globalThis.fetch = originalFetch;
    for (const result of results) if (result.metadata?.attemptId) await provider.cleanupAttempt(result.metadata.attemptId);
    await rm(root, { recursive: true, force: true });
  }
});
