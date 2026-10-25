import test from 'node:test';
import assert from 'node:assert/strict';
import { readFile, writeFile, mkdir, rm } from 'node:fs/promises';
import { join } from 'node:path';
import { createHash } from 'node:crypto';
import { createArtifactOracle } from './artifact-oracle.mjs';
import { prepare, cleanup } from './attempt-workspace.mjs';
import { cases, definitions } from './fixtures/runtime-accuracy/edit/oracle/definitions.mjs';

test('edit_fixtures_preserve_original_crlf_bytes_before_and_after_preparation', async () => {
  const originals = [
    ['edit-01', 'notes.txt', 'keep bytes\r\n'],
    ['edit-02', 'routes.txt', 'north\r\n# INSERT ROUTE HERE\r\nsouth\r\n'],
  ];
  for (const [taskId, file, original] of originals) {
    const c = cases.find(c => c.taskId === taskId);
    const expected = Buffer.from(original);
    assert.deepEqual(await readFile(join(c.fixtureRoot, file)), expected, `${taskId}: source bytes`);
    const workspace = await prepare(c);
    try {
      assert.deepEqual(await readFile(join(workspace.workRoot, file)), expected, `${taskId}: prepared bytes`);
    } finally { await cleanup(workspace); }
  }
});

const digest = bytes => createHash('sha256').update(bytes).digest('hex');
test('edit_oracle_accepts_four_reference_results_and_rejects_mutations', async () => {
  assert.equal(cases.length, 4);
  assert.equal(new Set(cases.map(c => c.taskId)).size, 4);
  assert.deepEqual(cases.map(c => c.split), ['dev', 'dev', 'dev', 'holdout']);
  const verify = createArtifactOracle(definitions);
  for (const c of cases) {
    const workspace = await prepare(c);
    try {
      const input = { taskId: c.taskId, workRoot: workspace.workRoot };
      assert.equal((await verify(input)).pass, false, `${c.taskId}: unchanged input`);
      await writeFile(join(workspace.workRoot, c.target), c.reference);
      assert.equal((await verify(input)).pass, true, `${c.taskId}: reference`);
      for (const [file, bytes] of Object.entries(definitions.get(c.taskId).unchangedFiles)) {
        assert.equal(digest(await readFile(join(workspace.workRoot, file))), digest(bytes));
        await writeFile(join(workspace.workRoot, file), Buffer.concat([bytes, Buffer.from(' ')]));
        assert.equal((await verify(input)).pass, false, `${c.taskId}: non-target mutation`);
        await writeFile(join(workspace.workRoot, file), bytes);
      }
      await writeFile(join(workspace.workRoot, c.target), c.mutation);
      assert.equal((await verify(input)).pass, false, `${c.taskId}: incorrect target`);
      await writeFile(join(workspace.workRoot, c.target), c.reference);
      await mkdir(join(workspace.workRoot, 'unexpected'));
      assert.equal((await verify(input)).pass, false, `${c.taskId}: extra directory`);
      await rm(join(workspace.workRoot, 'unexpected'), { recursive: true });
      await writeFile(join(workspace.workRoot, 'extra.txt'), 'extra');
      assert.equal((await verify(input)).pass, false, `${c.taskId}: extra file`);
    } finally { await cleanup(workspace); }
  }
});
