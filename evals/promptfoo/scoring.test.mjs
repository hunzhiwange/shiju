import assert from 'node:assert/strict';
import test from 'node:test';

import {
  directRustcodegraphCall,
  expectedFactsScore,
  maxToolCalls,
  noFailedTools,
  noNavigationFallback,
  readOnlyTrajectory,
  rustcodegraphOperation,
  successfulRun,
  successfulRustcodegraphCall,
} from './assertions.mjs';
import scoreTrajectory, { METRIC_NAMES } from './scoring.mjs';

function completeScores(overrides = {}) {
  return {
    [METRIC_NAMES.successfulRun]: 1,
    [METRIC_NAMES.readOnly]: 1,
    [METRIC_NAMES.noFailedTools]: 1,
    [METRIC_NAMES.directRustcodegraphCall]: 1,
    [METRIC_NAMES.successfulRustcodegraphCall]: 1,
    [METRIC_NAMES.rustcodegraphOperation]: 1,
    [METRIC_NAMES.noNavigationFallback]: 1,
    [METRIC_NAMES.expectedFacts]: 1,
    [METRIC_NAMES.toolCallBudget]: 1,
    ...overrides,
  };
}

test('metric names remain stable for Promptfoo reports', () => {
  assert.deepEqual(METRIC_NAMES, {
    successfulRun: 'successful_run',
    readOnly: 'read_only',
    noFailedTools: 'no_failed_tools',
    directRustcodegraphCall: 'direct_rustcodegraph_call',
    successfulRustcodegraphCall: 'rustcodegraph_call_success',
    rustcodegraphOperation: 'rustcodegraph_operation',
    noNavigationFallback: 'no_navigation_fallback',
    expectedFacts: 'expected_facts',
    toolCallBudget: 'tool_call_budget',
  });
});

test('scoreTrajectory returns a normalized perfect score', () => {
  const result = scoreTrajectory(completeScores());

  assert.deepEqual(result, {
    pass: true,
    score: 1,
    reason: '总分 100/100；全部硬门槛通过',
  });
});

test('scoreTrajectory treats critical trajectory metrics as hard gates', () => {
  const result = scoreTrajectory(
    completeScores({ [METRIC_NAMES.directRustcodegraphCall]: 0 }),
  );

  assert.equal(result.pass, false);
  assert.equal(result.score, 0.8);
  assert.match(result.reason, /未通过硬门槛：direct_rustcodegraph_call/);
});

test('old CLI and grep trajectory scores 45/100 and fails', () => {
  const output = '调用者位于 llm_messages.rs。';
  const context = {
    providerResponse: {
      metadata: {
        eventTypes: ['chat.step_start', 'chat.tool', 'chat.done'],
        toolCalls: [
          {
            name: 'bash',
            input: {
              command: 'rustcodegraph callers assemble_session_protocol_messages',
            },
            status: 'completed',
            isError: false,
          },
          {
            name: 'shell',
            input: { command: 'grep -R "assemble_session_protocol_messages" crates/' },
            status: 'completed',
            isError: false,
          },
        ],
      },
    },
    vars: {
      expectedOperations: '["callers"]',
      expectedTerms: '["llm_messages.rs"]',
    },
    config: { maxToolCalls: 12 },
  };
  const assertionResults = {
    [METRIC_NAMES.successfulRun]: successfulRun(output, context),
    [METRIC_NAMES.readOnly]: readOnlyTrajectory(output, context),
    [METRIC_NAMES.noFailedTools]: noFailedTools(output, context),
    [METRIC_NAMES.directRustcodegraphCall]: directRustcodegraphCall(output, context),
    [METRIC_NAMES.successfulRustcodegraphCall]: successfulRustcodegraphCall(output, context),
    [METRIC_NAMES.rustcodegraphOperation]: rustcodegraphOperation(output, context),
    [METRIC_NAMES.noNavigationFallback]: noNavigationFallback(output, context),
    [METRIC_NAMES.expectedFacts]: expectedFactsScore(output, context),
    [METRIC_NAMES.toolCallBudget]: maxToolCalls(output, context),
  };
  const result = scoreTrajectory(
    Object.fromEntries(
      Object.entries(assertionResults).map(([name, grading]) => [name, grading.score]),
    ),
  );

  assert.equal(assertionResults[METRIC_NAMES.directRustcodegraphCall].pass, false);
  assert.equal(assertionResults[METRIC_NAMES.rustcodegraphOperation].pass, false);
  assert.equal(assertionResults[METRIC_NAMES.noNavigationFallback].pass, false);
  assert.equal(result.pass, false);
  assert.equal(result.score, 0.45);
  assert.match(result.reason, /总分 45\/100/);
  assert.match(
    result.reason,
    /direct_rustcodegraph_call, rustcodegraph_call_success/,
  );
});
