import { dirname, join } from 'node:path';
import { createHash } from 'node:crypto';
import { readFileSync, lstatSync, mkdirSync, realpathSync } from 'node:fs';
const file = process.env.SHIJU_NOVA_MODEL_CONFIG;
let binding;
if (file) {
  const st = lstatSync(file);
  if (!st.isFile() || st.isSymbolicLink() || (st.mode & 0o777) !== 0o600 || st.uid !== process.getuid()) throw Error('Insecure Nova model binding');
  binding = JSON.parse(readFileSync(file, 'utf8'));
  const url = new URL(binding.endpoint);
  if (url.protocol !== 'http:' || url.hostname !== '127.0.0.1' || !url.port || url.pathname !== '/' || url.search || url.hash || url.username || url.password || !binding.model || !process.env.SHIJU_NOVA_RELAY_TOKEN) throw Error('Invalid Nova model binding');
}
export const bound = binding;
export function model() {
  return {
    id: binding.model, name: binding.model, provider: 'anthropic', baseUrl: binding.endpoint,
    contextWindow: 128000, maxOutputTokens: binding.outputLimit || 8192,
    pricing: { inputPricePerMillion: 0, outputPricePerMillion: 0 },
    capabilities: { prompting: true, vision: false, toolUse: true, streaming: true, extendedThinking: false, adaptiveThinking: false, complexityClassification: false, structuredOutput: false, supportsTemperature: true },
  };
}

// Explicit state isolation for a consented host-owned runtime; never grant consent here.
export const stateDirectory = process.env.SHIJU_NOVA_STATE_DIRECTORY;
if (stateDirectory) {
  const st = lstatSync(stateDirectory);
  if (!stateDirectory.startsWith('/') || !st.isDirectory() || st.isSymbolicLink() || (st.mode & 0o777) !== 0o700 || st.uid !== process.getuid()) throw Error('Insecure Nova state directory');
}

export function projectDirectory(cwd) {
  if (!binding && !stateDirectory) return null;
  const root = stateDirectory || dirname(file);
  const owner = lstatSync(root);
  if (!owner.isDirectory() || owner.isSymbolicLink() || (owner.mode & 0o777) !== 0o700 || owner.uid !== process.getuid()) throw Error('Insecure Nova project root');
  const dir = join(root, 'shiju-project-' + createHash('sha256').update(realpathSync(cwd)).digest('hex'));
  try { mkdirSync(dir, { mode: 0o700 }); } catch (error) { if (error.code !== 'EEXIST') throw error; }
  const st = lstatSync(dir);
  if (!st.isDirectory() || st.isSymbolicLink() || (st.mode & 0o777) !== 0o700 || st.uid !== process.getuid()) throw Error('Insecure Nova project state');
  return dir;
}
