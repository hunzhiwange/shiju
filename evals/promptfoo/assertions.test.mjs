import assert from 'node:assert/strict';
import test from 'node:test';

import { evaluate, noFailedTools } from './assertions.mjs';

const THREAD_ID = 'thread-child-1';
const PROJECT_ID = 'project-current';

function call(name, input, output, overrides = {}) {
  return {
    name,
    input,
    output: typeof output === 'string' ? output : JSON.stringify(output),
    status: 'completed',
    isError: false,
    ...overrides,
  };
}

function handoffCall() {
  return call(
    'handoff_thread',
    { thread_id: THREAD_ID },
    'handoff_thread is unsupported: this runtime cannot transfer threads between hosts or worktrees',
    { status: 'failed', isError: true },
  );
}

function completeTrajectory() {
  return [
    call(
      'create_thread',
      { prompt: '写一篇50字的科幻小说.' },
      { thread_id: THREAD_ID, project_id: PROJECT_ID, state: 'idle' },
    ),
    call('list_threads', {}, {
      threads: [{ thread_id: THREAD_ID, project_id: PROJECT_ID, parent_thread_id: 'parent-1' }],
    }),
    call('read_thread', { thread_id: THREAD_ID }, { thread_id: THREAD_ID, messages: [] }),
    call('wait_threads', { thread_ids: [THREAD_ID] }, { threads: [{ thread_id: THREAD_ID }] }),
    call(
      'send_message_to_thread',
      { thread_id: THREAD_ID, prompt: '继续' },
      { thread_id: THREAD_ID, state: 'idle' },
    ),
    call(
      'fork_thread',
      { thread_id: THREAD_ID },
      { thread_id: 'thread-fork-1', project_id: PROJECT_ID, parent_thread_id: 'parent-1' },
    ),
    call(
      'set_thread_title',
      { thread_id: THREAD_ID, title: '科幻小说' },
      { thread_id: THREAD_ID, project_id: PROJECT_ID, title: '科幻小说' },
    ),
    call(
      'set_thread_pinned',
      { thread_id: THREAD_ID, pinned: true },
      { thread_id: THREAD_ID, project_id: PROJECT_ID, pinned: true },
    ),
    call(
      'set_thread_archived',
      { thread_id: THREAD_ID, archived: true },
      { thread_id: THREAD_ID, project_id: PROJECT_ID, archived: true },
    ),
    handoffCall(),
  ];
}

function context(calls) {
  return {
    vars: { evaluationProfile: 'thread-commands' },
    providerResponse: {
      metadata: { eventTypes: ['chat.done'], toolCalls: calls },
    },
  };
}

function threadScore(calls, output = '已执行 thread 命令') {
  return evaluate(output, context(calls));
}

test('thread_profile_scores_real_calls_and_expected_handoff_rejection', () => {
  const result = threadScore(completeTrajectory());

  assert.equal(result.pass, true, result.reason);
  assert.equal(result.namedScores.thread_trajectory, 1);
  assert.match(
    result.componentResults.find((component) => component.namedScores.thread_trajectory)?.reason ?? '',
    /10\/10/,
  );
});

test('thread_profile_rejects_missing_alias_and_answer_only_claims', () => {
  const missing = threadScore(completeTrajectory().filter((tool) => tool.name !== 'read_thread'));
  assert.equal(missing.pass, false);

  const alias = completeTrajectory();
  alias.find((tool) => tool.name === 'read_thread').name = 'read_session';
  assert.equal(threadScore(alias).pass, false);

  assert.equal(
    threadScore([], 'create_thread list_threads read_thread wait_threads 全部成功').pass,
    false,
  );
});

test('thread_profile_rejects_failed_supported_call_and_wrong_target_scope', () => {
  const failed = completeTrajectory();
  Object.assign(failed.find((tool) => tool.name === 'set_thread_title'), {
    status: 'failed',
    isError: true,
  });
  assert.equal(threadScore(failed).pass, false);

  const wrongTarget = completeTrajectory();
  wrongTarget.find((tool) => tool.name === 'read_thread').input.thread_id = 'unrelated-thread';
  assert.equal(threadScore(wrongTarget).pass, false);
});

test('thread_profile_rejects_wrong_create_or_list_result_and_false_handoff_success', () => {
  const wrongPrompt = completeTrajectory();
  wrongPrompt[0].input.prompt = '写一篇科幻小说';
  assert.equal(threadScore(wrongPrompt).pass, false);

  const missingFromList = completeTrajectory();
  missingFromList.find((tool) => tool.name === 'list_threads').output = '{"threads":[]}';
  assert.equal(threadScore(missingFromList).pass, false);

  const handoffSucceeded = completeTrajectory();
  handoffSucceeded.splice(-1, 1, call('handoff_thread', { thread_id: THREAD_ID }, {}));
  assert.equal(threadScore(handoffSucceeded).pass, false);
});

test('accuracy_profile_requires_evaluator_oracle', () => {
  const metadata = { evaluationProfile: 'runtime-accuracy', eventTypes: ['chat.done'],
    terminal: { pass: true }, budget: { pass: true },
    boundary: { pass: true, admissionDigest: 'a'.repeat(64) } };
  const score = () => evaluate('pass: true; completed', {
    vars: { evaluationProfile: 'runtime-accuracy' }, providerResponse: { metadata },
  });
  assert.equal(score().pass, false);
  assert.equal(score().namedScores.result_correct, 0);
  const spoofed = evaluate('completed', { vars: { evaluationProfile: 'coding' },
    providerResponse: { metadata } });
  assert.equal(spoofed.namedScores.result_correct, 0);
  metadata.oracle = { pass: false, checks: [{ pass: false }], reason: 'wrong bytes' };
  assert.equal(score().pass, false);
  metadata.oracle = { pass: true, checks: [{ pass: true }], reason: 'verified' };
  assert.equal(score().pass, true);
  delete metadata.boundary.admissionDigest;
  assert.equal(score().pass, false);
  assert.equal(score().namedScores.boundary_checks, 0);
});

test('accuracy_recovery_rejects_unexpected_tool_failure', () => {
  const metadata = { evaluationProfile: 'runtime-accuracy', terminal: { pass: true },
    budget: { pass: true }, boundary: { pass: true, admissionDigest: 'a'.repeat(64) },
    oracle: { pass: true, checks: [{ pass: true }] },
    expectedFailure: { toolName: 'bash', operation: 'node probe.mjs', exitCode: 2,
      recoveryOperation: 'node repair.mjs' },
    toolCalls: [call('bash', { command: 'node probe.mjs' }, 'exit 2',
      { status: 'failed', isError: true, exitCode: 2, startedMs: 1, finishedMs: 2, startEventIndex: 0, completeEventIndex: 1, completeEventCount: 1 }),
    call('bash', { command: 'node repair.mjs' }, 'done', { exitCode: 0, startedMs: 3, finishedMs: 4, startEventIndex: 2, completeEventIndex: 3, completeEventCount: 1 })] };
  const score = () => evaluate('done', { providerResponse: { metadata } });
  assert.equal(score().pass, true);
  const original = structuredClone(metadata.toolCalls);
  for (const change of [
    calls => { calls[0].exitCode = 1; },
    calls => { calls[0].status = 'denied'; },
    calls => { calls[0].status = 'cancelled'; },
    calls => { delete calls[0].exitCode; },
    calls => { calls.shift(); },
    calls => { calls.push(structuredClone(calls[0])); },
    calls => { calls[0].input.command = 'node other.mjs'; },
    calls => { calls.reverse(); },
    calls => { calls[1].startedMs = 1; },
    calls => { calls[1].exitCode = 2; },
    calls => { calls[1].startEventIndex = 0; },
    calls => { delete calls[0].completeEventIndex; },
    calls => { calls[0].completeEventCount = 2; },
  ]) {
    metadata.toolCalls = structuredClone(original);
    change(metadata.toolCalls);
    assert.equal(score().pass, false);
  }
  metadata.toolCalls = structuredClone(original);
  metadata.toolCalls[1].startedMs = 2;
  assert.equal(score().pass, true);
  metadata.toolCalls = original;
  metadata.oracle.pass = false;
  assert.equal(score().pass, false);
  metadata.oracle.pass = true;
  metadata.budget.pass = false;
  assert.equal(score().pass, false);
  metadata.budget.pass = true;
  delete metadata.expectedFailure;
  assert.equal(score().pass, false);
});


test('legacy_profiles_keep_existing_failed_test_exception', () => {
  const failed = call('bash', { command: 'node --test broken.mjs' }, 'failed', { status: 'failed', isError: true });
  const passed = call('bash', { command: 'node --test fixed.mjs' }, 'passed');
  for (const profile of ['coding', 'rustcodegraph', 'thread-commands']) {
    const ctx = { vars: { evaluationProfile: profile }, providerResponse: { metadata: { toolCalls: [failed, passed] } } };
    assert.equal(noFailedTools('', ctx).pass, true);
    ctx.providerResponse.metadata.toolCalls.push(call('read_file', {}, 'denied', { status: 'denied', isError: true }));
    assert.equal(noFailedTools('', ctx).pass, false);
  }
});
