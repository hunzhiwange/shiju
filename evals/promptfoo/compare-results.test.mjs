import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import test from 'node:test';
import { compareResults, renderMarkdownReport } from './compare-results.mjs';

const fixture = async (name) => JSON.parse(await readFile(new URL(`./fixtures/result-format/${name}.json`, import.meta.url), 'utf8'));
const document = (rows) => ({ results: { results: rows } });

test('compare_groups_three_attempts_without_rejecting_valid_repeats', async () => {
  const input = await fixture('one-case');
  const rows = input.results.results;
  rows[2].success = false;
  rows[2].gradingResult = { pass: false, reason: 'expected artifact missing' };
  rows[2].score = 0;
  rows[2].namedScores.correctness = 0;
  const result = compareResults(input, input);
  const task = result.tasks[0];
  assert.equal(result.tasks.length, 1);
  assert.equal(task.afterAttemptCount, 3);
  assert.equal(task.afterPassCount, 2);
  assert.equal(task.afterPassRate, 2 / 3);
  assert.equal(task.afterScore, 0.666666666667);
  assert.deepEqual(task.afterRows, rows);
  assert.equal(result.metrics[0].afterScore, 0.666666666667);
  assert.equal(result.failures[0].reason, 'expected artifact missing');
});

test('compare_groups_two_tasks_with_three_attempts_each', async () => {
  const input = await fixture('two-cases');
  const result = compareResults(input, input);
  assert.equal(result.tasks.length, 2);
  assert.deepEqual(result.tasks.map((row) => row.afterAttemptCount), [3, 3]);
  assert.equal(result.providers[0].taskCount, 2);
  assert.equal(result.providers[0].afterScore, 0.5);
  assert.equal(result.failures.length, 3);
});

test('compare_rejects_non_numeric_metrics', async () => {
  for (const value of ['invalid', null, true, '', Infinity]) {
    const input = await fixture('one-case');
    input.results.results[0].namedScores.correctness = value;
    assert.throws(() => compareResults(input, input), /不是有限数字/);
  }
});

test('compare_replays_single_export_and_report', async () => {
  const input = await fixture('one-case');
  input.results.results = input.results.results.slice(0, 1);
  const result = compareResults(input, input);
  assert.equal(result.tasks[0].afterScore, 1);
  assert.equal(result.tasks[0].afterPass, true);
  assert.equal(result.tasks[0].afterAttemptCount, 1);
  assert.match(renderMarkdownReport(result), /offline-fixed.*100.00/);
});

test('compare_rejects_different_tasks', async () => {
  assert.throws(() => compareResults(document([{ provider: { label: 'p' }, description: 'a', score: 1, namedScores: {} }]), document([{ provider: { label: 'p' }, description: 'b', score: 1, namedScores: {} }])), /缺少记录/);
});

test('compare_rejects_missing_metrics_within_attempts_and_across_exports', async () => {
  const input = await fixture('one-case');
  const changed = structuredClone(input);
  changed.results.results[1].namedScores = {};
  assert.throws(() => compareResults(input, changed), /metric 集不一致/);
  for (const row of changed.results.results) row.namedScores = {};
  assert.throws(() => compareResults(input, changed), /metric 集不一致/);
});

test('compare_counts_identical_rows_as_independent_attempts', async () => {
  const input = await fixture('one-case');
  const row = input.results.results[0];
  const result = compareResults(document([row, row, row]), document([row]));
  assert.equal(result.tasks[0].beforeAttemptCount, 3);
  assert.equal(result.tasks[0].afterAttemptCount, 1);
});

// Runtime result metadata carries the frozen manifest, its digest and expected repeats.
async function accuracyInput() {
  const { manifestDigest } = await import('./accuracy-manifest.mjs');
  const manifest = JSON.parse(await readFile(new URL('./accuracy-manifest.json', import.meta.url), 'utf8'));
  const hash = 'a'.repeat(64);
  manifest.stage = 'frozen';
  Object.assign(manifest.contract.experiment, {
    model: 'model-v1', endpoint: 'http://localhost:3000', environmentDigest: hash,
    contextDigest: hash, permissionsDigest: hash, evaluatorDigest: hash,
    parameters: { temperature: 0, top_p: 1, seed: 1, max_output_tokens: 4096 },
  });
  manifest.contract.tasks = ['facts', 'edit', 'recovery', 'state', 'finish'].flatMap(category =>
    Array.from({ length: 4 }, (_, i) => ({ taskId: category + i, description: category + i,
      category, split: i === 3 ? 'holdout' : 'dev', fixtureDigest: hash, oracleDigest: hash })));
  manifest.run = { sourceCommit: 'before', sourceDiffDigest: hash };
  const base = (await fixture('one-case')).results.results[0];
  return document(manifest.contract.tasks.flatMap(task => Array.from({ length: 3 }, () => ({
    ...structuredClone(base), description: task.description,
    testCase: { description: task.description, metadata: { evaluationProfile: 'runtime-accuracy' } },
    metadata: { evaluationProfile: 'runtime-accuracy', manifest: structuredClone(manifest),
      manifestDigest: manifestDigest(manifest), expectedAttempts: 3 },
  }))));
}

test('compare_rejects_equal_but_incomplete_attempt_counts', async () => {
  const input = await accuracyInput(); input.results.results.pop();
  assert.throws(() => compareResults(input, input), /缺样.*finish3/);
});

test('compare_rejects_one_sided_missing_attempt', async () => {
  const input = await accuracyInput(); const after = structuredClone(input); after.results.results.pop();
  assert.throws(() => compareResults(input, after), /缺样.*finish3/);
});

test('compare_accepts_complete_contract_and_displays_source_changes', async () => {
  const input = await accuracyInput(); const after = structuredClone(input);
  for (const row of after.results.results) row.metadata.manifest.run.sourceCommit = 'after';
  const result = compareResults(input, after);
  assert.equal(result.tasks.length, 20);
  assert.match(renderMarkdownReport(result), /before.*after/s);
  assert.equal(result.tasks[0].afterAttemptCount, 3);
  assert.deepEqual(result.tasks[0].afterSourceIdentities, [{ sourceCommit: 'after', sourceDiffDigest: 'a'.repeat(64) }]);
});

test('compare_rejects_model_and_task_contract_changes_without_leaking_values', async () => {
  const { manifestDigest } = await import('./accuracy-manifest.mjs');
  for (const field of ['model', 'fixtureDigest']) {
    const input = await accuracyInput(); const after = structuredClone(input);
    for (const row of after.results.results) {
      if (field === 'model') row.metadata.manifest.contract.experiment.model = 'secret-model';
      else row.metadata.manifest.contract.tasks[0].fixtureDigest = 'b'.repeat(64);
      row.metadata.manifestDigest = manifestDigest(row.metadata.manifest);
    }
    assert.throws(() => compareResults(input, after), error =>
      /契约.*(?:facts0|edit0)/.test(error.message) && !error.message.includes('secret-model'));
  }
});

test('compare_rejects_runtime_metric_changes', async () => {
  const input = await accuracyInput(); const after = structuredClone(input);
  for (const row of after.results.results) row.namedScores = { changed: 1 };
  assert.throws(() => compareResults(input, after), /metric 集不一致/);
});

test('compare_requires_frozen_metadata_and_matching_expected_attempts', async () => {
  for (const mutate of [
    row => { delete row.metadata; },
    row => { delete row.metadata.manifestDigest; },
    row => { row.metadata.manifest.stage = 'draft'; },
    row => { row.metadata.expectedAttempts = 10; },
    row => { row.metadata.manifestDigest = 'b'.repeat(64); },
  ]) {
    const input = await accuracyInput(); mutate(input.results.results[0]);
    assert.throws(() => compareResults(input, input), /契约|expectedAttempts|manifest/);
  }
});

test('compare_accepts_a_complete_selected_split_and_response_metadata', async () => {
  const input = await accuracyInput();
  input.results.results = input.results.results.filter(row => row.description.startsWith('facts'));
  for (const row of input.results.results) {
    row.response = { metadata: row.metadata }; delete row.metadata;
  }
  assert.equal(compareResults(input, input).tasks.length, 4);
});
