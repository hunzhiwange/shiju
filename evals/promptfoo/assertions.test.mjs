import assert from 'node:assert/strict';
import test from 'node:test';

import {
  containsExpectedTerms,
  directRustcodegraphCall,
  expectedFactsScore,
  maxToolCalls,
  noNavigationFallback,
  noFailedTools,
  readOnlyTrajectory,
  rustcodegraphOperation,
  successfulRustcodegraphCall,
  successfulRun,
} from './assertions.mjs';

function context(metadata, extra = {}) {
  return { providerResponse: { metadata }, ...extra };
}

test('successfulRun requires output and chat.done', () => {
  assert.equal(successfulRun('ok', context({ eventTypes: ['chat.done'] })).pass, true);
  assert.equal(successfulRun('', context({ eventTypes: ['chat.done'] })).pass, false);
  assert.equal(successfulRun('ok', context({ eventTypes: ['chat.error'] })).pass, false);
});

test('tool assertions report failures and enforce limits', () => {
  const toolCalls = [
    { name: 'read', isError: false },
    { name: 'shell', status: 'failed', isError: true },
  ];
  assert.equal(noFailedTools('', context({ toolCalls })).pass, false);
  assert.equal(
    maxToolCalls('', { ...context({ toolCalls }), config: { maxToolCalls: 1 } }).pass,
    false,
  );
});

test('readOnlyTrajectory rejects write tools and mutating shell commands', () => {
  assert.equal(
    readOnlyTrajectory('', context({ toolCalls: [{ name: 'read_file', input: { path: 'x' } }] }))
      .pass,
    true,
  );
  assert.equal(
    readOnlyTrajectory('', context({ toolCalls: [{ name: 'apply_patch', input: {} }] })).pass,
    false,
  );
  assert.equal(
    readOnlyTrajectory(
      '',
      context({ toolCalls: [{ name: 'shell', input: { command: 'git reset --hard HEAD' } }] }),
    ).pass,
    false,
  );
});

test('containsExpectedTerms lists missing terms', () => {
  const result = containsExpectedTerms('alpha beta', {
    vars: { expectedTerms: '["alpha", "gamma"]' },
  });
  assert.equal(result.pass, false);
  assert.match(result.reason, /gamma/);
});

test('directRustcodegraphCall only accepts the builtin tool name', () => {
  const direct = directRustcodegraphCall(
    '',
    context({
      toolCalls: [
        {
          name: 'rustcodegraph',
          input: { operation: 'callers', symbol: 'assemble_session_protocol_messages' },
          status: 'completed',
        },
      ],
    }),
  );
  const cli = directRustcodegraphCall(
    '',
    context({
      toolCalls: [
        {
          name: 'bash',
          input: { command: 'rustcodegraph callers assemble_session_protocol_messages' },
          status: 'completed',
        },
      ],
    }),
  );

  assert.equal(direct.pass, true);
  assert.equal(cli.pass, false);
  assert.match(cli.reason, /内置 rustcodegraph/);
});

test('successfulRustcodegraphCall requires a completed builtin call', () => {
  const failed = successfulRustcodegraphCall(
    '',
    context({
      toolCalls: [
        {
          name: 'rustcodegraph',
          input: { operation: 'callers' },
          status: 'failed',
          isError: true,
        },
      ],
    }),
  );

  assert.equal(failed.pass, false);
  assert.match(failed.reason, /callers.*failed/);
});

test('rustcodegraphOperation matches configured operations and explains mismatches', () => {
  const toolCalls = [
    {
      name: 'rustcodegraph',
      input: { operation: 'callers' },
      status: 'completed',
      isError: false,
    },
  ];
  const matched = rustcodegraphOperation('', {
    ...context({ toolCalls }),
    vars: { expectedOperations: '["callers"]' },
  });
  const mismatched = rustcodegraphOperation('', {
    ...context({ toolCalls }),
    config: { expectedOperations: ['impact'] },
  });

  assert.equal(matched.pass, true);
  assert.equal(mismatched.pass, false);
  assert.match(mismatched.reason, /期望.*impact/);
  assert.match(mismatched.reason, /实际.*callers/);
});

test('rustcodegraphOperation explains failed builtin calls', () => {
  const result = rustcodegraphOperation('', {
    ...context({
      toolCalls: [
        {
          name: 'rustcodegraph',
          input: { operation: 'callers' },
          status: 'failed',
          isError: true,
        },
      ],
    }),
    vars: { expectedOperations: ['callers'] },
  });

  assert.equal(result.pass, false);
  assert.match(result.reason, /未成功.*callers\(failed\)/);
});

test('noNavigationFallback identifies CLI and text-search fallbacks', () => {
  const nativeOnly = noNavigationFallback(
    '',
    context({
      toolCalls: [
        {
          name: 'rustcodegraph',
          input: { operation: 'search', query: 'runner' },
          status: 'completed',
        },
      ],
    }),
  );
  const fallbacks = noNavigationFallback(
    '',
    context({
      toolCalls: [
        { name: 'bash', input: { command: 'rustcodegraph callers runner' }, status: 'completed' },
        { name: 'shell', input: { cmd: 'rg "runner" crates/' }, status: 'completed' },
      ],
    }),
  );

  assert.equal(nativeOnly.pass, true);
  assert.equal(fallbacks.pass, false);
  assert.match(fallbacks.reason, /CLI fallback/);
  assert.match(fallbacks.reason, /文本搜索 fallback/);
});

test('expectedFactsScore awards partial credit and lists missing facts', () => {
  const result = expectedFactsScore('alpha and gamma', {
    vars: { expectedTerms: '["alpha", "beta", "gamma"]' },
  });

  assert.equal(result.pass, false);
  assert.equal(result.score, 2 / 3);
  assert.match(result.reason, /2\/3/);
  assert.match(result.reason, /beta/);
});
