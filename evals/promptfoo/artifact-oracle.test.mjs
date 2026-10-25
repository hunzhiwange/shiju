import test from 'node:test';
import assert from 'node:assert/strict';
import { mkdtemp, mkdir, writeFile, symlink, rm, realpath } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { createArtifactOracle } from './artifact-oracle.mjs';

async function fixture(t) {
  const root = await mkdtemp(join(await realpath(tmpdir()), 'oracle-'));
  t.after(() => rm(root, { recursive: true, force: true }));
  const workRoot = join(root, 'work');
  await mkdir(workRoot);
  await writeFile(join(workRoot, 'answer.txt'), 'correct\n');
  await writeFile(join(workRoot, 'input.txt'), 'original\n');
  const oracle = createArtifactOracle(new Map([['edit', {
    expectedFiles: { 'answer.txt': Buffer.from('correct\n') },
    unchangedFiles: { 'input.txt': Buffer.from('original\n') },
    allowExtraFiles: false,
  }]]));
  return { root, workRoot, oracle };
}

test('oracle_accepts_correct_files_and_only_allowed_changes', async t => {
  const f = await fixture(t);
  assert.equal((await f.oracle({ taskId: 'edit', workRoot: f.workRoot })).pass, true);
});
test('oracle_rejects_wrong_bytes_even_when_agent_claims_success', async t => {
  const f = await fixture(t);
  await writeFile(join(f.workRoot, 'answer.txt'), 'secret wrong answer');
  const result = await f.oracle({ taskId: 'edit', workRoot: f.workRoot, response: 'success', verifier: '/etc/passwd' });
  assert.equal(result.pass, false);
  assert.equal(JSON.stringify(result).includes('secret wrong answer'), false);
});
test('oracle_rejects_missing_file', async t => {
  const f = await fixture(t);
  await rm(join(f.workRoot, 'answer.txt'));
  assert.equal((await f.oracle({ taskId: 'edit', workRoot: f.workRoot })).pass, false);
});
test('oracle_rejects_extra_files_and_empty_directories', async t => {
  const f = await fixture(t);
  await mkdir(join(f.workRoot, 'extra'));
  assert.equal((await f.oracle({ taskId: 'edit', workRoot: f.workRoot })).pass, false);
  await rm(join(f.workRoot, 'extra'), { recursive: true });
  await writeFile(join(f.workRoot, 'extra'), 'extra');
  assert.equal((await f.oracle({ taskId: 'edit', workRoot: f.workRoot })).pass, false);
});
test('oracle_rejects_symlink_escape', async t => {
  const f = await fixture(t);
  await writeFile(join(f.root, 'outside'), 'correct\n');
  await rm(join(f.workRoot, 'answer.txt'));
  await symlink(join(f.root, 'outside'), join(f.workRoot, 'answer.txt'));
  assert.equal((await f.oracle({ taskId: 'edit', workRoot: f.workRoot })).pass, false);
});
test('oracle_rejects_protected_file_changes', async t => {
  const f = await fixture(t);
  await writeFile(join(f.workRoot, 'input.txt'), 'changed');
  assert.equal((await f.oracle({ taskId: 'edit', workRoot: f.workRoot })).pass, false);
});
test('oracle_rejects_unknown_tasks_and_invalid_definitions', async t => {
  const f = await fixture(t);
  await assert.rejects(f.oracle({ taskId: 'unknown', workRoot: f.workRoot }), /configuration/);
  for (const definition of [{}, { expectedFiles: { '../outside': Buffer.from('x') }, unchangedFiles: {}, allowExtraFiles: false }]) {
    assert.throws(() => createArtifactOracle(new Map([['bad', definition]])), /configuration/);
  }
});
test('oracle_honors_explicit_extra_file_policy_and_snapshots_definitions', async t => {
  const f = await fixture(t);
  const bytes = Buffer.from('correct\n');
  const definitions = new Map([['extra', { expectedFiles: { 'answer.txt': bytes }, unchangedFiles: {}, allowExtraFiles: true }]]);
  const oracle = createArtifactOracle(definitions);
  bytes.fill(0);
  definitions.clear();
  assert.equal((await oracle({ taskId: 'extra', workRoot: f.workRoot })).pass, true);
});
test('oracle_rejects_symlink_work_root_and_nested_symlinks', async t => {
  const f = await fixture(t);
  await symlink(f.workRoot, join(f.root, 'alias'));
  assert.equal((await f.oracle({ taskId: 'edit', workRoot: join(f.root, 'alias') })).pass, false);
  await symlink(f.root, join(f.workRoot, 'nested'));
  assert.equal((await f.oracle({ taskId: 'edit', workRoot: f.workRoot })).pass, false);
});
test('oracle_compares_nested_binary_files_and_bounds_diagnostics', async t => {
  const f = await fixture(t);
  await mkdir(join(f.workRoot, 'output'));
  await writeFile(join(f.workRoot, 'output', 'bytes'), Buffer.from([0, 255, 10]));
  const oracle = createArtifactOracle(new Map([['binary', {
    expectedFiles: { 'output/bytes': Buffer.from([0, 255, 10]) },
    unchangedFiles: {}, allowExtraFiles: true,
  }]]));
  assert.equal((await oracle({ taskId: 'binary', workRoot: f.workRoot })).pass, true);
  for (let i = 0; i < 105; i++) await writeFile(join(f.workRoot, `extra-${i}`), 'private');
  const result = await f.oracle({ taskId: 'edit', workRoot: f.workRoot });
  assert.equal(result.pass, false);
  assert.ok(result.checks.length <= 100);
  assert.ok(JSON.stringify(result).length < 6000);
});
