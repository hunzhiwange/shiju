// Private, request-owned local transport for the supplier's own ACP engine.
// No cloud identity or entitlement is fabricated or changed.
import fs from 'node:fs';
import path from 'node:path';
import net from 'node:net';
import dns from 'node:dns';

export const active = !!process.env.SHIJU_QODER_MODEL_CONFIG;
let binding;
function privatePath(file, directory = false) {
  const s = fs.lstatSync(file);
  if (s.isSymbolicLink() || s.uid !== process.getuid() ||
      (s.mode & 0o777) !== (directory ? 0o700 : 0o600) ||
      !(directory ? s.isDirectory() : s.isFile())) throw Error('Insecure Qoder launch material');
}
export function validate() {
  const file = process.env.SHIJU_QODER_MODEL_CONFIG;
  if (!file || !path.isAbsolute(file) || process.platform !== 'darwin' || process.arch !== 'arm64')
    throw Error('Qoder local runtime requires a private Darwin arm64 session binding');
  privatePath(path.dirname(file), true);
  privatePath(file);
  if (!/^vw-session-model:[a-f0-9]{64}$/.test(process.env.VW_ACP_LAUNCH_REFERENCE || ''))
    throw Error('Qoder local runtime requires a session reference');
  const c = JSON.parse(fs.readFileSync(file, 'utf8'));
  const u = new URL(c.endpoint);
  if (u.protocol !== 'http:' || u.hostname !== '127.0.0.1' || !u.port ||
      u.pathname !== '/' || u.search || u.hash || u.username || u.password ||
      typeof c.token !== 'string' || c.token.length < 32 ||
      typeof c.model !== 'string' || !c.model || /[\r\n\0]/.test(c.model))
    throw Error('Invalid Qoder loopback binding');
  if (binding && JSON.stringify(c) !== JSON.stringify(binding)) throw Error('Qoder binding changed');
  const args = process.argv.slice(2);
  const flag = name => args[args.indexOf(name) + 1];
  if (!args.includes('--acp') || flag('--config-dir') !== path.dirname(file) ||
      flag('--setting-sources') !== 'user' || flag('--model') !== 'shiju/' + c.model ||
      args.includes('--settings') || args.includes('--fake-responses'))
    throw Error('Qoder local runtime requires verified launch arguments');
  binding = c;
  return c;
}
export function settings() {
  const c = validate();
  return {model: {name: 'shiju/' + c.model}, general: {enableAutoUpdate: false, maxAttempts: 1},
    providers: {shiju: {protocol: 'openai', baseUrl: new URL(c.endpoint).origin + '/v1', apiKey: c.token,
      model: c.model, models: [{model: c.model, maxOutputTokens: c.outputLimit,
        contextWindow: c.contextLimit, capabilities: {tools: true, vision: false}}]}}};
}

if (active) {
  const c = validate();
  const allowedPort = Number(new URL(c.endpoint).port);
  // Block Qoder's background cloud services before the supplier module initializes.
  // The selected provider can only reach the exact request-owned relay listener.
  const connect = net.Socket.prototype.connect;
  net.Socket.prototype.connect = function (...args) {
    const a = Array.isArray(args[0]) ? args[0] : args;
    const o = a[0];
    const host = typeof o === 'object' ? o.host || o.hostname || 'localhost' :
      typeof a[1] === 'string' ? a[1] : 'localhost';
    const port = typeof o === 'object' ? o.port : o;
    if (typeof o === 'object' && o.path) throw Error('Qoder local runtime refuses IPC network escape');
    if (host !== '127.0.0.1' || Number(port) !== allowedPort)
      throw Error('Qoder local runtime refuses non-session network');
    return connect.apply(this, args);
  };
  const lookup = dns.lookup;
  dns.lookup = function (host, ...args) {
    if (host !== '127.0.0.1') throw Error('Qoder local runtime refuses external DNS');
    return lookup.call(this, host, ...args);
  };
  for (const key of Object.keys(process.env)) {
    if (/^(QODER|QODERCN)_/.test(key) || /^(HTTP|HTTPS|ALL)_PROXY$/i.test(key)) delete process.env[key];
  }
}
