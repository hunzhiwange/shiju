import { createHash } from 'node:crypto';
import { readFileSync, writeFileSync, lstatSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { loadManifest, manifestDigest, validateManifest } from './accuracy-manifest.mjs';

const defaultRoot = dirname(fileURLToPath(import.meta.url));
const categories = ['facts', 'edit', 'recovery', 'state', 'finish'];
const evaluatorFiles = ['artifact-oracle.mjs', 'assertions.mjs', 'attempt-workspace.mjs',
  'vibewindow-provider.mjs', 'accuracy-manifest.mjs', 'compare-results.mjs', 'freeze-accuracy.mjs',
  'promptfooconfig.yaml', 'score.sh'];
const budget = { iterations: 24, toolCalls: 32, wallTimeMs: 900000 };

function safePath(file) {
  if (typeof file !== 'string' || file.includes('\\') || file.includes('\0')
      || file.split('/').some(part => !part || part === '.' || part === '..')
      || /^[A-Za-z]:/.test(file)) throw new Error('unsafe public input path');
  return file.split('/');
}
// Explicit allowlists only; never walk the repository or read user configuration.
function read(root, file) {
  let current = root;
  for (const part of safePath(file)) {
    current = join(current, part);
    if (lstatSync(current).isSymbolicLink()) throw new Error('symlink input');
  }
  if (!lstatSync(current).isFile()) throw new Error('input must be a regular file');
  return readFileSync(current);
}
// Exactly the byte framing used by attempt-workspace.prepare, including filenames.
function digestFiles(inputs) {
  const digest = createHash('sha256');
  for (const [file, bytes] of inputs.sort(([a], [b]) => a < b ? -1 : a > b ? 1 : 0))
    digest.update(JSON.stringify([file, bytes.length]) + '\n').update(bytes);
  return digest.digest('hex');
}
function digestPaths(root, files) {
  return digestFiles([...new Set(files)].map(file => [file, read(root, file)]));
}

// Restricted YAML reader for these five suites. Accept only the existing mapping,
// folded task and JSON string/list forms; reject aliases, tags and unknown fields.
function parseSuite(source) {
  return source.trim().split(/\n(?=- description: )/).map(block => {
    const lines = block.split('\n');
    const description = lines.shift().match(/^- description: ([^\n]+)$/)?.[1];
    if (!description || /^[&*!\[{]/.test(description)) throw new Error('invalid description');
    const row = { description, metadata: {}, vars: {} };
    const sections = new Set();
    let section;
    let text;
    let taskLines = 0;
    let assertions = 0;
    for (const line of lines) {
      if (!line.trim()) continue;
      const heading = line.match(/^  (metadata|vars|assert):$/)?.[1];
      if (heading) {
        if (sections.has(heading)) throw new Error('duplicate YAML section');
        sections.add(heading); section = heading; text = undefined; continue;
      }
      if (section === 'assert') {
        const allowed = ['    - type: javascript', '      value: file://./assertions.mjs:readOnlyTrajectory'];
        if (line !== allowed[assertions++]) throw new Error('unsupported assertion');
        continue;
      }
      const field = line.match(/^    (\w+):(?: (.*))?$/);
      if (field) {
        const [, key, raw = ''] = field;
        const allowed = section === 'metadata' ? ['suite', 'batch', 'category', 'split', 'taskId']
          : ['evaluationProfile', 'taskId', 'maxIterations', 'maxToolCalls', 'timeoutMs', 'fixtureFiles', 'task', 'turns'];
        if (!section || !allowed.includes(key) || Object.hasOwn(row[section], key)) throw new Error('unsupported or duplicate YAML field');
        text = undefined;
        if (key === 'fixtureFiles') row.vars[key] = JSON.parse(raw);
        else if (key === 'task') {
          if (raw === '>-') { text = 'task'; row.vars.task = ''; }
          else { row.vars.task = JSON.parse(raw); taskLines++; }
        } else if (key === 'turns') {
          if (raw !== '') throw new Error('unsupported turns');
          row.vars.turns = []; text = 'turns';
        } else {
          if (!/^[A-Za-z0-9-]+$/.test(raw)) throw new Error('unsupported YAML scalar');
          row[section][key] = raw;
        }
      } else if (text === 'task' && /^      \S/.test(line)) {
        row.vars.task += line.slice(6) + '\n'; taskLines++;
      } else if (text === 'turns' && /^      - "/.test(line)) {
        row.vars.turns.push(JSON.parse(line.slice(8)));
      } else throw new Error('unsupported YAML structure');
    }
    if (!taskLines || typeof row.vars.task !== 'string' || !row.vars.task.trim()
        || row.vars.turns?.some(turn => typeof turn !== 'string' || !turn.trim())
        || sections.has('assert') && assertions !== 2) throw new Error('incomplete task');
    return row;
  });
}

export function collect(root = defaultRoot) {
  const tasks = [];
  const sources = [...evaluatorFiles];
  for (const [categoryIndex, category] of categories.entries()) {
    const yaml = `tests/accuracy-${category}.yaml`;
    const oracle = `fixtures/runtime-accuracy/${category}/oracle/definitions.mjs`;
    sources.push(yaml, oracle);
    const rows = parseSuite(read(root, yaml).toString('utf8'));
    // Declarative JSON case arrays only: read the allowlist without executing oracle code.
    const oracleSource = read(root, oracle).toString('utf8');
    const declaration = oracleSource.match(/export const cases = (\[[\s\S]*?\n\])\.map\(/)?.[1];
    if (!declaration) throw new Error('unsupported oracle cases');
    const cases = JSON.parse(declaration);
    if (rows.length !== 4 || cases.length !== 4) throw new Error('matrix requires four tasks per category');
    for (const [index, row] of rows.entries()) {
      const id = `${category}-0${index + 1}`;
      const split = index === 3 ? 'holdout' : 'dev';
      if (row.metadata.taskId !== id || row.vars.taskId !== id || !row.description.includes(id)
          || row.metadata.category !== category || row.metadata.split !== split
          || row.metadata.batch !== String(13 + categoryIndex)
          || row.metadata.suite !== 'runtime-accuracy' || row.vars.evaluationProfile !== 'runtime-accuracy'
          || cases[index].taskId !== id || cases[index].split !== split)
        throw new Error('invalid task identity or split');
      if (row.vars.maxIterations !== String(budget.iterations) || row.vars.maxToolCalls !== String(budget.toolCalls)
          || row.vars.timeoutMs !== String(budget.wallTimeMs)) throw new Error('invalid task budget');
      if (category === 'state' ? row.vars.turns?.length !== 1 : Object.hasOwn(row.vars, 'turns'))
        throw new Error('invalid turn count');
      const files = row.vars.fixtureFiles;
      if (!Array.isArray(files) || !files.length || new Set(files).size !== files.length
          || JSON.stringify(files) !== JSON.stringify(cases[index].fixtureFiles)) throw new Error('private or unexpected public inputs');
      const inputs = files.map(file => {
        safePath(file);
        const bytes = read(root, `fixtures/runtime-accuracy/${category}/${id}/${file}`);
        if (/oracle\/|export const cases|expectedFiles|referenceFiles|fixtureRoot/.test(bytes.toString('utf8')))
          throw new Error('oracle payload in public fixture');
        return [file, bytes];
      });
      if (/oracle\/|expectedFiles|referenceFiles|fixtureRoot/.test(JSON.stringify(row.vars)))
        throw new Error('oracle payload in public prompt');
      tasks.push({ taskId: id, description: row.description, category, split,
        fixtureDigest: digestFiles(inputs),
        oracleDigest: digestPaths(root, [yaml, oracle, 'artifact-oracle.mjs', 'assertions.mjs']),
      });
    }
  }
  return { tasks, evaluatorDigest: digestPaths(root, sources) };
}
function sameTasks(a, b) {
  const keys = ['taskId', 'description', 'category', 'split', 'fixtureDigest', 'oracleDigest'];
  return a.length === b.length && a.every((task, index) => keys.every(key => task[key] === b[index][key]));
}
export function verify(root = defaultRoot) {
  const manifest = loadManifest(join(root, 'accuracy-manifest.json'));
  const current = collect(root);
  const experiment = manifest.contract.experiment;
  if (!sameTasks(manifest.contract.tasks, current.tasks) || experiment.evaluatorDigest !== current.evaluatorDigest
      || Object.keys(budget).some(key => experiment.budget[key] !== budget[key])) throw new Error('frozen suite changed');
  return manifestDigest(manifest);
}
export function freeze(root = defaultRoot) {
  const manifest = loadManifest(join(root, 'accuracy-manifest.json'));
  if (manifest.contract.tasks.length) return verify(root); // No silent baseline replacement.
  const collected = collect(root);
  if (Object.keys(budget).some(key => manifest.contract.experiment.budget[key] !== budget[key])) throw new Error('invalid manifest budget');
  manifest.contract.tasks = collected.tasks;
  manifest.contract.experiment.evaluatorDigest = collected.evaluatorDigest;
  validateManifest(manifest);
  writeFileSync(join(root, 'accuracy-manifest.json'), JSON.stringify(manifest, null, 2) + '\n');
  return manifestDigest(manifest);
}
if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  const [mode, ...extra] = process.argv.slice(2);
  if (['--help', '-h', 'help'].includes(mode) && !extra.length) {
    console.log('node evals/promptfoo/freeze-accuracy.mjs freeze  # write initial suite manifest; refuse replacement\nnode evals/promptfoo/freeze-accuracy.mjs verify  # read-only verification\nPrints accuracy-manifest.mjs manifestDigest, shared with compare. The experiment stays draft until actual model/environment are known.');
  } else {
    try {
      if (extra.length || !['freeze', 'verify'].includes(mode)) throw new Error('expected freeze or verify; see --help');
      console.log((mode === 'freeze' ? freeze : verify)());
    } catch (error) { console.error(error.message); process.exitCode = 1; }
  }
}
