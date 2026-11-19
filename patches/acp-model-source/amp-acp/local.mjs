// Owned loopback control plane for the pinned Amp local ThreadWorker.
import { createServer } from 'node:http';
import { randomBytes } from 'node:crypto';
import { readFile, writeFile, mkdir, rename, lstat } from 'node:fs/promises';
import { gunzipSync } from 'node:zlib';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

export const ampCommand = path.join(path.dirname(fileURLToPath(import.meta.url)), 'amp-cli');
export function localAmpEnvironment(overrides) {
  return { PATH: process.env.PATH, LANG: process.env.LANG || 'en_US.UTF-8', ...(process.env.TZ ? { TZ: process.env.TZ } : {}), ...overrides };
}
export function selectedStdio(servers = []) {
  const selected = Object.create(null);
  for (const server of servers) {
    if (server.type || !/^[A-Za-z0-9]+(?:_[A-Za-z0-9]+)*$/.test(server.name)
        || Object.hasOwn(selected, server.name) || !path.isAbsolute(server.command)) {
      throw Error('Amp requires distinct stdio MCP servers with absolute commands');
    }
    selected[server.name] = {
      command: server.command, args: server.args,
      env: Object.fromEntries(server.env.map(({ name, value }) => [name, value])),
    };
  }
  return selected;
}
const HOST_FIELDS = { Read: ['path', 'read_range'], create_file: ['path', 'content'], edit_file: ['path', 'old_str', 'new_str'], Bash: ['cmd'] };
export class LocalAmp {
  constructor(agent) {
    this.agent = agent;
    this.tokens = new Map();
  }
  async start() {
    if (this.starting) return this.starting;
    this.starting = this.open();
    return this.starting;
  }
  async open() {
    const endpoint = new URL(process.env.SHIJU_AMP_RELAY_URL || '');
    if (endpoint.protocol !== 'http:' || endpoint.hostname !== '127.0.0.1' || !endpoint.port || endpoint.pathname !== '/' || endpoint.search || endpoint.hash || endpoint.username || endpoint.password) throw Error('Amp requires a private loopback model relay');
    if (!process.env.SHIJU_AMP_MODEL || !process.env.SHIJU_AMP_BINDING || !process.env.SHIJU_AMP_RELAY_TOKEN) throw Error('Amp requires a selected session model');
    this.endpoint = endpoint;
    this.model = process.env.SHIJU_AMP_MODEL;
    this.directory = process.env.AMP_ACP_STATE_DIR;
    if (!this.directory || !path.isAbsolute(this.directory)) throw Error('Amp requires isolated absolute state');
    await mkdir(path.join(this.directory, 'threads'), { recursive: true, mode: 0o700 });
    for (const directory of [this.directory, path.join(this.directory, 'threads')]) {
      const info = await lstat(directory);
      if (!info.isDirectory() || info.isSymbolicLink() || (info.mode & 0o777) !== 0o700 || info.uid !== process.getuid()) throw Error('Insecure Amp local state directory');
    }
    this.server = createServer((req, res) => this.handle(req, res).catch(() => {
      if (!res.headersSent) res.writeHead(500, { 'content-type': 'application/json' });
      res.end(JSON.stringify({ error: 'Amp local boundary rejected request' }));
    }));
    await new Promise((resolve, reject) => {
      this.server.once('error', reject);
      this.server.listen(0, '127.0.0.1', resolve);
    });
    this.server.unref();
    this.url = `http://127.0.0.1:${this.server.address().port}`;
  }
  environment(sessionId) {
    let context = [...this.tokens.values()].find(value => value.sessionId === sessionId);
    if (!context) {
      const session = this.agent.sessions.get(sessionId);
      context = { sessionId, token: randomBytes(32).toString('hex'), threadId: session.threadId, tools: new Set() };
      this.tokens.set(context.token, context);
    }
    context.tools.clear();
    return {
      AMP_URL: this.url, AMP_API_KEY: context.token,
      AMP_CLI_PATH: path.join(path.dirname(fileURLToPath(import.meta.url)), 'amp-cli'),
      SHIJU_AMP_SESSION: sessionId,
      SHIJU_AMP_MCP_SERVERS: JSON.stringify(this.agent.sessions.get(sessionId).mcpConfig || {}),
      AMP_SKIP_UPDATE_CHECK: '1', AMP_DISABLE_PLUGINS: '1',
      AMP_ACP_DISABLE_PLUGIN_LIST: '1', PLUGINS: 'permissions',
      HOME: this.directory, XDG_CONFIG_HOME: path.join(this.directory, 'config'),
      XDG_DATA_HOME: path.join(this.directory, 'data'), XDG_STATE_HOME: path.join(this.directory, 'state'),
      HTTP_PROXY: '', HTTPS_PROXY: '', ALL_PROXY: '', http_proxy: '', https_proxy: '', all_proxy: '',
      NO_PROXY: '127.0.0.1,localhost', no_proxy: '127.0.0.1,localhost',
    };
  }
  async reloadSelection(session, params) {
    if (session.active || session.queue.length || session.reloading) throw Error('Amp session is active');
    const selection = selectedStdio(params.mcpServers);
    if (params.cwd && path.resolve(params.cwd) !== path.resolve(session.cwd)) throw Error('Amp session cwd mismatch');
    session.reloading = true;
    try {
      this.revoke(params.sessionId);
      await this.agent.transport.closeSession(params.sessionId);
      session.mcpConfig = selection;
      session.processStarted = false;
    } finally {
      session.reloading = false;
    }
  }
  revoke(sessionId) {
    for (const [token, context] of this.tokens) if (context.sessionId === sessionId) this.tokens.delete(token);
  }
  close() {
    this.tokens.clear();
    this.server?.closeAllConnections();
    this.server?.close();
  }
  async exportThread(threadId, cwd) {
    if (![...this.agent.sessions.values()].some(session => session.threadId === threadId && path.resolve(session.cwd) === path.resolve(cwd))) throw Error('Amp history identity mismatch');
    const thread = JSON.parse(await readFile(this.threadPath(threadId), 'utf8'));
    if (thread.id !== threadId || !Array.isArray(thread.messages)) throw Error('Invalid Amp persisted history');
    return thread;
  }
  async body(req) {
    const limit = 16 * 1024 * 1024;
    const chunks = []; let size = 0;
    for await (const chunk of req) {
      size += chunk.length;
      if (size > limit) throw Error('Amp control request too large');
      chunks.push(chunk);
    }
    const bytes = Buffer.concat(chunks);
    const encoding = req.headers['content-encoding'];
    if (encoding === 'gzip' && new URL(req.url, this.url).pathname === '/api/internal') {
      return JSON.parse(gunzipSync(bytes, { maxOutputLength: limit }).toString());
    }
    if (encoding && encoding !== 'identity') throw Error('Unsupported Amp control encoding');
    return JSON.parse(bytes.toString());
  }
  json(res, value) {
    res.writeHead(200, { 'content-type': 'application/json' });
    res.end(JSON.stringify(value));
  }
  threadPath(id) {
    if (!/^T-[a-zA-Z0-9-]+$/.test(id)) throw Error('Invalid Amp thread identity');
    return path.join(this.directory, 'threads', `${id}.json`);
  }
  async handle(req, res) {
    const bearer = req.headers.authorization?.startsWith('Bearer ') ? req.headers.authorization.slice(7) : null;
    const context = this.tokens.get(bearer || (req.url === '/api/provider/anthropic/v1/messages' ? req.headers['x-api-key'] : null));
    if (!context || !this.agent.sessions.has(context.sessionId)) { res.writeHead(401); res.end(); return; }
    if (req.method !== 'POST') throw Error('Unsupported Amp control method');
    const body = await this.body(req);
    if (this.tokens.get(context.token) !== context) throw Error('Amp control context was revoked');
    const route = new URL(req.url, this.url).pathname;
    const selection = Object.keys(this.agent.sessions.get(context.sessionId).mcpConfig || {});
    if (route === '/shiju/tool') {
      if (body.sessionId !== context.sessionId) throw Error('Amp tool session mismatch');
      this.json(res, { output: await this.tool(body) }); return;
    }
    if (route === '/shiju/mcp-permission') {
      if (body.sessionId !== context.sessionId || !context.tools.has(body.name)) throw Error('Amp MCP tool is not selected');
      await this.approveTool(body, 'other');
      if (this.tokens.get(context.token) !== context) throw Error('Amp control context was revoked');
      this.json(res, {}); return;
    }
    if (route === '/api/internal') {
      const { method, params = {} } = body;
      switch (method) {
        case 'getUserInfo': this.json(res, { ok: true, result: { id: 'shiju-local', userID: 'shiju-local', email: 'local@localhost', username: 'local', displayName: 'ShiJu local', features: [], organizations: [], workspaces: [] } }); return;
        case 'loadPlugins': this.json(res, { ok: true, result: [] }); return;
        case 'getUserFreeTierStatus': this.json(res, { ok: true, result: { isFreeTier: false } }); return;
        case 'getThread': {
          if (!context.threadId) context.threadId = params.thread;
          if (context.threadId !== params.thread) throw Error('Amp thread identity mismatch');
          let thread;
          try { thread = JSON.parse(await readFile(this.threadPath(params.thread), 'utf8')); }
          catch (e) { if (e.code !== 'ENOENT') throw e; }
          this.json(res, thread ? { ok: true, result: { thread: { data: thread } } } : { ok: false, error: { code: 'thread-not-found' } }); return;
        }
        case 'uploadThread': {
          const thread = params.thread;
          if (thread.id !== context.threadId) throw Error('Amp thread identity mismatch');
          const target = this.threadPath(thread.id);
          const tmp = `${target}.${randomBytes(8).toString('hex')}.tmp`;
          await writeFile(tmp, JSON.stringify(thread), { mode: 0o600, flag: 'wx' });
          await rename(tmp, target);
          this.json(res, { ok: true, result: {} }); return;
        }
        case 'getThreadLinkInfo': this.json(res, { ok: true, result: { usesThreadActors: false } }); return;
        default: throw Error('Unsupported Amp control operation');
      }
    }
    if (route !== '/api/provider/anthropic/v1/messages') throw Error('Unsupported Amp provider route');
    // Includes title and auxiliary inference: every request uses this session's model.
    body.model = this.model;
    const outputLimit = Number(process.env.SHIJU_AMP_OUTPUT_LIMIT || '0');
    if (Number.isSafeInteger(outputLimit) && outputLimit > 0) body.max_tokens = Math.min(body.max_tokens, outputLimit);
    if (process.env.SHIJU_AMP_TEMPERATURE !== 'true') delete body.temperature;
    if (process.env.SHIJU_AMP_REASONING !== 'true') delete body.thinking;
    // This legacy Opus display default is not a selected-model preference.
    if (body.thinking?.display === 'summarized') delete body.thinking.display;
    // The legacy Opus tier's effort default is not a selected-model preference.
    if (body.output_config?.effort !== undefined) {
      delete body.output_config.effort;
      if (Object.keys(body.output_config).length === 0) delete body.output_config;
    }
    if (body.thinking?.type === 'enabled' && body.thinking.budget_tokens >= body.max_tokens) {
      if (body.max_tokens <= 1024) throw Error('Amp reasoning requires a larger model output budget');
      body.thinking.budget_tokens = body.max_tokens - 1;
    }
    if (body.stream) context.tools.clear();
    if (Array.isArray(body.tools)) body.tools = body.tools.filter(tool => {
      if (tool.name === 'set_title') return !body.stream || !selection.length;
      if (Object.hasOwn(HOST_FIELDS, tool.name)) return body.stream || !selection.length;
      const match = /^mcp__([A-Za-z0-9]+(?:_[A-Za-z0-9]+)*)__[A-Za-z0-9_]+$/.exec(tool.name);
      if (!match || !selection.includes(match[1])) return false;
      if (body.stream) context.tools.add(tool.name);
      return true;
    }).map(tool => {
      if (!HOST_FIELDS[tool.name]) return tool;
      const properties = Object.fromEntries(Object.entries(tool.input_schema.properties).filter(([key]) => HOST_FIELDS[tool.name].includes(key)));
      return { ...tool, input_schema: { ...tool.input_schema, properties, additionalProperties: false }, ...(tool.name === 'Read' ? { description: 'Read a UTF-8 text file through the host with a 1-indexed inclusive read_range. Default [1,1000], maximum 10000 lines. Directory listing and binary files are unavailable.' } : {}) };
    });
    const active = this.agent.sessions.get(context.sessionId)?.active;
    if (!active || active.controller.signal.aborted) throw Error('Amp inference outside active turn');
    const abort = new AbortController();
    const cancelUpstream = () => abort.abort();
    active.controller.signal.addEventListener('abort', cancelUpstream, { once: true });
    res.on('close', () => { active.controller.signal.removeEventListener('abort', cancelUpstream); abort.abort(); });
    const upstream = await fetch(new URL('v1/messages', this.endpoint), {
      method: 'POST', redirect: 'error', signal: abort.signal,
      headers: { 'content-type': 'application/json', authorization: `Bearer ${process.env.SHIJU_AMP_RELAY_TOKEN}`, 'x-api-key': process.env.SHIJU_AMP_RELAY_TOKEN, 'anthropic-version': '2023-06-01' },
      body: JSON.stringify(body),
    });
    if (!upstream.ok) {
      await upstream.body?.cancel();
      res.writeHead(upstream.status, { 'content-type': 'application/json' });
      const type = upstream.status === 401 || upstream.status === 403 ? 'authentication_error' : upstream.status === 404 ? 'not_found_error' : 'api_error';
      res.end(JSON.stringify({ type: 'error', error: { type, message: `Selected model request rejected (${upstream.status})` } }));
      return;
    }
    res.writeHead(upstream.status, { 'content-type': upstream.headers.get('content-type') || 'application/json' });
    if (upstream.body) for await (const chunk of upstream.body) {
      if (!res.write(chunk)) await new Promise(resolve => res.once('drain', resolve));
    }
    res.end();
  }
  async approveTool(request, kind, filename) {
    const session = this.agent.sessions.get(request.sessionId);
    const active = session?.active;
    const check = () => { if (!active || active.controller.signal.aborted || session.active !== active) throw Error('Amp turn is no longer active'); };
    check();
    const permission = await this.agent.client.requestPermission({
      sessionId: request.sessionId,
      toolCall: { toolCallId: request.id, title: request.name, kind, status: 'pending', rawInput: request.input || {}, ...(filename ? { locations: [{ path: filename }] } : {}) },
      options: [{ optionId: 'allow_once', name: 'Allow once', kind: 'allow_once' }, { optionId: 'reject_once', name: 'Reject', kind: 'reject_once' }],
    });
    check();
    if (permission.outcome?.outcome !== 'selected' || permission.outcome.optionId !== 'allow_once') throw Error('Host denied Amp tool');
    return { active, check };
  }
  async tool(request) {
    const session = this.agent.sessions.get(request.sessionId);
    const args = request.input || {};
    const name = request.name;
    if (!Object.hasOwn(HOST_FIELDS, name)) throw Error('Amp tool has no host route');
    if (Object.keys(args).some(key => !HOST_FIELDS[name].includes(key))) throw Error('Unsupported Amp tool arguments');
    const file = args.path;
    if (name !== 'Bash' && (typeof file !== 'string' || !file)) throw Error('Amp tool path required');
    const filename = file && path.resolve(session.cwd, file);
    const kind = name === 'Read' ? 'read' : name === 'Bash' ? 'execute' : 'edit';
    const { active, check } = await this.approveTool(request, kind, filename);
    if (name === 'Read') {
      const range = args.read_range || [1, 1000];
      if (!Array.isArray(range) || range.length !== 2 || !range.every(Number.isInteger) || range[0] < 1 || range[1] < range[0] || range[1] - range[0] >= 10000) throw Error('Invalid Amp text read range');
      const result = await this.agent.client.readTextFile({ sessionId: request.sessionId, path: filename, line: range[0], limit: range[1] - range[0] + 1 });
      check(); return result.content.split('\n').map((line, index) => `${range[0] + index}: ${line}`).join('\n');
    }
    if (name === 'create_file') {
      if (typeof args.content !== 'string') throw Error('Amp content required');
      await this.agent.client.writeTextFile({ sessionId: request.sessionId, path: filename, content: args.content });
      check(); return 'File written by host';
    }
    if (name === 'edit_file') {
      if (typeof args.old_str !== 'string' || !args.old_str || typeof args.new_str !== 'string') throw Error('Unsupported Amp edit contract');
      const result = await this.agent.client.readTextFile({ sessionId: request.sessionId, path: filename });
      check();
      const position = result.content.indexOf(args.old_str);
      if (position < 0 || position !== result.content.lastIndexOf(args.old_str)) throw Error('Amp edit must match exactly once');
      await this.agent.client.writeTextFile({ sessionId: request.sessionId, path: filename, content: result.content.replace(args.old_str, () => args.new_str) });
      check(); return 'File edited by host';
    }
    if (typeof args.cmd !== 'string' || !args.cmd) throw Error('Amp command required');
    const terminal = await this.agent.client.createTerminal({ sessionId: request.sessionId, command: '/bin/bash', args: ['--noprofile', '--norc', '-c', args.cmd], cwd: session.cwd, outputByteLimit: 65536 });
    const kill = () => { terminal.kill().catch(() => {}); };
    active.controller.signal.addEventListener('abort', kill, { once: true });
    try {
      // Cancellation can arrive while the host is creating the terminal, before
      // the listener exists. Reap that terminal before rejecting the stale turn.
      if (active.controller.signal.aborted || session.active !== active) await terminal.kill();
      check();
      const exit = await terminal.waitForExit();
      check();
      const output = await terminal.currentOutput();
      check();
      return JSON.stringify({ ...output, exitCode: exit.exitCode, signal: exit.signal });
    } finally { active.controller.signal.removeEventListener('abort', kill); await terminal.release(); }
  }
}
