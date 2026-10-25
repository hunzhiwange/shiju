import test from 'node:test';
import assert from 'node:assert/strict';
import { readFile, writeFile, mkdir, rm } from 'node:fs/promises';
import { join } from 'node:path';
import { createHash } from 'node:crypto';
import { createArtifactOracle } from './artifact-oracle.mjs';
import { prepare, cleanup } from './attempt-workspace.mjs';
import { cases, definitions } from './fixtures/runtime-accuracy/state/oracle/definitions.mjs';

const digest = bytes => createHash('sha256').update(bytes).digest('hex');
test('state_oracle_accepts_four_reference_results_and_rejects_mutations', async () => {
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

// Replace only HTTP/model and external admission supervision. Use actual
// fixture copies, provider session handling, cleanup, and byte oracle.
test('state_provider_replays_two_turns_and_isolates_each_attempt', async () => {
  const { default: Provider } = await import('./vibewindow-provider.mjs');
  const { loadManifest } = await import('./accuracy-manifest.mjs');
  const yaml = await readFile(new URL('./tests/accuracy-state.yaml', import.meta.url), 'utf8');
  const blocks = yaml.trim().split(/\n\n+/);
  assert.equal(blocks.length, 4);
  const roots = new Set();
  const fetch = globalThis.fetch;
  try {
    for (const [index, c] of cases.entries()) {
      const block = blocks[index];
      assert.match(block, new RegExp(`taskId: ${c.taskId}`));
      assert.match(block, /category: state/);
      assert.match(block, new RegExp(`split: ${c.split}`));
      const prompt = JSON.parse(block.match(/    task: (.+)/)[1]);
      const turns = [...block.matchAll(/      - (.+)/g)].map(match => JSON.parse(match[1]));
      assert.equal(turns.length, 1);
      const probe = await prepare(c);
      const fixtureDigest = probe.inputDigest;
      await cleanup(probe);
      const manifest = loadManifest(new URL('./accuracy-manifest.json', import.meta.url));
      manifest.contract.tasks.push({ taskId: c.taskId, description: c.taskId,
        category: 'state', split: c.split, fixtureDigest, oracleDigest: 'a'.repeat(64) });
      const provider = new Provider({ config: { evaluationProfile: 'runtime-accuracy',
        manifest, taskId: c.taskId, fixtureRoot: c.fixtureRoot, fixtureFiles: c.fixtureFiles,
        timeoutMs: 1000, accuracyAcceptance: { definitions,
          admit: async ({ workRoot }) => ({ workRoot, mode: 'restricted-sandbox',
            configurationDigest: 'b'.repeat(64), forbiddenWriteScopes: ['/repository'],
            evidencePath: '/trusted/admission.json', verifiedAt: '2026-10-25T00:00:00Z', digest: 'c'.repeat(64) }),
          settle: async ({ admission }) => ({ writersExited: true, admissionDigest: admission.digest,
            checks: [{ kind: 'non_target_files', pass: true }] }),
        } } });
      for (const correct of [true, false]) {
        const requests = [];
        globalThis.fetch = async (_url, options) => {
          const request = JSON.parse(options.body);
          requests.push(request);
          const workRoot = options.headers['X-VibeWindow-Directory'];
          if (requests.length === 1) {
            assert.equal(roots.has(workRoot), false);
            roots.add(workRoot);
            assert.equal(request.session_id, undefined);
            // First-round request of holdout is explicitly superseded later.
            if (c.taskId === 'state-04') await writeFile(join(workRoot, c.target), c.mutation);
          } else {
            assert.equal(request.session_id, 'retained-session');
            await writeFile(join(workRoot, c.target), correct ? c.reference : c.mutation);
          }
          return new Response([
            { type: 'chat.delta', delta: '完成' },
            { type: 'chat.done', session_id: 'retained-session', message_id: `answer-${requests.length}`,
              usage: { input_tokens: 2, output_tokens: 3 } },
          ].map(event => `data: ${JSON.stringify(event)}\n\n`).join(''),
          { headers: { 'Content-Type': 'text/event-stream' } });
        };
        const result = await provider.callApi(prompt, { vars: { turns } });
        try {
          assert.equal(requests.length, 2);
          assert.equal(requests[0].messages.at(-1).content, prompt);
          assert.equal(requests[1].messages.at(-1).content, turns[0]);
          assert.equal(result.metadata.turns.length, 2);
          assert.equal(result.metadata.oracle.pass, correct, c.taskId);
          assert.equal(result.metadata.cleanup, 'cleaned');
        } finally {
          if (provider.attempts.has(result.metadata.attemptId)) await provider.cleanupAttempt(result.metadata.attemptId);
        }
      }
    }
    assert.equal(roots.size, 8);
  } finally { globalThis.fetch = fetch; }
});
