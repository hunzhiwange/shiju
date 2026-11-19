import { AsyncLocalStorage } from 'node:async_hooks';
import { randomUUID } from 'node:crypto';
import { isAbsolute } from 'node:path';
import { Client } from '@modelcontextprotocol/sdk/client/index.js';
import { StdioClientTransport } from '@modelcontextprotocol/sdk/client/stdio.js';

const scope = new AsyncLocalStorage();
const plans = new WeakMap();
const running = new WeakMap();
const key = /^[A-Za-z0-9_-]+$/;

export function validate(servers = []) {
  const names = new Set();
  for (const server of servers) {
    if (server.type || !isAbsolute(server.command ?? '') || !key.test(server.name) ||
        !Array.isArray(server.args) || server.env?.length || names.has(server.name)) {
      throw new Error('Dirac MCP prototype requires unique absolute stdio commands without custom environment');
    }
    names.add(server.name);
  }
  return servers;
}

export function save(session, servers) {
  if (servers.length) plans.set(session, servers);
}

export function hasMounts(session) {
  return plans.has(session);
}

export function stop(session) {
  running.get(session)?.abort();
}

async function discover(server, cwd, signal, clients) {
  const client = new Client({ name: 'shiju-dirac', version: '1' });
  clients.push(client);
  const transport = new StdioClientTransport({ command: server.command, args: server.args, cwd, stderr: 'ignore' });
  let rejectAbort;
  const abort = new Promise((_, reject) => { rejectAbort = () => reject(signal.reason); });
  signal.addEventListener('abort', rejectAbort, { once: true });
  try {
    signal.throwIfAborted();
    await Promise.race([client.connect(transport), abort]);
  } finally {
    signal.removeEventListener('abort', rejectAbort);
  }
  const tools = [];
  const cursors = new Set();
  let cursor;
  do {
    const page = await client.listTools(cursor ? { cursor } : {}, { signal });
    tools.push(...page.tools);
    cursor = page.nextCursor;
    if (cursor && cursors.has(cursor)) throw new Error('Dirac MCP discovery repeated a cursor');
    cursors.add(cursor);
  } while (cursor);
  return tools.map(tool => ({ client, service: server.name, tool }));
}

export async function prompt(session, invoke, requestPermission, emit) {
  const servers = plans.get(session);
  if (!servers) return invoke();
  if (running.has(session)) throw new Error('Dirac MCP session is busy');
  const controller = new AbortController();
  const signal = AbortSignal.any([controller.signal, AbortSignal.timeout(10000)]);
  const clients = [];
  running.set(session, controller);
  try {
    const registrations = [];
    for (const server of servers) registrations.push(...await discover(server, session.cwd, signal, clients));
    if (!registrations.length) throw new Error('Dirac MCP discovery returned no tools');
    let queue = Promise.resolve();
    const tools = registrations.map(({ client, service, tool }) => {
      const name = service + '-' + tool.name;
      if (!key.test(name) || name.length > 64) throw new Error('Invalid Dirac MCP tool name');
      const spec = {
        id: name, name, description: tool.description ?? tool.name,
        parameters: Object.entries(tool.inputSchema.properties ?? {}).map(([name, schema]) => ({
          ...schema, name, instruction: schema.description ?? name,
          required: tool.inputSchema.required?.includes(name) ?? false,
        })),
      };
      const processCall = async rawInput => {
        const fields = { toolCallId: randomUUID(), title: name };
        const update = (status, rawOutput) => emit({
          sessionUpdate: 'tool_call_update', ...fields, status, rawOutput,
        });
        controller.signal.throwIfAborted();
        await emit({ sessionUpdate: 'tool_call', ...fields, kind: 'other', status: 'pending', rawInput });
        try {
          const decision = await requestPermission({ ...fields, rawInput }, [
            { optionId: 'approve_once', name: 'Approve once', kind: 'allow_once' },
            { optionId: 'reject_once', name: 'Reject', kind: 'reject_once' },
          ]);
          controller.signal.throwIfAborted();
          if (decision.outcome?.outcome !== 'selected' || decision.outcome.optionId !== 'approve_once') {
            throw new Error('Permission denied by user');
          }
          await update('in_progress');
          const result = await client.callTool({ name: tool.name, arguments: rawInput }, undefined, {
            signal: AbortSignal.any([controller.signal, AbortSignal.timeout(30000)]),
          });
          const output = JSON.stringify(result.structuredContent ?? result.content);
          await update(result.isError ? 'failed' : 'completed', output);
          return output;
        } catch (error) {
          const message = error instanceof Error ? error.message : String(error);
          await update('failed', message);
          return message;
        }
      };
      return {
        id: name, name, source: 'task', exposure: { kind: 'configurable' }, spec,
        factory: () => ({ spec: () => spec, supportedSurfaces: () => ['all'], processCall: rawInput => {
          const result = queue.then(() => processCall(rawInput));
          queue = result.catch(() => {});
          return result;
        }}),
      };
    });
    return await scope.run({ tools, signal: controller.signal }, invoke);
  } finally {
    controller.abort();
    running.delete(session);
    await Promise.allSettled(clients.map(client => client.close()));
  }
}

export function expose(existing) {
  const active = scope.getStore();
  if (!active) return existing;
  active.signal.throwIfAborted();
  const names = new Set(existing.map(tool => tool.spec.name));
  for (const tool of active.tools) {
    if (names.has(tool.name)) throw new Error('Duplicate Dirac MCP tool name: ' + tool.name);
    names.add(tool.name);
  }
  return [...existing, ...active.tools];
}
