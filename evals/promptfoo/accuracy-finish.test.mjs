import test from 'node:test';
import assert from 'node:assert/strict';
import { readFile, writeFile, rm, stat } from 'node:fs/promises';
import { join } from 'node:path';
import { createArtifactOracle } from './artifact-oracle.mjs';
import { evaluate, readOnlyTrajectory } from './assertions.mjs';
import { prepare, cleanup } from './attempt-workspace.mjs';
import { cases, definitions } from './fixtures/runtime-accuracy/finish/oracle/definitions.mjs';

const grade = (oracle, terminal = true) => evaluate('完成', { vars: { evaluationProfile: 'runtime-accuracy' },
  providerResponse: { metadata: { oracle, terminal: { pass: terminal },
    boundary: { pass: true, admissionDigest: 'a'.repeat(64) }, budget: { pass: true } } } });

test('finish_oracle_accepts_four_reference_results_and_rejects_mutations', async () => {
  assert.equal(cases.length, 4);
  assert.equal(new Set(cases.map(c => c.taskId)).size, 4);
  assert.deepEqual(cases.map(c => c.split), ['dev', 'dev', 'dev', 'holdout']);
  const verify = createArtifactOracle(definitions);
  for (const c of cases) {
    const workspace = await prepare(c);
    try {
      const input = { taskId: c.taskId, workRoot: workspace.workRoot };
      const before = await stat(join(workspace.workRoot, 'result.txt')).catch(() => null);
      if (c.taskId !== 'finish-02') {
        for (const [file, bytes] of Object.entries(c.referenceFiles)) await writeFile(join(workspace.workRoot, file), bytes);
      }
      assert.equal(grade(await verify(input)).pass, true, `${c.taskId}: reference`);
      assert.equal(grade(await verify(input), false).pass, false, `${c.taskId}: missing chat.done`);
      if (c.taskId === 'finish-02') {
        const after = await stat(join(workspace.workRoot, 'result.txt'));
        assert.equal(after.mtimeMs, before.mtimeMs);
        assert.equal(after.ino, before.ino);
      }
      if (c.missingFile) await rm(join(workspace.workRoot, c.missingFile));
      else await writeFile(join(workspace.workRoot, 'result.txt'), 'incorrect\n');
      assert.equal(grade(await verify(input)).pass, false, `${c.taskId}: claims completion with wrong/missing artifact`);
      for (const [file, bytes] of Object.entries(c.referenceFiles)) await writeFile(join(workspace.workRoot, file), bytes);
      await writeFile(join(workspace.workRoot, 'notes.txt'), 'changed\n');
      assert.equal(grade(await verify(input)).pass, false, `${c.taskId}: protected input changed`);
    } finally { await cleanup(workspace); }
  }
});

test('finish_yaml_exposes_only_public_inputs_and_frozen_budgets', async () => {
  const yaml = await readFile(new URL('./tests/accuracy-finish.yaml', import.meta.url), 'utf8');
  const blocks = yaml.trim().split(/\n\n+/);
  assert.equal(blocks.length, 4);
  for (const [index, c] of cases.entries()) {
    const block = blocks[index];
    assert.match(block, new RegExp(`taskId: ${c.taskId}`));
    for (const field of ['suite: runtime-accuracy', 'category: finish', `split: ${c.split}`,
      'evaluationProfile: runtime-accuracy', 'maxIterations: 24', 'maxToolCalls: 32', 'timeoutMs: 900000']) assert.ok(block.includes(field));
    assert.deepEqual(JSON.parse(block.match(/fixtureFiles: (.+)/)[1]), c.fixtureFiles);
    assert.doesNotMatch(block, /oracle\/|referenceFiles|故意做错/);
  }
});

// Only model/HTTP and external sandbox supervision are replaced. The provider,
// SSE parser, fixture copies, byte oracle, terminal grading and cleanup are real.
test('finish_provider_requires_chat_done_and_rejects_completion_claims', async () => {
  const { default: Provider } = await import('./vibewindow-provider.mjs');
  const { loadManifest } = await import('./accuracy-manifest.mjs');
  const { execFileSync } = await import('node:child_process');
  const blocks = (await readFile(new URL('./tests/accuracy-finish.yaml', import.meta.url), 'utf8')).trim().split(/\n\n+/);
  const originalFetch = globalThis.fetch;
  try {
    for (const [index, c] of cases.entries()) {
      const prompt = JSON.parse(blocks[index].match(/    task: (.+)/)[1]);
      const probe = await prepare(c);
      const fixtureDigest = probe.inputDigest;
      await cleanup(probe);
      const manifest = loadManifest(new URL('./accuracy-manifest.json', import.meta.url));
      assert.equal(manifest.contract.tasks.find(task => task.taskId === c.taskId)?.fixtureDigest, fixtureDigest);
      const provider = new Provider({ config: { evaluationProfile: 'runtime-accuracy', manifest,
        taskId: c.taskId, fixtureRoot: c.fixtureRoot, fixtureFiles: c.fixtureFiles, timeoutMs: 1000,
        accuracyAcceptance: { definitions,
          admit: async ({ workRoot }) => ({ workRoot, mode: 'restricted-sandbox',
            configurationDigest: 'b'.repeat(64), forbiddenWriteScopes: ['/repository'],
            evidencePath: '/trusted/admission.json', verifiedAt: '2026-10-25T00:00:00Z', digest: 'c'.repeat(64) }),
          settle: async ({ admission }) => ({ writersExited: true, admissionDigest: admission.digest,
            checks: [{ kind: 'non_target_files', pass: true }] }),
        } } });
      for (const variant of ['correct', 'wrong', 'missing-done']) {
        globalThis.fetch = async (_url, options) => {
          assert.equal(JSON.parse(options.body).messages.at(-1).content, prompt);
          const workRoot = options.headers['X-VibeWindow-Directory'];
          if (c.taskId !== 'finish-02') {
            for (const [file, bytes] of Object.entries(c.referenceFiles)) await writeFile(join(workRoot, file), bytes);
          }
          if (variant === 'wrong') {
            if (c.missingFile) await rm(join(workRoot, c.missingFile));
            else await writeFile(join(workRoot, 'result.txt'), 'incorrect\n');
          } else if (['finish-01', 'finish-02'].includes(c.taskId)) {
            // Execute the exact public validation, including the no-write case.
            const command = prompt.match(/python3 -c '([^']+)'/)[1];
            execFileSync('python3', ['-c', command], { cwd: workRoot });
          }
          const events = [{ type: 'chat.delta', delta: '完成' }];
          if (variant !== 'missing-done') events.push({ type: 'chat.done', session_id: 'finish-session',
            message_id: 'finish-answer', usage: { input_tokens: 2, output_tokens: 3 } });
          return new Response(events.map(event => `data: ${JSON.stringify(event)}\n\n`).join(''),
            { headers: { 'Content-Type': 'text/event-stream' } });
        };
        const result = await provider.callApi(prompt);
        try {
          assert.equal(result.metadata.oracle.pass, variant !== 'wrong', `${c.taskId}: ${variant} bytes`);
          assert.equal(result.metadata.terminal.pass, variant !== 'missing-done', `${c.taskId}: ${variant} terminal`);
          assert.equal(evaluate(result.output, { providerResponse: result }).pass, variant === 'correct');
          assert.equal(result.metadata.cleanup, 'cleaned');
        } finally {
          if (provider.attempts.has(result.metadata.attemptId)) await provider.cleanupAttempt(result.metadata.attemptId);
        }
      }
    }
  } finally { globalThis.fetch = originalFetch; }
});

// Bytes alone cannot prove that an already-correct file was not rewritten.
// Reuse the existing read-only trace assertion as a supplementary gate.
test('finish_existing_result_rejects_rewrite_even_with_correct_bytes', () => {
  const context = toolCalls => ({ providerResponse: { metadata: { toolCalls } } });
  assert.equal(readOnlyTrajectory('', context([{ name: 'exec_command', status: 'completed',
    input: { cmd: 'python3 -c \'from pathlib import Path; assert Path("result.txt").read_bytes() == b"status=ready\\n"\'' } }])).pass, true);
  for (const tool of [
    { name: 'write_file', input: { path: 'result.txt', content: 'status=ready\n' } },
    { name: 'exec_command', input: { cmd: 'touch result.txt' } },
    { name: 'apply_patch', input: 'same bytes' },
  ]) assert.equal(readOnlyTrajectory('', context([tool])).pass, false);
});
