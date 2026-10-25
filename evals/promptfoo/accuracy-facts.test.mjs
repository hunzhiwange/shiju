import test from 'node:test';
import assert from 'node:assert/strict';
import { createArtifactOracle } from './artifact-oracle.mjs';
import { prepare, cleanup } from './attempt-workspace.mjs';
import { cases, definitions } from './fixtures/runtime-accuracy/facts/oracle/definitions.mjs';

test('facts_oracle_accepts_four_reference_results_and_rejects_mutations', async () => {
  assert.equal(cases.length, 4);
  assert.equal(new Set(cases.map(c => c.taskId)).size, 4);
  assert.deepEqual(cases.map(c => c.split), ['dev', 'dev', 'dev', 'holdout']);
  const verify = createArtifactOracle(definitions);
  for (const c of cases) {
    const workspace = await prepare(c);
    try {
      const input = { taskId: c.taskId, workRoot: workspace.workRoot };
      assert.equal((await verify({ ...input, response: ` \n${JSON.stringify(c.reference, null, 2)}\t` })).pass, true);
      assert.equal((await verify({ ...input, response: JSON.stringify(c.mutation) })).pass, false);
    } finally { await cleanup(workspace); }
  }
});
