import assert from 'node:assert/strict';
import test from 'node:test';

import { compareResults, renderMarkdownReport } from './compare-results.mjs';

function result(provider, description, score, namedScores, options = {}) {
  return {
    provider: {
      id: `file://provider-${provider}.mjs`,
      label: provider,
    },
    testCase: { description },
    score,
    namedScores,
    success: options.success ?? true,
    gradingResult: {
      pass: options.success ?? true,
      reason: options.reason ?? '全部硬门槛通过',
    },
    response: {
      metadata: {
        model: options.model ?? provider,
      },
    },
  };
}

function fixture(results) {
  return { results: { results } };
}

const beforeResults = [
  result('DeepSeek', 'contract · callers', 0.8, {
    direct_rustcodegraph_call: 1,
    expected_facts: 0.5,
  }),
  result('DeepSeek', 'selection · callers', 0.6, {
    direct_rustcodegraph_call: 0,
    expected_facts: 1,
  }),
  result('Qwen', 'contract · callers', 0.7, {
    direct_rustcodegraph_call: 0.5,
    expected_facts: 0.5,
  }),
  result('Qwen', 'selection · callers', 0.5, {
    direct_rustcodegraph_call: 0,
    expected_facts: 0.5,
  }),
];

const afterResults = [
  result('Qwen', 'selection · callers', 0.9, {
    direct_rustcodegraph_call: 1,
    expected_facts: 1,
  }),
  result('DeepSeek', 'contract · callers', 0.9, {
    direct_rustcodegraph_call: 1,
    expected_facts: 1,
  }),
  result('Qwen', 'contract · callers', 0.8, {
    direct_rustcodegraph_call: 1,
    expected_facts: 0.5,
  }),
  result(
    'DeepSeek',
    'selection · callers',
    0.5,
    {
      direct_rustcodegraph_call: 0,
      expected_facts: 0.5,
    },
    { success: false, reason: '未通过硬门槛：direct_rustcodegraph_call' },
  ),
];

test('compareResults matches two providers and tasks, then aggregates score and metric deltas', () => {
  const comparison = compareResults(fixture(beforeResults), fixture(afterResults));

  assert.deepEqual(comparison.providers, [
    {
      provider: 'DeepSeek',
      beforeScore: 0.7,
      afterScore: 0.7,
      delta: 0,
      taskCount: 2,
    },
    {
      provider: 'Qwen',
      beforeScore: 0.6,
      afterScore: 0.85,
      delta: 0.25,
      taskCount: 2,
    },
  ]);
  assert.deepEqual(comparison.metrics, [
    {
      provider: 'DeepSeek',
      metric: 'direct_rustcodegraph_call',
      beforeScore: 0.5,
      afterScore: 0.5,
      delta: 0,
    },
    {
      provider: 'DeepSeek',
      metric: 'expected_facts',
      beforeScore: 0.75,
      afterScore: 0.75,
      delta: 0,
    },
    {
      provider: 'Qwen',
      metric: 'direct_rustcodegraph_call',
      beforeScore: 0.25,
      afterScore: 1,
      delta: 0.75,
    },
    {
      provider: 'Qwen',
      metric: 'expected_facts',
      beforeScore: 0.5,
      afterScore: 0.75,
      delta: 0.25,
    },
  ]);
  assert.deepEqual(comparison.tasks[0], {
    provider: 'DeepSeek',
    task: 'contract · callers',
    beforeScore: 0.8,
    afterScore: 0.9,
    delta: 0.1,
    beforePass: true,
    afterPass: true,
  });
  assert.deepEqual(comparison.failures, [
    {
      provider: 'DeepSeek',
      task: 'selection · callers',
      reason: '未通过硬门槛：direct_rustcodegraph_call',
    },
  ]);
});

test('renderMarkdownReport shows provider, task, metric deltas, and failed task details', () => {
  const report = renderMarkdownReport(
    compareResults(fixture(beforeResults), fixture(afterResults)),
    { beforeName: 'before.json', afterName: 'after.json' },
  );

  assert.match(report, /# Promptfoo 前后结果对比/);
  assert.match(report, /\| Qwen \| 60\.00 \| 85\.00 \| \+25\.00 \| 2 \|/);
  assert.match(
    report,
    /\| Qwen \| selection · callers \| 50\.00 \| 90\.00 \| \+40\.00 \|/,
  );
  assert.match(
    report,
    /\| Qwen \| direct_rustcodegraph_call \| 25\.00 \| 100\.00 \| \+75\.00 \|/,
  );
  assert.match(report, /DeepSeek.*selection · callers.*direct_rustcodegraph_call/);
});

test('compareResults rejects a task missing from after results', () => {
  assert.throws(
    () => compareResults(fixture(beforeResults), fixture(afterResults.slice(1))),
    /after 结果缺少记录：Qwen :: selection · callers/,
  );
});

test('compareResults rejects duplicate stable keys', () => {
  assert.throws(
    () => compareResults(fixture([...beforeResults, beforeResults[0]]), fixture(afterResults)),
    /before 结果包含重复记录：DeepSeek :: contract · callers/,
  );
});

test('compareResults rejects mismatched named metrics', () => {
  const afterWithoutMetric = structuredClone(afterResults);
  delete afterWithoutMetric[1].namedScores.expected_facts;

  assert.throws(
    () => compareResults(fixture(beforeResults), fixture(afterWithoutMetric)),
    /metric 集不一致.*DeepSeek :: contract · callers.*expected_facts/,
  );
});
