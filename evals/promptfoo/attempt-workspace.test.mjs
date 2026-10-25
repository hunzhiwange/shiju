import { test } from 'node:test';
import assert from 'node:assert/strict';
import { mkdtemp, mkdir, writeFile, readFile, symlink, rm, readdir } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join, isAbsolute } from 'node:path';
import { spawn } from 'node:child_process';
import { once } from 'node:events';
import { prepare, cleanup } from './attempt-workspace.mjs';

async function fixture(t) {
  const root = await mkdtemp(join(tmpdir(), 'workspace-test-'));
  t.after(() => rm(root, { recursive: true, force: true }));
  const source = join(root, 'source');
  await mkdir(join(source, 'public'), { recursive: true });
  await writeFile(join(source, 'public/input.txt'), 'public input');
  await writeFile(join(source, 'oracle.txt'), 'private answer');
  const options = { fixtureRoot: source, fixtureFiles: ['public/input.txt'] };
  return { root, source, options };
}

test('workspace_rejects_fixture_symlink_and_preserves_source', async t => {
  const { root, source } = await fixture(t);
  const canary = join(root, 'canary');
  await writeFile(canary, 'untouched');
  await symlink(canary, join(source, 'link'));
  await assert.rejects(prepare({ fixtureRoot: source, fixtureFiles: ['link'] }), /symlink/);
  assert.equal(await readFile(canary, 'utf8'), 'untouched');
  assert.equal(await readFile(join(source, 'public/input.txt'), 'utf8'), 'public input');
});

test('workspace_allocates_distinct_absolute_roots', async t => {
  const { options } = await fixture(t);
  const a = await prepare(options), b = await prepare(options);
  t.after(() => Promise.all([cleanup(a), cleanup(b)]));
  assert.notEqual(a.attemptId, b.attemptId);
  assert.notEqual(a.workRoot, b.workRoot);
  assert.ok(isAbsolute(a.workRoot));
  assert.equal(a.inputDigest, b.inputDigest);
});

test('workspace_copies_only_listed_public_files_without_writeback', async t => {
  const { source, options } = await fixture(t);
  const a = await prepare(options);
  t.after(() => cleanup(a));
  assert.deepEqual(await readdir(a.workRoot), ['public']);
  await writeFile(join(a.workRoot, 'public/input.txt'), 'changed');
  const b = await prepare(options);
  t.after(() => cleanup(b));
  assert.equal(a.inputDigest, b.inputDigest);
  assert.equal(await readFile(join(source, 'public/input.txt'), 'utf8'), 'public input');
});

for (const path of ['../oracle.txt', '/tmp/input', 'public/../../oracle.txt', 'public\\input.txt']) {
  test('workspace_rejects_untrusted_path_' + path, async t => {
    const { options } = await fixture(t);
    await assert.rejects(prepare({ ...options, fixtureFiles: [path] }), /relative/);
  });
}

test('workspace_rejects_symlink_parent', async t => {
  const { source, options } = await fixture(t);
  await symlink('public', join(source, 'alias'));
  await assert.rejects(prepare({ ...options, fixtureFiles: ['alias/input.txt'] }), /symlink/);
});

test('workspace_cleanup_rejects_forged_handle_and_preserves_canary', async t => {
  const { root, options } = await fixture(t);
  await writeFile(join(root, 'canary'), 'untouched');
  const a = await prepare(options);
  await assert.rejects(cleanup({ ...a, workRoot: root }), /unknown/);
  await symlink(root, join(a.workRoot, 'outside'));
  await cleanup(a);
  assert.equal(await readFile(join(root, 'canary'), 'utf8'), 'untouched');
  await cleanup(a);
});

test('workspace_cleanup_refuses_replaced_root', async t => {
  const { root, options } = await fixture(t);
  const a = await prepare(options);
  await rm(a.workRoot, { recursive: true });
  await symlink(root, a.workRoot);
  t.after(() => rm(a.workRoot, { force: true }));
  await assert.rejects(cleanup(a), /replaced/);
  assert.equal(await readFile(join(root, 'source/oracle.txt'), 'utf8'), 'private answer');
});

test('workspace_cleanup_waits_for_tracked_process_exit', async t => {
  const { options } = await fixture(t);
  const a = await prepare(options);
  const child = spawn(process.execPath, ['-e', 'setTimeout(() => {}, 60000)'], { cwd: a.workRoot });
  a.trackProcess(child);
  t.after(async () => { if (child.exitCode === null && child.signalCode === null) { child.kill(); await once(child, 'exit'); } await cleanup(a); });
  await assert.rejects(cleanup(a), /active process/);
  child.kill();
  await once(child, 'exit');
  await cleanup(a);
  assert.deepEqual(a.cleanupPolicy, { success: 'after-process-exit', failure: 'after-process-exit', interruption: 'after-process-exit' });
});

test('workspace_digest_changes_with_public_input_bytes', async t => {
  const { source, options } = await fixture(t);
  const a = await prepare(options);
  await writeFile(join(source, 'public/input.txt'), 'new public input');
  const b = await prepare(options);
  t.after(() => Promise.all([cleanup(a), cleanup(b)]));
  assert.match(a.inputDigest, /^[a-f0-9]{64}$/);
  assert.notEqual(a.inputDigest, b.inputDigest);
});

test('workspace_rejects_directories_and_duplicate_inputs', async t => {
  const { options } = await fixture(t);
  await assert.rejects(prepare({ ...options, fixtureFiles: ['public'] }), /regular file/);
  await assert.rejects(prepare({ ...options, fixtureFiles: ['public/input.txt', 'public/input.txt'] }), /duplicate/);
});
