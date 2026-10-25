import test from 'node:test';
import assert from 'node:assert/strict';
import { readFile, writeFile } from 'node:fs/promises';
import { spawn } from 'node:child_process';
import { join } from 'node:path';
import { prepare, cleanup } from './attempt-workspace.mjs';
import { createArtifactOracle } from './artifact-oracle.mjs';
import { evaluate } from './assertions.mjs';
import { cases, definitions } from './fixtures/runtime-accuracy/recovery/oracle/definitions.mjs';

async function run(workspace, command, calls) {
  const child = spawn('/bin/sh', ['-c', command], { cwd: workspace.workRoot, stdio: 'ignore' });
  workspace.trackProcess(child);
  const exitCode = await new Promise((resolve, reject) => {
    child.once('error', reject);
    child.once('exit', resolve);
  });
  const index = calls.length * 2;
  calls.push({ name: 'bash', input: { command }, status: exitCode === 0 ? 'completed' : 'failed',
    isError: exitCode !== 0, exitCode, startedMs: index, finishedMs: index + 1,
    startEventIndex: index, completeEventIndex: index + 1, completeEventCount: 1 });
  return exitCode;
}

const grade = (oracle, c, toolCalls) => evaluate('完成', { providerResponse: { metadata: {
  evaluationProfile: 'runtime-accuracy', oracle, toolCalls,
  expectedFailure: definitions.get(c.taskId).expectedFailure,
  terminal: { pass: true }, boundary: { pass: true, admissionDigest: 'a'.repeat(64) },
  budget: { pass: true },
} } });

test('recovery_oracle_accepts_four_reference_results_and_rejects_mutations', async () => {
  assert.equal(cases.length, 4);
  assert.equal(new Set(cases.map(c => c.taskId)).size, 4);
  assert.deepEqual(cases.map(c => c.split), ['dev', 'dev', 'dev', 'holdout']);
  const verify = createArtifactOracle(definitions);
  for (const c of cases) {
    const workspace = await prepare(c);
    const calls = [];
    try {
      const input = { taskId: c.taskId, workRoot: workspace.workRoot };
      assert.equal((await verify(input)).pass, false);
      assert.equal(await run(workspace, c.operation, calls), 2);
      assert.equal((await verify(input)).pass, false, `${c.taskId}: failed attempt is not completion`);
      if (c.taskId === 'recovery-03') assert.equal(await readFile(join(workspace.workRoot, 'result.txt'), 'utf8'), 'ready\n');
      for (const [file, bytes] of Object.entries(c.repairs)) await writeFile(join(workspace.workRoot, file), bytes);
      assert.equal(await run(workspace, c.recoveryOperation, calls), 0);
      for (const [file, bytes] of Object.entries(definitions.get(c.taskId).expectedFiles)) {
        assert.deepEqual(await readFile(join(workspace.workRoot, file)), bytes, `${c.taskId}: hand-derived bytes`);
      }
      const oracle = await verify(input);
      assert.equal(grade(oracle, c, calls).pass, true, `${c.taskId}: reference`);
      for (const mutation of [
        cs => cs.filter(call => call.exitCode === 0),
        cs => cs.toReversed(),
        cs => [...cs, cs.find(call => call.exitCode === 2)],
        cs => cs.map(call => ({ ...call, exitCode: undefined })),
      ]) assert.equal(grade(oracle, c, mutation(calls)).pass, false, `${c.taskId}: invalid recovery trace`);
      await writeFile(join(workspace.workRoot, 'result.txt'), 'wrong\n');
      assert.equal(grade(await verify(input), c, calls).pass, false, `${c.taskId}: wrong result`);
      await writeFile(join(workspace.workRoot, 'result.txt'), definitions.get(c.taskId).expectedFiles['result.txt']);
      await writeFile(join(workspace.workRoot, 'runner.mjs'), '// bypass\n');
      assert.equal(grade(await verify(input), c, calls).pass, false, `${c.taskId}: modified script`);
    } finally { await cleanup(workspace); }
  }
});

test('recovery_yaml_freezes_public_inputs_commands_and_budgets', async () => {
  const yaml = await readFile(new URL('./tests/accuracy-recovery.yaml', import.meta.url), 'utf8');
  const blocks = yaml.trim().split(/\n\n+/);
  assert.equal(blocks.length, 4);
  for (const [index, c] of cases.entries()) {
    const block = blocks[index];
    for (const field of [`taskId: ${c.taskId}`, `split: ${c.split}`, 'category: recovery',
      'suite: runtime-accuracy', 'batch: 15', 'evaluationProfile: runtime-accuracy',
      'maxIterations: 24', 'maxToolCalls: 32', 'timeoutMs: 900000', c.operation, c.recoveryOperation]) assert.ok(block.includes(field), field);
    assert.deepEqual(JSON.parse(block.match(/fixtureFiles: (.+)/)[1]), c.fixtureFiles);
    assert.doesNotMatch(block, /oracle\/|expectedFiles|reference/);
  }
});
