// Single edit-01 supervisor binding. No identity/admission material comes from vars.
import { spawn } from 'node:child_process';
import { createInterface } from 'node:readline';
import { once } from 'node:events';
import { readFile, writeFile, mkdir, mkdtemp, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import VibeWindowProvider from './vibewindow-provider.mjs';
import { verify } from './freeze-accuracy.mjs';
import { loadManifest } from './accuracy-manifest.mjs';
import { createArtifactOracle } from './artifact-oracle.mjs';
import { prepare, cleanup } from './attempt-workspace.mjs';

const root = path.dirname(fileURLToPath(import.meta.url));
const results = path.join(root, 'results/accuracy-smoke');
const model = 'token-plan/deepseek-v4-flash-0731';

export async function trustedTask() {
  const source = await readFile(path.join(root, 'fixtures/runtime-accuracy/edit/oracle/definitions.mjs'), 'utf8');
  // Read declarative edit-01 data without importing the module, which eagerly
  // opens every case's fixtures (including holdout).
  const cases = JSON.parse(source.match(/export const cases = (\[[\s\S]*?\n\])\.map\(/)[1]);
  const task = cases.find(c => c.taskId === 'edit-01' && c.split === 'dev');
  const fixtureRoot = path.join(root, 'fixtures/runtime-accuracy/edit/edit-01');
  const unchangedFiles = Object.fromEntries(await Promise.all(task.fixtureFiles.filter(f => f !== task.target)
    .map(async f => [f, await readFile(path.join(fixtureRoot, f))])));
  const definitions = new Map([[task.taskId, { expectedFiles: { [task.target]: Buffer.from(task.reference) },
    unchangedFiles, allowExtraFiles: false }]]);
  return { ...task, fixtureRoot, definitions };
}

export class SupervisorClient {
  constructor() {
    this.child = spawn('python3', [path.join(root, 'accuracy-smoke.py')], { stdio: ['pipe', 'pipe', 'inherit'] });
    this.lines = createInterface({ input: this.child.stdout })[Symbol.asyncIterator]();
    this.exited = once(this.child, 'exit');
  }
  async call(method, args = {}) {
    this.child.stdin.write(JSON.stringify({ method, args }) + '\n');
    const next = await this.lines.next();
    if (next.done) throw new Error('supervisor exited without response');
    const data = JSON.parse(next.value);
    if (data.error) { this.failure = data.error; throw new Error(data.error); }
    return data.result;
  }
  async close() {
    if (this.child.exitCode === null && this.child.signalCode === null) {
      // EOF also triggers cleanup if an earlier RPC failed.
      this.child.stdin.end();
    }
    const [code, signal] = await this.exited;
    if (code !== 0 || signal) throw new Error(this.failure ? `supervisor failed: ${this.failure}` : 'supervisor cleanup failed');
  }
}

let invoked = false;
export default class AccuracySmokeProvider {
  id() { return 'accuracy-smoke-edit-01'; }
  async callApi(prompt, context) {
    if (invoked) throw new Error('one real task only; no retries');
    invoked = true;
    verify();
    const task = await trustedTask();
    const supervisor = new SupervisorClient();
    try {
      const { endpoint } = await supervisor.call('initialize');
      // Provider env overrides must not redirect a supervised run.
      for (const key of Object.keys(process.env)) {
        if (key.startsWith('VIBEWINDOW_EVAL_') || ['VIBEWINDOW_PLATFORM_ID', 'VIBEWINDOW_COMPANY_ID'].includes(key)) delete process.env[key];
      }
      const provider = new VibeWindowProvider({ config: { endpoint, model, agent: 'main',
        platformId: '10000', companyId: '100000000', evaluationProfile: 'runtime-accuracy',
        manifest: loadManifest(path.join(root, 'accuracy-manifest.json')),
        taskId: task.taskId, fixtureRoot: task.fixtureRoot, fixtureFiles: task.fixtureFiles,
        timeoutMs: 900000, maxIterations: 24, maxToolCalls: 32,
        accuracyAcceptance: { definitions: task.definitions,
          admit: async identity => {
            const proof = await supervisor.call('admit', identity);
            await supervisor.call('arm');
            return proof;
          },
          settle: identity => supervisor.call('settle', identity),
        },
      } });
      const result = await provider.callApi(prompt, context);
      result.metadata.actualModel = model;
      result.metadata.usageStatus = result.tokenUsage ? 'reported' : 'missing';
      result.metadata.failureClassification = result.error ? 'infrastructure-or-transport'
        : !result.metadata.boundary?.pass ? 'isolation'
        : !result.metadata.terminal?.pass ? 'terminal'
        : !result.metadata.budget?.pass ? 'budget'
        : !result.metadata.oracle?.pass ? 'capability' : 'none';
      await mkdir(results, { recursive: true });
      await writeFile(path.join(results, 'attempt.json'), JSON.stringify(result, null, 2) + '\n');
      return result;
    } finally { await supervisor.close(); }
  }
}

export async function replayWrongArtifact() {
  const task = await trustedTask();
  const workRoot = await mkdtemp(path.join(tmpdir(), 'vw-smoke-replay-'));
  try {
    for (const [file, bytes] of Object.entries(task.definitions.get(task.taskId).unchangedFiles)) {
      await writeFile(path.join(workRoot, file), bytes);
    }
    await writeFile(path.join(workRoot, task.target), task.mutation);
    const verdict = await createArtifactOracle(task.definitions)({ taskId: task.taskId, workRoot });
    if (verdict.pass) throw new Error('incorrect artifact accepted');
    return verdict;
  } finally { await rm(workRoot, { recursive: true }); }
}

async function main() {
  verify();
  const replay = await replayWrongArtifact();
  await mkdir(results, { recursive: true });
  await writeFile(path.join(results, 'wrong-artifact-replay.json'), JSON.stringify(replay, null, 2));
  if (process.argv[2] === '--preflight') {
    const task = await trustedTask();
    const workspace = await prepare(task);
    const supervisor = new SupervisorClient();
    try {
      await supervisor.call('initialize');
      await supervisor.call('admit', { attemptId: workspace.attemptId, workRoot: workspace.workRoot });
      await supervisor.call('close');
    } finally {
      await supervisor.close();
      await cleanup(workspace);
    }
    return;
  }
  if (process.argv[2] !== '--run') throw new Error('expected --preflight or --run');
  // Keep the frozen score.sh/config/provider byte-identical. The only difference
  // from score.sh's eval invocation is this trusted, evaluator-owned config.
  const config = { description: 'edit-01 isolated smoke', prompts: ['{{task}}'],
    providers: [{ id: 'file://' + fileURLToPath(import.meta.url) }],
    tests: [{ description: 'edit-01 · 精确文件编辑', metadata: { suite: 'runtime-accuracy', split: 'dev', category: 'edit', taskId: 'edit-01' },
      vars: { evaluationProfile: 'runtime-accuracy', maxIterations: 24, maxToolCalls: 32, timeoutMs: 900000,
        task: (await readFile(path.join(root, 'tests/accuracy-edit.yaml'), 'utf8')).split('    task: >-\n')[1].split('\n\n- description:')[0].split('\n').map(s => s.trim()).join('\n') },
      assert: [{ type: 'javascript', value: 'file://' + path.join(root, 'assertions.mjs') + ':evaluate' },
        { type: 'javascript', value: 'file://' + path.join(root, 'assertions.mjs') + ':readOnlyTrajectory' }] }],
    sharing: false };
  const configPath = path.join(results, 'config.json');
  await writeFile(configPath, JSON.stringify(config, null, 2));
  const args = ['--yes', 'promptfoo@0.122.0', 'eval', '-c', configPath,
    '--repeat', '1', '--no-cache', '-j', '1', '--filter-metadata', 'suite=runtime-accuracy',
    '--filter-metadata', 'split=dev', '--filter-metadata', 'category=edit', '--filter-pattern', 'edit-01',
    '-o', path.join(results, 'result.json')];
  await writeFile(path.join(results, 'eval-command.json'), JSON.stringify(['npx', ...args]));
  const child = spawn('npx', args, { stdio: 'inherit' });
  const [code] = await once(child, 'exit');
  process.exitCode = code ?? 1;
}
if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) await main();
