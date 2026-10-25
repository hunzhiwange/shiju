import test from 'node:test';
import assert from 'node:assert/strict';
import { mkdtempSync, cpSync, readFileSync, writeFileSync, rmSync, symlinkSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { spawnSync } from 'node:child_process';
import { freeze, verify, collect, evaluatorFiles } from './freeze-accuracy.mjs';
import { manifestDigest } from './accuracy-manifest.mjs';
import { prepare, cleanup } from './attempt-workspace.mjs';

const categories = ['facts', 'edit', 'recovery', 'state', 'finish'];

function sandbox(fn) {
  const root = mkdtempSync(join(tmpdir(), 'accuracy-freeze-'));
  try {
    for (const category of categories) {
      cpSync(new URL(`./tests/accuracy-${category}.yaml`, import.meta.url), join(root, `tests/accuracy-${category}.yaml`), { recursive: true });
      cpSync(new URL(`./fixtures/runtime-accuracy/${category}`, import.meta.url), join(root, `fixtures/runtime-accuracy/${category}`), { recursive: true });
    }
    for (const file of evaluatorFiles) cpSync(new URL(file, import.meta.url), join(root, file));
    const manifest = JSON.parse(readFileSync(new URL('./accuracy-manifest.json', import.meta.url)));
    manifest.contract.tasks = [];
    manifest.contract.experiment.evaluatorDigest = 'unknown';
    writeFileSync(join(root, 'accuracy-manifest.json'), JSON.stringify(manifest));
    return fn(root);
  } finally { rmSync(root, { recursive: true, force: true }); }
}
function change(root, file, fn) {
  const path = join(root, file); writeFileSync(path, fn(readFileSync(path, 'utf8')));
}
test('freeze_accepts_twenty_tasks_and_verify_is_read_only', () => sandbox(root => {
  const digest = freeze(root);
  const before = readFileSync(join(root, 'accuracy-manifest.json'));
  assert.equal(verify(root), digest);
  assert.equal(freeze(root), digest);
  assert.deepEqual(readFileSync(join(root, 'accuracy-manifest.json')), before);
  const manifest = JSON.parse(before);
  assert.equal(manifestDigest(manifest), digest);
  assert.equal(manifest.contract.tasks.length, 20);
  assert.equal(manifest.contract.tasks.filter(t => t.split === 'dev').length, 15);
  assert.equal(manifest.contract.tasks.filter(t => t.split === 'holdout').length, 5);
  assert.equal(manifest.stage, 'draft', 'offline suite freeze must not invent actual model/environment');
}));
test('freeze_rejects_changed_fixture', () => sandbox(root => {
  freeze(root);
  change(root, 'fixtures/runtime-accuracy/facts/facts-02/records.csv', s => s + '\n');
  assert.throws(() => verify(root), /changed/);
  assert.throws(() => freeze(root), /changed/);
}));
test('freeze_rejects_change_to_each_of_twenty_questions', () => {
  for (const category of categories) for (let index = 1; index <= 4; index++) sandbox(root => {
    freeze(root);
    change(root, `tests/accuracy-${category}.yaml`, s => s.replace(`${category}-0${index} ·`, `${category}-0${index} revised ·`));
    assert.throws(() => verify(root), /changed/);
  });
});
test('freeze_rejects_changed_oracles_and_evaluator', () => {
  for (const file of [...evaluatorFiles, ...categories.map(c => `fixtures/runtime-accuracy/${c}/oracle/definitions.mjs`)]) sandbox(root => {
    freeze(root); change(root, file, s => s + '\n');
    assert.throws(() => verify(root), /changed/);
  });
});
test('freeze_rejects_missing_duplicate_split_budget_and_private_inputs', () => {
  for (const mutate of [
    s => s.slice(0, s.indexOf('\n- description: facts-04')),
    s => s.replaceAll('facts-02', 'facts-01'),
    s => s.replace('split: holdout', 'split: dev'),
    s => s.replace('maxIterations: 24', 'maxIterations: 25'),
    s => s.replace('"records.csv"', '"../oracle/definitions.mjs"'),
    s => s.replace('"records.csv"', '"extra.txt"'),
    s => s.replace('    task: >-', '    reference: "answer"\n    task: >-'),
    s => s.replace('    taskId: facts-01', '    taskId: facts-01\n    taskId: facts-01'),
  ]) sandbox(root => {
    change(root, 'tests/accuracy-facts.yaml', mutate);
    assert.throws(() => freeze(root));
  });
});
test('freeze_rejects_symlinks_and_copied_oracle_payloads', () => {
  for (const mode of ['symlink', 'oracle']) sandbox(root => {
    const path = join(root, 'fixtures/runtime-accuracy/facts/facts-02/records.csv');
    if (mode === 'symlink') {
      rmSync(path); symlinkSync('../oracle/definitions.mjs', path);
    } else writeFileSync(path, 'export const cases = []; expectedFiles: answer');
    assert.throws(() => freeze(root), /symlink|oracle/);
  });
});
test('freeze_rejects_manifest_tampering_without_rewriting', () => {
  for (const mutate of [
    m => m.contract.tasks.pop(),
    m => { m.contract.tasks[0].oracleDigest = 'b'.repeat(64); },
    m => { m.contract.experiment.budget.toolCalls++; },
    m => { m.contract.experiment.evaluatorDigest = 'b'.repeat(64); },
  ]) sandbox(root => {
    freeze(root);
    change(root, 'accuracy-manifest.json', s => { const m = JSON.parse(s); mutate(m); return JSON.stringify(m); });
    const before = readFileSync(join(root, 'accuracy-manifest.json'));
    assert.throws(() => verify(root));
    assert.throws(() => freeze(root));
    assert.deepEqual(readFileSync(join(root, 'accuracy-manifest.json')), before);
  });
});
test('freeze_fixture_digest_matches_real_attempt_preparation', async () => {
  const root = new URL('./', import.meta.url).pathname;
  const collected = collect(root);
  for (const category of categories) {
    const { cases } = await import(`./fixtures/runtime-accuracy/${category}/oracle/definitions.mjs`);
    for (const c of cases) {
      const workspace = await prepare(c);
      try {
        assert.equal(collected.tasks.find(t => t.taskId === c.taskId).fixtureDigest, workspace.inputDigest, c.taskId);
      } finally { await cleanup(workspace); }
    }
  }
});
test('freeze_cli_help_and_invalid_mode', () => {
  const script = new URL('./freeze-accuracy.mjs', import.meta.url).pathname;
  const help = spawnSync(process.execPath, [script, '--help'], { encoding: 'utf8' });
  assert.equal(help.status, 0);
  assert.match(help.stdout, /freeze.*write/);
  assert.match(help.stdout, /verify.*read-only/);
  assert.equal(spawnSync(process.execPath, [script, 'invalid']).status, 1);
});
