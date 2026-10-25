import test from 'node:test';
import assert from 'node:assert/strict';
import { replayWrongArtifact, trustedTask, SupervisorClient } from './accuracy-smoke.mjs';

test('smoke_uses_only_edit_01_dev_registry', async () => {
  const task = await trustedTask();
  assert.equal(task.taskId, 'edit-01');
  assert.equal(task.split, 'dev');
  assert.deepEqual([...task.definitions.keys()], ['edit-01']);
  assert.deepEqual(task.fixtureFiles, ['settings.ini', 'notes.txt']);
});
test('smoke_rejects_wrong_target_bytes_without_model', async () => {
  assert.equal((await replayWrongArtifact()).pass, false);
});

import { auditSmoke } from './audit-smoke.mjs';
function evidence() {
  return [{ error: 'HTTP 403', metadata: { attemptId: 'a', workRoot: '/work/a', directory: '/work/a',
    endpoint: 'http://127.0.0.1:1234/v1/chat/stream', model: 'p/m', manifestDigest: 'd',
    admission: { workRoot: '/work/a', digest: 'proof' }, boundary: { pass: true, admissionDigest: 'proof' },
    oracle: { pass: false }, budgetStopped: true, cleanup: 'cleaned' } },
  { attemptId: 'a', workRoot: '/work/a', endpoint: 'http://127.0.0.1:1234', model: 'p/m',
    writersExited: true, endpointClosed: true, slotReleased: true, modelCalls: 24 }, 'sessionID=ses_one'];
}
test('smoke_audit_distinguishes_guarded_budget_exhaustion_from_transport_failure', () => {
  const args = evidence();
  assert.equal(auditSmoke(...args).outcome, 'capability-budget-exhausted');
  args[1].relayError = 'upstream failed';
  assert.equal(auditSmoke(...args).eligibleSmokeEvidence, false);
});
test('smoke_audit_rejects_mixed_attempt_paths_and_multiple_sessions', () => {
  const args = evidence();
  args[0].metadata.directory = '/work/other';
  assert.throws(() => auditSmoke(...args), /binding failed/);
  args[0].metadata.directory = '/work/a';
  args[2] += ' sessionID=ses_two';
  assert.throws(() => auditSmoke(...args), /binding failed/);
});
test('smoke_audit_rejects_unsettled_writers_even_with_a_passing_oracle', () => {
  const args = evidence();
  args[0].metadata.oracle.pass = true;
  args[1].writersExited = false;
  assert.throws(() => auditSmoke(...args), /settlement failed/);
});

test('smoke_supervisor_preserves_admission_failure_instead_of_masking_it_as_cleanup', async () => {
  const client = Object.create(SupervisorClient.prototype);
  client.child = { exitCode: 1, signalCode: null };
  client.exited = Promise.resolve([1, null]);
  client.failure = 'runtime model route unavailable before admission';
  await assert.rejects(client.close(), /runtime model route unavailable/);
});
