import assert from 'node:assert/strict';
import path from 'node:path';
import test from 'node:test';
import { fileURLToPath } from 'node:url';

import VibeWindowProvider, {
  aggregateEvents,
  parseSseStream,
  resolveTimeoutMs,
} from './vibewindow-provider.mjs';

const encoder = new TextEncoder();

test('timeout configuration uses a fifteen minute default and preserves valid overrides', () => {
    assert.equal(resolveTimeoutMs({}, {}), 900_000);
  assert.equal(resolveTimeoutMs({ timeoutMs: 240_000 }, {}), 240_000);
    assert.equal(resolveTimeoutMs({ timeoutMs: 0 }, {}), 900_000);
  assert.equal(resolveTimeoutMs({}, { VIBEWINDOW_EVAL_TIMEOUT_MS: '300000' }), 300_000);
    assert.equal(resolveTimeoutMs({}, { VIBEWINDOW_EVAL_TIMEOUT_MS: 'invalid' }), 900_000);
});

function responseFromChunks(chunks, init = {}) {
  const body = new ReadableStream({
    start(controller) {
      for (const chunk of chunks) {
        controller.enqueue(encoder.encode(chunk));
      }
      controller.close();
    },
  });
  return new Response(body, {
    status: init.status ?? 200,
    headers: {
      'content-type': 'text/event-stream',
      ...init.headers,
    },
  });
}

test('parseSseStream handles split frames, comments, and multiline data', async () => {
  const response = responseFromChunks([
    ': keep-alive\n\nid: 1\nda',
    'ta: {"type":"chat.delta",\n',
    'data: "delta":"ok"}\n\n',
    'id: 2\ndata: {"type":"chat.done","session_id":"session-1"}\n\n',
  ]);

  const events = [];
  for await (const event of parseSseStream(response.body)) {
    events.push(event);
  }

  assert.deepEqual(events, [
    { type: 'chat.delta', delta: 'ok' },
    { type: 'chat.done', session_id: 'session-1' },
  ]);
});

test('aggregateEvents returns output, usage, steps, and merged tool calls', () => {
  const result = aggregateEvents([
    { type: 'chat.context_loaded', paths: ['/workspace/AGENTS.md'] },
    { type: 'chat.step_start', step_index: 0, created_ms: 10, model: 'provider/model' },
    {
      type: 'chat.tool',
      step_index: 0,
      call_id: 'call-1',
      name: 'shell',
      kind: 'start',
      status: 'running',
      occurred_ms: 20,
      payload: { status: 'running', input: '{"command":"pwd"}' },
    },
    {
      type: 'chat.tool',
      step_index: 0,
      call_id: 'call-1',
      name: 'shell',
      kind: 'complete',
      status: 'completed',
      occurred_ms: 30,
      payload: {
        status: 'completed',
        input: '{"command":"pwd"}',
        output: '/workspace',
      },
    },
    { type: 'chat.delta', delta: '调用者是 runner。' },
    {
      type: 'chat.step_finish',
      step_index: 0,
      finished_ms: 40,
      usage: {
        input_tokens: 100,
        output_tokens: 20,
        cached_tokens: 60,
        reasoning_tokens: 5,
      },
      finish_reason: 'stop',
      model: 'provider/model',
    },
    {
      type: 'chat.done',
      session_id: 'session-1',
      message_id: 'message-1',
      usage: {
        input_tokens: 100,
        output_tokens: 20,
        cached_tokens: 60,
        reasoning_tokens: 5,
      },
    },
  ]);

  assert.equal(result.output, '调用者是 runner。');
  assert.deepEqual(result.tokenUsage, { total: 120, prompt: 100, completion: 20 });
  assert.equal(result.finishReason, 'stop');
  assert.equal(result.sessionId, 'session-1');
  assert.deepEqual(result.metadata.contextPaths, ['/workspace/AGENTS.md']);
  assert.deepEqual(result.metadata.usage, {
    inputTokens: 100,
    outputTokens: 20,
    cachedTokens: 60,
    reasoningTokens: 5,
  });
  assert.deepEqual(result.metadata.steps, [
    {
      index: 0,
      startedMs: 10,
      finishedMs: 40,
      model: 'provider/model',
      finishReason: 'stop',
      usage: {
        inputTokens: 100,
        outputTokens: 20,
        cachedTokens: 60,
        reasoningTokens: 5,
      },
    },
  ]);
  assert.deepEqual(result.metadata.toolCalls, [
    {
      id: 'call-1',
      name: 'shell',
      input: { command: 'pwd' },
      output: '/workspace',
      isError: false,
      status: 'completed',
      stepIndex: 0,
      startedMs: 20,
      finishedMs: 30,
    },
  ]);
});

test('aggregateEvents rejects explicit errors and streams without a terminal event', () => {
  assert.throws(
    () => aggregateEvents([{ type: 'chat.error', error: 'provider failed' }]),
    /provider failed/,
  );
  assert.throws(
    () => aggregateEvents([{ type: 'chat.delta', delta: 'partial' }]),
    /closed before chat.done/,
  );
});

test('provider sends the configured request and returns Promptfoo metadata', async (t) => {
  const originalFetch = globalThis.fetch;
  t.after(() => {
    globalThis.fetch = originalFetch;
  });

  let captured;
  globalThis.fetch = async (url, options) => {
    captured = { url, options };
    return responseFromChunks([
      'data: {"type":"chat.step_start","step_index":0,"created_ms":10,"model":"test/model"}\n\n',
      'data: {"type":"chat.delta","delta":"answer"}\n\n',
      'data: {"type":"chat.step_finish","step_index":0,"finished_ms":20,"usage":{"input_tokens":3,"output_tokens":2,"cached_tokens":0,"reasoning_tokens":0},"finish_reason":"stop","model":"test/model"}\n\n',
      'data: {"type":"chat.done","session_id":"session-1","usage":{"input_tokens":3,"output_tokens":2,"cached_tokens":0,"reasoning_tokens":0}}\n\n',
    ]);
  };

  const provider = new VibeWindowProvider({
    id: 'vibewindow-test',
    config: {
      endpoint: 'http://127.0.0.1:9999/v1/chat/stream',
      directory: '/workspace',
      model: 'default/model',
      agent: 'main',
      platformId: 'platform',
      companyId: 'company',
      timeoutMs: 5000,
    },
  });

  const result = await provider.callApi('find callers', {
    vars: { model: 'test/model', agent: 'reviewer' },
  });

  assert.equal(provider.id(), 'vibewindow-test');
  assert.equal(captured.url, 'http://127.0.0.1:9999/v1/chat/stream');
  assert.equal(captured.options.headers['X-Platform-ID'], 'platform');
  assert.equal(captured.options.headers['X-Company-ID'], 'company');
  assert.equal(captured.options.headers['X-VibeWindow-Directory'], '/workspace');
  assert.deepEqual(JSON.parse(captured.options.body), {
    model: 'test/model',
    agent: 'reviewer',
    options: { max_output_tokens: 0 },
    messages: [{ role: 'user', content: 'find callers' }],
  });
  assert.equal(result.output, 'answer');
  assert.equal(result.sessionId, 'session-1');
  assert.deepEqual(result.tokenUsage, { total: 5, prompt: 3, completion: 2 });
  assert.equal(result.metadata.model, 'test/model');
  assert.equal(result.metadata.agent, 'reviewer');
  assert.equal(typeof result.latencyMs, 'number');
});

test('provider surfaces bounded HTTP errors without leaking request headers', async (t) => {
  const originalFetch = globalThis.fetch;
  const originalModel = process.env.VIBEWINDOW_EVAL_MODEL;
  delete process.env.VIBEWINDOW_EVAL_MODEL;
  t.after(() => {
    globalThis.fetch = originalFetch;
    if (originalModel === undefined) {
      delete process.env.VIBEWINDOW_EVAL_MODEL;
    } else {
      process.env.VIBEWINDOW_EVAL_MODEL = originalModel;
    }
  });

  let requestBody;
  globalThis.fetch = async (_url, options) => {
    requestBody = JSON.parse(options.body);
    assert.equal(
      options.headers['X-VibeWindow-Directory'],
      path.resolve(fileURLToPath(new URL('../..', import.meta.url))),
    );
    return new Response('bad gateway'.repeat(200), {
      status: 502,
      statusText: 'Bad Gateway',
    });
  };

  const provider = new VibeWindowProvider({
    config: {
      directory: '.',
      platformId: 'secret-platform',
      companyId: 'secret-company',
    },
  });
  const result = await provider.callApi('prompt');

  assert.match(result.error, /502 Bad Gateway/);
  assert.equal(requestBody.model, 'token-plan/deepseek-v4-flash-0731');
  assert.ok(result.error.length < 1300);
  assert.doesNotMatch(result.error, /secret-platform|secret-company/);
});
