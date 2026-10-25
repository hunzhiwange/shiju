import { createHash } from 'node:crypto';
import { readFileSync } from 'node:fs';

const categories = ['facts', 'edit', 'recovery', 'state', 'finish'];

function object(value, keys, path) {
  if (!value || typeof value !== 'object' || Array.isArray(value)) {
    throw new Error(path + ' must be an object');
  }
  for (const key of Object.keys(value)) {
    if (!keys.includes(key)) throw new Error(path + ': unknown field ' + key);
  }
  for (const key of keys) {
    if (!Object.hasOwn(value, key)) throw new Error(path + ': missing ' + key);
  }
}

function string(value, path, frozen) {
  if (typeof value !== 'string' || !value.trim() || (frozen && value === 'unknown')) {
    throw new Error(path + ' must be a known nonempty string');
  }
}

function integer(value, path) {
  if (!Number.isSafeInteger(value) || value <= 0) throw new Error(path + ' must be a positive integer');
}

function hash(value, path, frozen) {
  if (!frozen && value === 'unknown') return;
  if (typeof value !== 'string' || !/^[a-f0-9]{64}$/.test(value)) {
    throw new Error(path + ' must be a SHA-256 digest');
  }
}

/** Validate the versioned contract; run metadata never participates in comparison. */
export function validateManifest(manifest) {
  object(manifest, ['schemaVersion', 'stage', 'contract', 'run'], 'manifest');
  if (manifest.schemaVersion !== 1) throw new Error('unsupported schemaVersion');
  if (!['draft', 'frozen'].includes(manifest.stage)) throw new Error('invalid stage');
  const frozen = manifest.stage === 'frozen';
  object(manifest.contract, ['experiment', 'tasks'], 'contract');
  const e = manifest.contract.experiment;
  object(e, ['model', 'label', 'endpoint', 'budget', 'parameters', 'environmentDigest',
    'contextDigest', 'permissionsDigest', 'evaluatorDigest', 'execution'], 'experiment');
  for (const key of ['model', 'label', 'endpoint']) string(e[key], key, frozen);
  if (e.endpoint !== 'unknown') {
    const endpoint = new URL(e.endpoint);
    if (!['http:', 'https:'].includes(endpoint.protocol) || endpoint.username || endpoint.password
        || endpoint.search || endpoint.hash) throw new Error('endpoint must not contain credentials, query or fragment');
  }
  object(e.budget, ['iterations', 'toolCalls', 'wallTimeMs'], 'budget');
  for (const [key, value] of Object.entries(e.budget)) integer(value, 'budget.' + key);
  object(e.parameters, ['temperature', 'top_p', 'seed', 'max_output_tokens'], 'parameters');
  for (const [key, value] of Object.entries(e.parameters)) {
    if (value === 'provider-default') continue;
    if (!frozen && value === 'unknown') continue;
    if (!Number.isFinite(value)) throw new Error('parameters.' + key + ' must be known and finite');
    if (key === 'temperature' && value < 0) throw new Error('invalid temperature');
    if (key === 'top_p' && (value <= 0 || value > 1)) throw new Error('invalid top_p');
    if (key === 'seed' && !Number.isSafeInteger(value)) throw new Error('invalid seed');
    if (key === 'max_output_tokens') integer(value, key);
  }
  for (const key of ['environmentDigest', 'contextDigest', 'permissionsDigest', 'evaluatorDigest']) {
    hash(e[key], key, frozen);
  }
  object(e.execution, ['repeats', 'concurrency', 'cache'], 'execution');
  integer(e.execution.repeats, 'repeats');
  integer(e.execution.concurrency, 'concurrency');
  if (typeof e.execution.cache !== 'boolean') throw new Error('cache must be boolean');
  if (frozen && (e.execution.repeats !== 3 || e.execution.concurrency !== 1 || e.execution.cache)) {
    throw new Error('frozen execution requires three attempts, concurrency one and no cache');
  }
  if (!Array.isArray(manifest.contract.tasks)) throw new Error('tasks must be an array');
  const ids = new Set();
  for (const task of manifest.contract.tasks) {
    object(task, ['taskId', 'description', 'category', 'split', 'fixtureDigest', 'oracleDigest'], 'task');
    string(task.taskId, 'taskId', frozen);
    string(task.description, 'description', frozen);
    if (ids.has(task.taskId)) throw new Error('duplicate taskId');
    ids.add(task.taskId);
    if (!categories.includes(task.category)) throw new Error('invalid category');
    if (!['dev', 'holdout'].includes(task.split)) throw new Error('invalid split');
    hash(task.fixtureDigest, 'fixtureDigest', frozen);
    hash(task.oracleDigest, 'oracleDigest', frozen);
  }
  if (frozen) {
    if (manifest.contract.tasks.length !== 20) throw new Error('frozen matrix requires 20 tasks');
    for (const category of categories) {
      const tasks = manifest.contract.tasks.filter(task => task.category === category);
      if (tasks.length !== 4 || tasks.some((task, i) => task.split !== (i === 3 ? 'holdout' : 'dev'))) {
        throw new Error('frozen matrix requires three dev then one holdout per category');
      }
    }
  }
  // Run fields are optional before execution, but never accept arbitrary payloads.
  if (!manifest.run || typeof manifest.run !== 'object' || Array.isArray(manifest.run)) {
    throw new Error('run must be an object');
  }
  for (const [key, value] of Object.entries(manifest.run)) {
    if (['sourceCommit', 'sourceDiffDigest', 'time'].includes(key)) {
      string(value, 'run.' + key, false);
      if (key === 'sourceDiffDigest') hash(value, key, false);
    } else if (['processId', 'attempt'].includes(key)) integer(value, 'run.' + key);
    else throw new Error('run: unknown field ' + key);
  }
  return manifest;
}

export function loadManifest(path) {
  return validateManifest(JSON.parse(readFileSync(path, 'utf8')));
}

function stable(value) {
  if (Array.isArray(value)) return '[' + value.map(stable).join(',') + ']';
  if (value && typeof value === 'object') {
    return '{' + Object.keys(value).sort().map(key => JSON.stringify(key) + ':' + stable(value[key])).join(',') + '}';
  }
  return JSON.stringify(value);
}

/** Hash only the comparison contract; source identity, time and attempts belong to run. */
export function manifestDigest(manifest) {
  validateManifest(manifest);
  return createHash('sha256').update(stable(manifest.contract)).digest('hex');
}

/** Draft digests are diagnostic and cannot authorize a formal comparison. */
export function assertComparable(manifest) {
  validateManifest(manifest);
  if (manifest.stage !== 'frozen') throw new Error('draft manifest cannot be used for formal comparison');
  return manifestDigest(manifest);
}
