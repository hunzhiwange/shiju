// Run from the pinned checkout: node --import tsx /absolute/path/verify-state-dir.mjs "$PWD"
import assert from 'node:assert/strict';
import { homedir } from 'node:os';
import { join, resolve } from 'node:path';
import { pathToFileURL } from 'node:url';
const { getPiAcpDir, getPiAcpSessionMapPath } = await import(
  pathToFileURL(resolve(process.argv[2], 'src/acp/paths.ts')));
const previous = process.env.PI_ACP_STATE_DIR;
try {
  delete process.env.PI_ACP_STATE_DIR;
  assert.equal(getPiAcpDir(), join(homedir(), '.pi', 'pi-acp'));
  for (const value of ['', 'relative']) {
    process.env.PI_ACP_STATE_DIR = value;
    assert.throws(() => getPiAcpDir(), /must be absolute/);
  }
  for (const name of ['first', 'second']) {
    const root = resolve(process.argv[2], 'isolated-state', name);
    process.env.PI_ACP_STATE_DIR = root;
    assert.equal(getPiAcpDir(), root);
    assert.equal(getPiAcpSessionMapPath(), join(root, 'session-map.json'));
  }
  console.log('Pi ACP dedicated state directory: passed');
} finally {
  if (previous === undefined) delete process.env.PI_ACP_STATE_DIR;
  else process.env.PI_ACP_STATE_DIR = previous;
}
