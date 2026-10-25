// Offline evidence reconciliation. Never changes Promptfoo grades or raw traces.
import { readFile, writeFile } from 'node:fs/promises';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

export function auditSmoke(result, environment, gatewayLog) {
  const data = result.metadata;
  const sessions = [...new Set([...gatewayLog.matchAll(/sessionID=(ses_[A-Za-z0-9]+)/g)].map(m => m[1]))];
  if (sessions.length !== 1 || environment.attemptId !== data.attemptId
      || environment.workRoot !== data.workRoot || data.directory !== data.workRoot
      || data.endpoint !== environment.endpoint + '/v1/chat/stream'
      || data.admission.workRoot !== data.workRoot
      || data.boundary.admissionDigest !== data.admission.digest
      || environment.model !== data.model) throw new Error('attempt/session/path binding failed');
  if (!environment.writersExited || !environment.endpointClosed || !environment.slotReleased
      || data.boundary.pass !== true || data.cleanup !== 'cleaned') throw new Error('isolation or settlement failed');
  const modelBudgetStop = data.budgetStopped === true && environment.modelCalls === 24
    && !environment.relayError && String(result.error).includes('403');
  const outcome = modelBudgetStop ? 'capability-budget-exhausted'
    : result.error ? 'infrastructure-error' : data.oracle.pass ? 'passed' : 'capability-output-failed';
  return { attemptId: data.attemptId, sessionId: sessions[0], sessionSource: 'own Gateway process log',
    workRoot: data.workRoot, endpoint: data.endpoint, manifestDigest: data.manifestDigest,
    model: environment.model, modelTasks: environment.modelTasks, modelCalls: environment.modelCalls,
    usageStatus: data.usageStatus, terminal: data.terminal, budget: data.budget,
    oracle: data.oracle, boundary: data.boundary, outcome,
    eligibleSmokeEvidence: outcome !== 'infrastructure-error',
    rawGradingUnchanged: true, baselinePass: false };
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  const directory = fileURLToPath(new URL('./results/accuracy-smoke/', import.meta.url));
  const [result, environment, log] = await Promise.all(['attempt.json', 'environment.json', 'gateway.log']
    .map(name => readFile(path.join(directory, name), 'utf8')));
  const audit = auditSmoke(JSON.parse(result), JSON.parse(environment), log);
  await writeFile(path.join(directory, 'audit.json'), JSON.stringify(audit, null, 2) + '\n');
  console.log(JSON.stringify({ outcome: audit.outcome, eligibleSmokeEvidence: audit.eligibleSmokeEvidence }));
}
