// Keep a selected catalog on one ACP connection and execute through the reviewed stdio sandbox.
import { AsyncLocalStorage } from 'node:async_hooks';
import { execFile } from 'node:child_process';
import { createHash } from 'node:crypto';
import { lstatSync, realpathSync } from 'node:fs';
import { realpath } from 'node:fs/promises';
import { basename, dirname, isAbsolute, normalize } from 'node:path';
import { fileURLToPath } from 'node:url';
import { bound } from './shiju-session-model.mjs';

const helper = fileURLToPath(new URL('./shiju-mcp.py', import.meta.url));
const capability = 'compass.host-tools.v1';
const ownerApproval = 'compass.owner-approval.v1';
const states = new WeakMap();
const sessionMethods = new Set(['session/new', 'newSession', 'session/load', 'loadSession',
  'session/resume', 'resumeSession', 'session/fork', 'forkSession']);
const namePattern = /^[a-zA-Z0-9]+(?:_[a-zA-Z0-9]+)*$/;
const object = value => value !== null && typeof value === 'object' && !Array.isArray(value);
const invalid = message => Object.assign(new Error(message), { code: -32602 });

const skillSessions = new WeakMap();
const skillScope = new AsyncLocalStorage();
const mountedSkills = new WeakSet();

export function isMountedSkill(skill) {
  return mountedSkills.has(skill);
}

function skillSelection(connection, method, params) {
  const meta = params?._meta;
  if (!Object.hasOwn(meta ?? {}, 'shiju.skillMounts')) return;
  const selection = meta['shiju.skillMounts'];
  if (selection?.version !== 1 || !Array.isArray(selection.skills)) {
    throw invalid('Invalid Nova skill selection');
  }
  if (!selection.skills.length) return;
  if (!bound || process.platform !== 'darwin' || process.arch !== 'arm64' ||
      !['session/new', 'session/load'].includes(method) ||
      meta[ownerApproval] !== true || !connection.ownerApprovalRequired) {
    throw invalid('Nova skills require an owned session-model connection with owner approval');
  }

  const names = new Set();
  const root = dirname(selection.skills[0]?.path ?? '');
  const lease = dirname(root);
  if (basename(root) !== 'skills' || !basename(lease).startsWith('shiju-session-skills-')) {
    throw invalid('Invalid Nova session skill view');
  }
  const owner = lstatSync(lease);
  if (!owner.isDirectory() || owner.uid !== process.getuid() || (owner.mode & 0o777) !== 0o700 ||
      !lstatSync(root).isDirectory()) {
    throw invalid('Nova session skill view must be private');
  }
  for (const { name, path } of selection.skills) {
    if (typeof name !== 'string' || name.trim() === '' || name.includes('..') || name.includes('\\') ||
        typeof path !== 'string' || !isAbsolute(path) || normalize(path) !== path ||
        dirname(path) !== root || basename(path) !== name || names.has(name.toLowerCase()) ||
        !lstatSync(path).isSymbolicLink()) {
      throw invalid('Invalid Nova session skill entry');
    }
    names.add(name.toLowerCase());
  }
  return { workspace: realpathSync(params.cwd), skills: selection.skills };
}

export async function dispatchSkills(connection, method, params, dispatch) {
  const state = skillSessions.get(connection);
  if (method === 'initialize' && state?.selection) {
    throw invalid('Nova skills require an initialized exclusive ACP connection');
  }
  if (sessionMethods.has(method)) {
    const selection = skillSelection(connection, method, params);
    if (state && (state.selection || selection)) {
      throw invalid('Nova skills require an exclusive ACP connection');
    }
    if (!selection) {
      skillSessions.set(connection, {});
      return dispatch(params);
    }
    const next = { selection, session: undefined, closed: false };
    skillSessions.set(connection, next);
    const response = await dispatch(params);
    next.session = response?.sessionId ?? params.sessionId;
    if (typeof next.session !== 'string') throw invalid('Nova did not identify the selected skill session');
    return response;
  }
  if (state?.selection && params?.sessionId === state.session) {
    if (['session/close', 'session/delete'].includes(method)) state.closed = true;
    if (state.turn && (state.closed || ['session/cancel', 'cancel'].includes(method))) {
      state.turn.active = false;
    }
    if (['session/prompt', 'prompt'].includes(method)) {
      if (state.closed || state.turn) throw invalid('Nova skills require an idle selected session');
      const turn = { ...state.selection, session: state.session, active: true };
      state.turn = turn;
      try { return await skillScope.run(turn, () => dispatch(params)); }
      finally {
        turn.active = false;
        state.turn = undefined;
      }
    }
  }
  return dispatch(params);
}

export async function appendSkills(cwd, nativeSession, native, scan) {
  const selection = skillScope.getStore();
  if (!selection?.active || (nativeSession && nativeSession !== selection.session) ||
      await realpath(cwd) !== selection.workspace) return native;
  const existing = new Set(native.map(skill => skill.name.toLowerCase()));
  const mounted = [];
  for (const { name, path } of selection.skills) {
    const skill = await scan(path, 'project');
    if (!skill || skill.name.toLowerCase() !== name.toLowerCase()) {
      throw invalid('Nova could not discover the selected skill');
    }
    if (!existing.has(name.toLowerCase())) {
      mountedSkills.add(skill);
      mounted.push(skill);
    }
  }
  return [...native, ...mounted];
}

function operation(request, signal) {
  signal.throwIfAborted();
  return new Promise((resolve, reject) => {
    const child = execFile('/usr/bin/python3', ['-I', helper], {
      env: { HOME: process.env.HOME, PATH: '/usr/bin:/bin' },
      maxBuffer: 4 * 1024 * 1024,
      timeout: 20000,
    }, (error, stdout) => {
      signal.removeEventListener('abort', disconnect);
      if (error || signal.aborted) {
        reject(new Error('Nova MCP operation failed or was cancelled'));
        return;
      }
      try { resolve(JSON.parse(stdout)); } catch (error) { reject(error); }
    });
    function disconnect() { child.stdin.end(); }
    child.stdin.on('error', () => child.kill());
    signal.addEventListener('abort', disconnect, { once: true });
    child.stdin.write(JSON.stringify(request) + '\n');
  });
}

async function discover(servers, workspace, allowed, signal) {
  const names = new Set();
  const tools = new Map();
  for (const definition of servers) {
    const { name, command, args, env, type, _meta } = definition;
    if ((type && type !== 'stdio') || !namePattern.test(name) || names.has(name) ||
        typeof command !== 'string' || !isAbsolute(command) || !Array.isArray(args) ||
        !Array.isArray(env) || env.length) {
      throw invalid('Nova MCP requires distinct absolute stdio servers with no environment overrides');
    }
    names.add(name);
    const server = { name, command, args };
    const { allow = [], deny = [] } = _meta?.['shiju.nova.toolPolicy'] ?? {};
    const catalog = await operation({ operation: 'list', workspace, server }, signal);
    for (const descriptor of catalog) {
      const id = `${name}__${descriptor.name}`;
      let policyId = 'mcp__' + id.toLowerCase();
      if (id !== id.toLowerCase() || policyId.length > 64) {
        const hash = createHash('sha256').update(name + '\0' + descriptor.name).digest('hex').slice(0, 16);
        policyId = policyId.slice(0, 47) + '_' + hash;
      }
      if ((allowed && !allowed.has(policyId)) || deny.includes(descriptor.name) || deny.includes(policyId) ||
          (allow.length && !allow.includes(descriptor.name) && !allow.includes(policyId))) continue;
      const tool = 'mcp_shiju_' + id.toLowerCase();
      if (!namePattern.test(descriptor.name) || tool.length > 64 || tools.has(tool) ||
          tools.size >= 256 || descriptor.inputSchema?.type !== 'object' ||
          !object(descriptor.inputSchema.properties)) throw invalid('Invalid Nova MCP tool descriptor');
      tools.set(tool, {
        server, name: descriptor.name,
        descriptor: { name: tool, description: descriptor.description ?? descriptor.name,
          inputSchema: descriptor.inputSchema, requiresApproval: true },
      });
    }
  }
  if (!tools.size) throw invalid('Selected Nova MCP servers have no permitted tools');
  return tools;
}

export async function dispatchMcp(connection, method, params, dispatch) {
  let state = states.get(connection);
  if (method === 'initialize' && state) throw invalid('Nova ACP connection is already initialized for a session');
  if (sessionMethods.has(method)) {
    if (state?.managed) throw invalid('Nova MCP requires a separate ACP connection for each session');
    const servers = params.mcpServers;
    const selected = Array.isArray(servers) && servers.length > 0;
    if (!selected) {
      states.set(connection, { managed: false });
      return dispatch(params);
    }
    if (state || !['session/new', 'session/load'].includes(method) || !bound ||
        process.platform !== 'darwin' || servers.length > 16 ||
        params._meta?.[ownerApproval] !== true || !connection.ownerApprovalRequired) {
      throw invalid('Nova MCP requires an owned session-model connection with owner approval');
    }
    state = { managed: true };
    states.set(connection, state);
    try {
      state.workspace = await realpath(params.cwd);
      const allowlist = params._meta['shiju.nova.mcpAllowlist'];
      if (allowlist !== undefined && (!Array.isArray(allowlist) ||
          !allowlist.every(tool => typeof tool === 'string'))) throw invalid('Invalid Nova MCP allowlist');
      state.tools = await discover(servers, state.workspace,
        allowlist === undefined ? undefined : new Set(allowlist), AbortSignal.timeout(30000));
      await connection.negotiateHostTools({
        version: 1, status: 'ready', tools: [...state.tools.values()].map(tool => tool.descriptor),
      });
      const response = await dispatch({ ...params, mcpServers: [],
        _meta: { ...params._meta, [capability]: true } });
      if (response?._meta?.[capability]?.status !== 'ready') {
        throw invalid('Nova did not activate the selected MCP catalog');
      }
      state.session = response.sessionId ?? params.sessionId;
      if (typeof state.session !== 'string') throw invalid('Nova did not identify the MCP session');
      return response;
    } catch (error) {
      await connection.negotiateHostTools(undefined);
      throw error;
    }
  }

  if (!state?.managed) return dispatch(params);
  if (['session/cancel', 'cancel', 'session/close', 'session/delete'].includes(method)) {
    if (params.sessionId === state.session) {
      state.turn?.controller.abort();
      if (method === 'session/close' || method === 'session/delete') {
        state.session = undefined;
        state.tools.clear();
        await connection.negotiateHostTools(undefined);
      }
    }
    return dispatch(params);
  }
  if (!['session/prompt', 'prompt'].includes(method)) return dispatch(params);
  if (!state.session || params.sessionId !== state.session || state.turn) {
    throw invalid('Nova MCP prompt does not belong to the idle selected session');
  }
  const turn = { controller: new AbortController(), calls: new Set() };
  state.turn = turn;
  try { return await dispatch(params); }
  finally {
    turn.controller.abort();
    state.turn = undefined;
  }
}

export function hasSelectedMcp(connection) {
  return states.get(connection)?.managed === true;
}

export async function callMcp(connection, { sessionId, callId, tool, input }) {
  const state = states.get(connection);
  const turn = state?.turn;
  const target = state?.tools?.get(tool);
  const denied = { schemaVersion: 1, ok: false, error: 'Nova MCP operation denied or cancelled' };
  if (!turn || turn.controller.signal.aborted || sessionId !== state.session || !target ||
      !connection.canCallHostTool(sessionId, tool) || typeof callId !== 'string' || !object(input) ||
      turn.calls.has(callId) || turn.calls.size >= 256) return denied;
  turn.calls.add(callId);
  const arguments_ = structuredClone(input);

  const decision = await connection.sendRequest('permission/request', {
    sessionId,
    toolCall: { toolCallId: callId, title: tool, kind: 'other', rawInput: arguments_ },
    options: [
      { optionId: 'allow_once', name: 'Allow once', kind: 'allow_once' },
      { optionId: 'reject_once', name: 'Reject', kind: 'reject_once' },
    ],
  });
  if (decision?.outcome?.outcome !== 'selected' || decision.outcome.optionId !== 'allow_once' ||
      turn.controller.signal.aborted || state.turn !== turn ||
      !connection.canCallHostTool(sessionId, tool)) return denied;
  const result = await operation({ operation: 'call', workspace: state.workspace,
    server: target.server, name: target.name, arguments: arguments_ }, turn.controller.signal);
  if (result.isError) return { schemaVersion: 1, ok: false, error: 'Selected Nova MCP tool failed' };
  return { schemaVersion: 1, ok: true, result };
}
