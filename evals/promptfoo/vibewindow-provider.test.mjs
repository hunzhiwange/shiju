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
      taskId: 'edit-1', fixtureRoot: root, fixtureFiles: ['input.txt'], timeoutMs: 1000,
      accuracyAcceptance: { admit: async ({ workRoot }) => admission(workRoot) } } });
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
    for (const result of results) if (provider.attempts.has(result.metadata?.attemptId)) await provider.cleanupAttempt(result.metadata.attemptId);
    await rm(root, { recursive: true, force: true });
  }
});

// The callbacks replace only the external environment supervisor. Artifact
// verification and evidence archival use the real filesystem and oracle.
function admission(workRoot) {
  return { workRoot, mode: 'restricted-sandbox', configurationDigest: 'b'.repeat(64),
    forbiddenWriteScopes: ['/repository'], evidencePath: '/trusted/admission.json',
    verifiedAt: '2026-10-25T00:00:00Z', digest: 'c'.repeat(64) };
}

test('accuracy_provider_verifies_artifacts_and_cleans_only_after_writer_exit', async t => {
  const { mkdtemp, writeFile, readFile, access, rm } = await import('node:fs/promises');
  const { tmpdir } = await import('node:os');
  const { join } = await import('node:path');
  const { prepare, cleanup } = await import('./attempt-workspace.mjs');
  const { loadManifest } = await import('./accuracy-manifest.mjs');
  const { evaluate } = await import('./assertions.mjs');
  const root = await mkdtemp(join(tmpdir(), 'accuracy-integration-'));
  const originalFetch = globalThis.fetch;
  t.after(async () => { globalThis.fetch = originalFetch; await rm(root, { recursive: true, force: true }); });
  await writeFile(join(root, 'answer.txt'), 'input');
  const initial = await prepare({ fixtureRoot: root, fixtureFiles: ['answer.txt'] });
  const fixtureDigest = initial.inputDigest;
  await cleanup(initial);
  const manifest = loadManifest(new URL('./accuracy-manifest.json', import.meta.url));
  manifest.contract.tasks.push({ taskId: 'edit-test', description: 'edit', category: 'edit', split: 'dev',
    fixtureDigest, oracleDigest: 'a'.repeat(64) });
  const config = { evaluationProfile: 'runtime-accuracy', manifest, taskId: 'edit-test',
    fixtureRoot: root, fixtureFiles: ['answer.txt'], timeoutMs: 1000,
    accuracyAcceptance: {
      definitions: new Map([['edit-test', { expectedFiles: { 'answer.txt': Buffer.from('correct') },
        unchangedFiles: {}, allowExtraFiles: false }]]),
      admit: async ({ workRoot }) => admission(workRoot),
      settle: async ({ admission }) => ({ writersExited: true, admissionDigest: admission.digest,
        checks: [{ kind: 'non_target_files', pass: true }] }),
    } };
  let content = 'wrong';
  let events = turnEvents({ sessionId: 'session', messageId: 'answer', output: 'pass: true', callId: 'c', toolName: 'write_file' });
  let httpError = false;
  let requests = 0;
  globalThis.fetch = async (_url, options) => {
    requests++;
    if (manifest.contract.experiment.budget.wallTimeMs === 1) await new Promise(resolve => setTimeout(resolve, 5));
    await writeFile(join(options.headers['X-VibeWindow-Directory'], 'answer.txt'), content);
    return httpError ? new Response('unavailable', { status: 503 }) : sseResponse(events);
  };
  const score = result => evaluate(result.output, { vars: { evaluationProfile: 'runtime-accuracy' }, providerResponse: result });
  const provider = new VibeWindowProvider({ config });
  const results = [];
  t.after(async () => {
    for (const result of results) if (provider.attempts.has(result.metadata?.attemptId)) await provider.cleanupAttempt(result.metadata.attemptId);
  });
  const run = async () => { const result = await provider.callApi('edit'); results.push(result); return result; };
  const wrong = await run();
  assert.equal(wrong.metadata.oracle.pass, false);
  assert.equal(score(wrong).pass, false);
  assert.equal(wrong.metadata.cleanup, 'cleaned');
  await assert.rejects(access(wrong.metadata.workRoot));
  const archived = JSON.parse(await readFile(wrong.metadata.evidencePath, 'utf8'));
  assert.equal(archived.oracle.pass, false);
  assert.equal(archived.admission.digest, 'c'.repeat(64));
  content = 'correct';
  const correct = await run();
  assert.equal(correct.metadata.oracle.pass, true);
  assert.equal(score(correct).pass, true);
  assert.equal(correct.metadata.cleanup, 'cleaned');
  // Real HTTP/SSE boundary replay: only the final answer reaches JSON acceptance.
  const fileDefinition = config.accuracyAcceptance.definitions.get('edit-test');
  config.accuracyAcceptance.definitions.set('edit-test', {
    ...fileDefinition, expectedJson: { port: 6287 },
  });
  const wrongJson = await run();
  assert.equal(wrongJson.metadata.oracle.pass, false);
  events = turnEvents({ sessionId: 'session', messageId: 'answer',
    output: '{"port":6287}', callId: 'c', toolName: 'read_file' });
  const correctJson = await run();
  assert.equal(correctJson.metadata.oracle.pass, true);
  assert.equal(score(correctJson).pass, true);
  config.accuracyAcceptance.definitions.set('edit-test', fileDefinition);
  manifest.contract.experiment.budget.wallTimeMs = 1;
  const overBudget = await run();
  assert.equal(score(overBudget).namedScores.execution_budget, 0);
  manifest.contract.experiment.budget.wallTimeMs = 900000;
  const definitions = config.accuracyAcceptance.definitions;
  delete config.accuracyAcceptance.definitions;
  const missing = await run();
  assert.equal(missing.metadata.oracle.pass, false);
  assert.equal(score(missing).pass, false);
  config.accuracyAcceptance.definitions = new Map([['unknown', definitions.get('edit-test')]]);
  const unknown = await run();
  assert.equal(unknown.metadata.oracle.pass, false);
  assert.equal(unknown.metadata.cleanup, 'cleaned');
  config.accuracyAcceptance.definitions = definitions;
  httpError = true;
  const completedHttpError = await run();
  assert.match(completedHttpError.error, /503/);
  assert.equal(completedHttpError.metadata.cleanup, 'cleaned');
  assert.equal(score(completedHttpError).pass, false);
  httpError = false;
  config.accuracyAcceptance.settle = async () => ({ writersExited: false });
  const pending = await run();
  assert.equal(pending.metadata.cleanup, 'cleanup_pending');
  assert.equal(await readFile(join(pending.metadata.workRoot, 'answer.txt'), 'utf8'), 'correct');
  assert.equal(pending.metadata.oracle.pass, false);
  config.accuracyAcceptance.settle = async () => { throw new Error('supervisor unavailable'); };
  httpError = true;
  const failed = await run();
  assert.match(failed.error, /503/);
  assert.equal(failed.metadata.cleanup, 'cleanup_pending');
  assert.equal(await readFile(join(failed.metadata.workRoot, 'answer.txt'), 'utf8'), 'correct');
  config.accuracyAcceptance.settle = async ({ admission }) => ({ writersExited: true,
    admissionDigest: admission.digest, checks: [{ kind: 'non_target_files', pass: false }] });
  httpError = false;
  const boundary = await run();
  assert.equal(boundary.metadata.oracle.pass, true);
  assert.equal(score(boundary).pass, false);
  assert.equal(score(boundary).namedScores.boundary_checks, 0);
  events = events.filter(event => event.type !== 'chat.done');
  const truncated = await run();
  assert.equal(score(truncated).pass, false);
  assert.equal(score(truncated).namedScores.result_correct, 1);
  assert.equal(score(truncated).namedScores.normal_terminal, 0);
  // A failed archive must preserve both the workspace and existing evidence.
  config.accuracyAcceptance.admit = async ({ workRoot, attemptId }) => {
    await writeFile(new URL(`./results/accuracy-evidence/${attemptId}.json`, import.meta.url), 'archive canary');
    return admission(workRoot);
  };
  const archiveFailure = await run();
  assert.equal(archiveFailure.metadata.cleanup, 'cleanup_pending');
  assert.equal(await readFile(join(archiveFailure.metadata.workRoot, 'answer.txt'), 'utf8'), 'correct');
  assert.equal(await readFile(new URL(`./results/accuracy-evidence/${archiveFailure.metadata.attemptId}.json`, import.meta.url), 'utf8'), 'archive canary');
  delete config.accuracyAcceptance.admit;
  const denied = await run();
  assert.match(denied.error, /admission/);
  config.accuracyAcceptance.admit = async ({ workRoot }) => ({ ...admission(workRoot), workRoot: root });
  const invalidProof = await run();
  assert.match(invalidProof.error, /admission evidence invalid/);
  assert.equal(invalidProof.metadata.cleanup, 'cleaned');
  assert.equal(await readFile(join(root, 'answer.txt'), 'utf8'), 'input');
  assert.equal(requests, 13);
});

test('accuracy_recovery_uses_sse_exit_code_and_independent_file_oracle', async t => {
  const { mkdtemp, writeFile, rm } = await import('node:fs/promises');
  const { tmpdir } = await import('node:os');
  const { join } = await import('node:path');
  const { prepare, cleanup } = await import('./attempt-workspace.mjs');
  const { loadManifest } = await import('./accuracy-manifest.mjs');
  const { evaluate } = await import('./assertions.mjs');
  const root = await mkdtemp(join(tmpdir(), 'recovery-'));
  const originalFetch = globalThis.fetch;
  t.after(async () => { globalThis.fetch = originalFetch; await rm(root, { recursive: true, force: true }); });
  await writeFile(join(root, 'answer.txt'), 'input');
  const initial = await prepare({ fixtureRoot: root, fixtureFiles: ['answer.txt'] });
  const fixtureDigest = initial.inputDigest;
  await cleanup(initial);
  const manifest = loadManifest(new URL('./accuracy-manifest.json', import.meta.url));
  manifest.contract.tasks.push({ taskId: 'recovery-test', description: 'recover', category: 'recovery', split: 'dev',
    fixtureDigest, oracleDigest: 'a'.repeat(64) });
  const definition = { expectedFiles: { 'answer.txt': Buffer.from('correct') }, unchangedFiles: {}, allowExtraFiles: false,
    expectedFailure: { toolName: 'bash', operation: 'node probe.mjs', exitCode: 2, recoveryOperation: 'node repair.mjs' } };
  const config = { evaluationProfile: 'runtime-accuracy', manifest, taskId: 'recovery-test', fixtureRoot: root,
    fixtureFiles: ['answer.txt'], timeoutMs: 1000, accuracyAcceptance: {
      definitions: new Map([['recovery-test', definition]]),
      admit: async ({ workRoot }) => admission(workRoot),
      settle: async ({ admission }) => ({ writersExited: true, admissionDigest: admission.digest,
        checks: [{ kind: 'non_target_files', pass: true }] }),
    } };
  let content = 'correct';
  let exitCode = 2;
  let duplicateFailure = false;
  globalThis.fetch = async (_url, options) => {
    await writeFile(join(options.headers['X-VibeWindow-Directory'], 'answer.txt'), content);
    const events = [];
    for (const [index, command, code] of [[0, 'node probe.mjs', exitCode], [1, 'node repair.mjs', 0]]) {
      events.push({ type: 'chat.tool', kind: 'start', status: 'running', call_id: String(index), name: 'bash',
        step_index: index, occurred_ms: index * 3 + 1, payload: { input: { command } } },
      { type: 'chat.tool', kind: 'complete', status: index === 0 ? 'failed' : 'completed', call_id: String(index), name: 'bash',
        step_index: index, occurred_ms: index * 3 + 2, payload: { metadata: { exit_code: code }, output: 'exit_code=2' } });
    }
    if (duplicateFailure) events.splice(2, 0, structuredClone(events[1]));
    events.push({ type: 'chat.delta', delta: 'recovered' }, { type: 'chat.done', session_id: 'session', message_id: 'answer' });
    return sseResponse(events);
  };
  const provider = new VibeWindowProvider({ config });
  const run = async () => provider.callApi('recover', { vars: { expectedFailure: { exitCode: 1 } } });
  const score = result => evaluate(result.output, { providerResponse: result });
  const correct = await run();
  assert.equal(correct.metadata.toolCalls[0].exitCode, 2);
  assert.equal(correct.metadata.expectedFailure.exitCode, 2);
  assert.equal(correct.metadata.toolCalls.length, 2);
  assert.equal(score(correct).pass, true);
  content = 'wrong';
  assert.equal(score(await run()).pass, false);
  content = 'correct';
  exitCode = 1;
  assert.equal(score(await run()).pass, false);
  exitCode = undefined;
  assert.equal(score(await run()).pass, false);
  exitCode = 2;
  duplicateFailure = true;
  assert.equal(score(await run()).pass, false);
  duplicateFailure = false;
  manifest.contract.experiment.budget.toolCalls = 1;
  assert.equal(score(await run()).pass, false);
});
