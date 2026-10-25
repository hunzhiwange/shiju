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
