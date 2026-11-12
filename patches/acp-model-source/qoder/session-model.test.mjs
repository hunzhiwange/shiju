import {test} from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import {spawnSync} from 'node:child_process';
import {fileURLToPath} from 'node:url';

const modulePath = fileURLToPath(new URL('./session-model.mjs', import.meta.url));
function probe(change, body = '') {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), 'qoder-boundary-'));
  fs.chmodSync(root, 0o700);
  const config = {endpoint: 'http://127.0.0.1:12345', token: 'x'.repeat(40),
    model: 'selected-model', outputLimit: 4096, contextLimit: 32768};
  const file = path.join(root, 'model.json');
  fs.writeFileSync(file, JSON.stringify(config), {mode: 0o600});
  const env = {...process.env, SHIJU_QODER_MODEL_CONFIG: file,
    VW_ACP_LAUNCH_REFERENCE: 'vw-session-model:' + 'a'.repeat(64)};
  const wrapper = path.join(root, 'probe.mjs');
  fs.writeFileSync(wrapper, `import * as m from ${JSON.stringify(modulePath)};\n${body}`);
  change?.({root, file, config, env});
  try {
    return spawnSync(process.execPath, [wrapper, '--acp', '--config-dir', root,
      '--setting-sources', 'user', '--model', 'shiju/selected-model'],
    {env, encoding: 'utf8', timeout: 10000});
  } finally { fs.rmSync(root, {recursive: true, force: true}); }
}
const options = {skip: process.platform !== 'darwin' || process.arch !== 'arm64'};
test('binding requires private files and an owning session', options, () => {
  for (const change of [
    ({file}) => fs.chmodSync(file, 0o644),
    ({root}) => fs.chmodSync(root, 0o755),
    ({env}) => delete env.VW_ACP_LAUNCH_REFERENCE,
    ({file, config}) => fs.writeFileSync(file, JSON.stringify({...config, endpoint: 'https://example.invalid'})),
    ({file}) => { fs.renameSync(file, file + '.real'); fs.symlinkSync(file + '.real', file); },
  ]) assert.notEqual(probe(change).status, 0);
});
test('local transport blocks official hosts and unrelated loopback ports', options, () => {
  const result = probe(null, `
    import net from 'node:net';
    import dns from 'node:dns';
    for (const [host,port] of [['api2.qoder.sh',443], ['127.0.0.1',12346]]) {
      let denied = false;
      try { net.connect({host,port}); } catch { denied = true; }
      if (!denied) throw Error('network escaped');
    }
    let denied = false;
    try { dns.lookup('api2.qoder.sh',()=>{}); } catch { denied = true; }
    if (!denied) throw Error('DNS escaped');
    if (Object.keys(m.settings().providers).join() !== 'shiju') throw Error('model catalog escaped');
    if (process.env.QODER_PERSONAL_ACCESS_TOKEN) throw Error('ambient auth leaked');
  `);
  assert.equal(result.status, 0, result.stderr);
});
test('revoked launch material cannot prepare another prompt', options, () => {
  const result = probe(null, `
    import fs from 'node:fs';
    fs.unlinkSync(process.env.SHIJU_QODER_MODEL_CONFIG);
    let denied = false;
    try { m.validate(); } catch { denied = true; }
    if (!denied) throw Error('revoked binding accepted');
  `);
  assert.equal(result.status, 0, result.stderr);
});
