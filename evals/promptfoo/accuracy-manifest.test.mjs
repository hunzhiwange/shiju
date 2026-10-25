import assert from 'node:assert/strict';
import test from 'node:test';
import { loadManifest, validateManifest, manifestDigest, assertComparable } from './accuracy-manifest.mjs';

const digest = 'a'.repeat(64);
function frozen() {
  return {
    schemaVersion: 1, stage: 'frozen',
    contract: {
      experiment: {
        model: 'provider/model@version', label: 'baseline', endpoint: 'http://localhost:3000',
        budget: { iterations: 24, toolCalls: 32, wallTimeMs: 900000 },
        parameters: { temperature: 0, top_p: 1, seed: 1, max_output_tokens: 4096 },
        environmentDigest: digest, contextDigest: digest, permissionsDigest: digest,
        evaluatorDigest: digest, execution: { repeats: 3, concurrency: 1, cache: false },
      },
      tasks: ['facts', 'edit', 'recovery', 'state', 'finish'].flatMap(category =>
        Array.from({ length: 4 }, (_, i) => ({
          taskId: category + i, description: category + ' task ' + i, category,
          split: i === 3 ? 'holdout' : 'dev', fixtureDigest: digest, oracleDigest: digest,
        }))),
    },
    run: { sourceCommit: 'abc', sourceDiffDigest: digest, processId: 1, time: 'now', attempt: 1 },
  };
}
test('manifest_loads_draft_but_rejects_formal_comparison', () => {
  const value = loadManifest(new URL('./accuracy-manifest.json', import.meta.url));
  assert.equal(value.stage, 'draft');
  assert.throws(() => assertComparable(value), /draft/);
});
test('manifest_accepts_frozen_matrix', () => {
  assert.equal(validateManifest(frozen()).stage, 'frozen');
  assert.doesNotThrow(() => assertComparable(frozen()));
});
test('manifest_rejects_invalid_frozen_matrix', () => {
  for (const mutate of [
    m => m.contract.tasks.pop(),
    m => { m.contract.tasks[0].category = 'edit'; },
    m => { m.contract.tasks[0].split = 'holdout'; },
    m => { m.contract.tasks[3].split = 'dev'; m.contract.tasks[0].split = 'holdout'; },
  ]) {
    const value = frozen(); mutate(value);
    assert.throws(() => validateManifest(value));
  }
});
test('manifest_rejects_missing_frozen_fields', () => {
  for (const key of Object.keys(frozen().contract.experiment)) {
    const value = frozen(); delete value.contract.experiment[key];
    assert.throws(() => validateManifest(value), new RegExp(key));
  }
  const value = frozen(); value.contract.experiment.model = 'unknown';
  assert.throws(() => validateManifest(value), /model/);
});
test('manifest_rejects_duplicate_ids_and_invalid_split', () => {
  const duplicate = frozen(); duplicate.contract.tasks[1].taskId = duplicate.contract.tasks[0].taskId;
  assert.throws(() => validateManifest(duplicate), /duplicate/);
  const split = frozen(); split.contract.tasks[0].split = 'test';
  assert.throws(() => validateManifest(split), /split/);
});
test('manifest_digest_changes_with_budget', () => {
  const value = frozen(); const before = manifestDigest(value);
  value.contract.experiment.budget.iterations++;
  assert.notEqual(manifestDigest(value), before);
});
test('manifest_digest_covers_model_fixture_oracle_and_parameters', () => {
  for (const mutate of [
    m => { m.contract.experiment.model += '-new'; },
    m => { m.contract.tasks[0].fixtureDigest = 'b'.repeat(64); },
    m => { m.contract.tasks[0].oracleDigest = 'b'.repeat(64); },
    m => { m.contract.experiment.parameters.seed++; },
  ]) {
    const value = frozen(); const before = manifestDigest(value); mutate(value);
    assert.notEqual(manifestDigest(value), before);
  }
});
test('manifest_digest_ignores_key_order_and_run_identity', () => {
  const value = frozen();
  const reverse = x => Array.isArray(x) ? x.map(reverse) : x && typeof x === 'object'
    ? Object.fromEntries(Object.entries(x).reverse().map(([k, v]) => [k, reverse(v)])) : x;
  assert.equal(manifestDigest(value), manifestDigest(reverse(value)));
  const changed = structuredClone(value);
  changed.run = { sourceCommit: 'def', time: 'later', attempt: 2 };
  assert.equal(manifestDigest(value), manifestDigest(changed));
});
test('manifest_rejects_unknown_fields_and_invalid_budget', () => {
  for (const mutate of [
    m => { m.secret = 'redacted'; },
    m => { m.contract.experiment.apiKey = 'redacted'; },
    m => { m.contract.tasks[0].extra = true; },
    m => { m.contract.experiment.parameters.extra = 1; },
    m => { m.contract.experiment.budget.iterations = 0; },
    m => { m.contract.experiment.endpoint = 'http://user:secret@localhost'; },
  ]) {
    const value = frozen(); mutate(value); assert.throws(() => validateManifest(value));
  }
});
test('manifest_rejects_missing_task_fields_and_invalid_digests', () => {
  for (const key of Object.keys(frozen().contract.tasks[0])) {
    const value = frozen(); delete value.contract.tasks[0][key];
    assert.throws(() => validateManifest(value), new RegExp(key));
  }
  const value = frozen(); value.contract.tasks[0].oracleDigest = 'unknown';
  assert.throws(() => validateManifest(value), /oracleDigest/);
});
test('manifest_rejects_unknown_schema_run_and_execution_fields', () => {
  for (const mutate of [
    m => { m.schemaVersion = 2; },
    m => { m.stage = 'ready'; },
    m => { m.run.rawTrace = []; },
    m => { m.contract.experiment.execution.repeats = 10; },
    m => { m.contract.experiment.execution.cache = true; },
    m => { m.contract.experiment.parameters.seed = 'unknown'; },
    m => { m.contract.experiment.parameters.top_p = 2; },
  ]) {
    const value = frozen(); mutate(value); assert.throws(() => validateManifest(value));
  }
});

test('manifest_frozen_parameters_distinguish_explicit_provider_defaults_from_unknown_values', () => {
  const value = frozen();
  for (const key of Object.keys(value.contract.experiment.parameters)) {
    value.contract.experiment.parameters[key] = 'provider-default';
  }
  assert.doesNotThrow(() => validateManifest(value));
  for (const unsupported of ['unknown', 'default', null]) {
    const invalid = structuredClone(value);
    invalid.contract.experiment.parameters.seed = unsupported;
    assert.throws(() => validateManifest(invalid));
  }
});
