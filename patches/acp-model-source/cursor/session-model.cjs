// Fixed Cursor ACP runtime: Chat inference with native shell and MCP execution and blob history.
'use strict';
const { readFileSync, lstatSync } = require('node:fs');
const { createHash } = require('node:crypto');
const { isAbsolute } = require('node:path');
const toolStepLimit = 2000;

function validateConfig(c) {
  const u = new URL(c.endpoint);
  if (u.protocol !== 'http:' || u.hostname !== '127.0.0.1' || !u.port || u.pathname !== '/' || u.username || u.password || u.search || u.hash)
    throw new Error('Cursor requires a loopback relay');
  if (!c.model || !c.token || !Number.isSafeInteger(c.outputLimit) || c.outputLimit < 1)
    throw new Error('Invalid Cursor session model binding');
  return c;
}
function config() {
  const path = process.env.SHIJU_CURSOR_MODEL_CONFIG;
  const stat = lstatSync(path);
  if (!stat.isFile() || stat.isSymbolicLink() || (stat.mode & 0o777) !== 0o600 || stat.uid !== process.getuid())
    throw new Error('Insecure Cursor model configuration');
  return validateConfig(JSON.parse(readFileSync(path, 'utf8')));
}
function proto(exports, name) {
  const type = Object.values(exports).find(t => typeof t === 'function' && t.typeName === `agent.v1.${name}`);
  if (!type) throw new Error('Cursor reviewed protobuf contract mismatch');
  return type;
}
const tool = { type: 'function', function: {
  name: 'shell', description: 'Run a foreground command with Cursor native shell permissions. Inspect files and create or edit files using shell commands. Do not start background jobs.',
  parameters: { type: 'object', properties: { command: { type: 'string' } }, required: ['command'], additionalProperties: false },
} };

function shellArgs(call, cwd) {
  if (call.name !== 'shell') throw new Error('Cursor unsupported returned tool');
  let a;
  try { a = JSON.parse(call.arguments); } catch { throw new Error('Cursor invalid tool arguments'); }
  if (!a || Object.keys(a).length !== 1 || typeof a.command !== 'string' || !a.command.trim())
    throw new Error('Cursor invalid shell arguments');
  return { command: a.command, workingDirectory: cwd, timeout: 60000, toolCallId: call.id };
}

function mcpTools(descriptors) {
  const tools = new Map();
  for (const descriptor of descriptors) {
    const { name, providerIdentifier, toolName, clientKey, description, inputSchema } = descriptor;
    if (!/^[a-zA-Z0-9_-]{1,64}$/.test(name) || name === 'shell' || tools.has(name)
        || !providerIdentifier || !toolName || !clientKey || inputSchema?.type !== 'object')
      throw new Error('Cursor unsupported or ambiguous MCP tool descriptor');
    tools.set(name, {
      definition: { type: 'function', function: { name, description, parameters: inputSchema } },
      target: { name, providerIdentifier, toolName, serverIdentifier: clientKey },
    });
  }
  return tools;
}

function mcpArgs(call, target) {
  let args;
  try { args = JSON.parse(call.arguments); } catch { throw new Error('Cursor invalid tool arguments'); }
  if (!args || typeof args !== 'object' || Array.isArray(args))
    throw new Error('Cursor invalid MCP tool arguments');
  return { ...target, args, toolCallId: call.id };
}

async function* parseStream(body, tools = new Map()) {
  const decoder = new TextDecoder(); let buffer = '', done = false, finish, call, content = '', reasoning = '';
  function consume(line) {
    if (!line.startsWith('data:')) return;
    if (done) throw new Error('Cursor data after model terminal');
    const data = line.slice(5).trim();
    if (data === '[DONE]') { done = true; return; }
    let event;
    try { event = JSON.parse(data); } catch { throw new Error('Cursor invalid model stream'); }
    if (event.error) throw new Error('Cursor model stream failed');
    if ((event.choices?.length ?? 0) > 1) throw new Error('Cursor multiple model choices unsupported');
    const choice = event.choices?.[0]; if (!choice) return;
    const d = choice.delta ?? {};
    if (finish && (Object.keys(d).length || choice.finish_reason)) throw new Error('Cursor data after finish reason');
    if (d.refusal || d.audio || d.function_call) throw new Error('Cursor unsupported model content');
    if (d.content != null && typeof d.content !== 'string' || d.reasoning_content != null && typeof d.reasoning_content !== 'string')
      throw new Error('Cursor unsupported model content');
    content += d.content ?? ''; reasoning += d.reasoning_content ?? '';
    for (const t of d.tool_calls ?? []) {
      if (t.index !== 0 || t.type && t.type !== 'function') throw new Error('Cursor parallel or unsupported tool calls');
      call ??= { id: '', name: '', arguments: '' };
      call.id += t.id ?? ''; call.name += t.function?.name ?? ''; call.arguments += t.function?.arguments ?? '';
    }
    if (choice.finish_reason) finish = choice.finish_reason;
    return { text: d.content ?? '', reasoning: d.reasoning_content ?? '' };
  }
  for await (const chunk of body) {
    buffer += decoder.decode(chunk, { stream: true }); let end;
    while ((end = buffer.indexOf('\n')) >= 0) {
      const delta = consume(buffer.slice(0, end).replace(/\r$/, '')); buffer = buffer.slice(end + 1);
      if (delta) yield delta;
    }
  }
  buffer += decoder.decode();
  if (buffer.trim()) { const delta = consume(buffer.trim()); if (delta) yield delta; }
  if (!done || !['stop', 'tool_calls'].includes(finish) || (finish === 'tool_calls') !== !!call || call && !call.id)
    throw new Error('Cursor incomplete or unsupported model completion');
  if (call && tools.has(call.name)) mcpArgs(call, tools.get(call.name).target);
  else if (call) shellArgs(call, '/');
  yield { completed: true, content, reasoning, call };
}

function history(conversation) {
  const messages = [];
  for (const turn of conversation.turns) {
    if (turn.turn.case !== 'agentConversationTurn') throw new Error('Cursor unsupported history turn');
    const t = turn.turn.value;
    if (t.userMessage.selectedContext?.selectedImages?.length) throw new Error('Cursor images unsupported');
    messages.push({ role: 'user', content: t.userMessage.text });
    let text = '', reasoning = '';
    for (const step of t.steps) {
      if (step.message.case === 'thinkingMessage') reasoning += step.message.value.text;
      else if (step.message.case === 'assistantMessage') text += step.message.value.text;
      else if (step.message.case === 'toolCall') {
        const tc = step.message.value;
        const { args, result } = tc.tool.value ?? {};
        if (!tc.toolCallId || !args || !result?.result.case)
          throw new Error('Cursor unsupported or incomplete history tool');
        let name, parameters;
        if (tc.tool.case === 'shellToolCall') {
          name = 'shell'; parameters = { command: args.command };
        } else if (tc.tool.case === 'mcpToolCall') {
          ({ name, args: parameters = {} } = args.toJson());
          if (!name) throw new Error('Cursor incomplete MCP history tool');
        } else throw new Error('Cursor unsupported history tool');
        messages.push({ role: 'assistant', content: text || null, ...(reasoning ? { reasoning_content: reasoning } : {}),
          tool_calls: [{ id: tc.toolCallId, type: 'function', function: { name, arguments: JSON.stringify(parameters) } }] });
        messages.push({ role: 'tool', tool_call_id: tc.toolCallId, content: JSON.stringify(result.toJson()) });
        text = ''; reasoning = '';
      } else throw new Error('Cursor unsupported history step');
    }
    if (text || reasoning) messages.push({ role: 'assistant', content: text, ...(reasoning ? { reasoning_content: reasoning } : {}) });
  }
  return messages;
}

async function run(api, ctx, state, action, model, updates, resources, blobs, actions, store, mcp, options) {
  const c = config();
  if ((options.requestedModel?.modelId ?? model?.modelId) !== c.model)
    throw new Error('Cursor selected model mismatch');
  if (action.action.case !== 'userMessageAction' || state.summary?.length)
    throw new Error('Cursor unsupported action or summarized history');
  const user = action.action.value.userMessage;
  if (!user || user.selectedContext?.selectedImages?.length || user.selectedContext?.cursorCommands?.length || user.selectedContext?.selectedSkills?.length)
    throw new Error('Cursor unsupported user content');
  if (store.getMetadata('mode') && store.getMetadata('mode') !== 'default')
    throw new Error('Cursor session model supports agent mode only');
  await api.context(ctx, action, resources, store.getId());
  const requestContext = action.action.value.requestContext;
  const cwd = store.getMetadata('workspacePath');
  const mountedTools = mcpTools(mcp);
  const tools = [tool, ...Array.from(mountedTools.values(), entry => entry.definition)];
  const messages = [{ role: 'system', content: [
    'You are Cursor, a coding assistant. Use the available tools as requested. Follow the user write scope.',
    'When using the shell tool, run foreground commands only.',
    'If the user specifies an exact fallback response and its condition is met, return only that response.',
    'Stop gathering evidence once it answers the request. Prefer existing tool results to redundant lookups.',
    'For proposed checks, state the setup and what distinguishes each outcome. Report predictions as predictions.',
    'Workspace and rules:',
    JSON.stringify(requestContext?.toJson() ?? {}),
  ].join('\n') },
    ...history(await store.getFullConversation(ctx)), { role: 'user', content: user.text }];
  const make = (name, value) => proto(api.pb, name).fromJson(value);
  const put = async value => {
    const bytes = value.toBinary(), id = createHash('sha256').update(bytes).digest();
    await blobs.setBlob(ctx, id, bytes); return id;
  };
  const stepIds = [], userId = await put(user);
  const save = async () => {
    const turn = make('ConversationTurnStructure', { agentConversationTurn: { userMessage: userId.toString('base64'), steps: stepIds.map(id => id.toString('base64')) } });
    const checkpoint = proto(api.pb, 'ConversationStateStructure').fromBinary(state.toBinary());
    checkpoint.turns.push(await put(turn)); await store.handleCheckpoint(ctx, checkpoint);
  };
  const step = async value => stepIds.push(await put(make('ConversationStep', value)));
  const exec = api.exec.fromResources(resources);
  await updates.sendUpdate(ctx, api.updates.userMessageAppended(user));
  try {
    for (let index = 0; index <= toolStepLimit; index++) {
      ctx.signal.throwIfAborted();
      const finalOnly = index === toolStepLimit;
      const requestMessages = finalOnly ? [
        { ...messages[0], content: messages[0].content + '\nThe tool execution limit has been reached. Answer using the results already collected and identify any remaining gaps.' },
        ...messages.slice(1),
      ] : messages;
      const response = await fetch(new URL('v1/chat/completions', c.endpoint), {
        method: 'POST', headers: { 'Content-Type': 'application/json', Authorization: `Bearer ${c.token}` },
        body: JSON.stringify({ model: c.model, messages: requestMessages, tools, parallel_tool_calls: false,
          ...(finalOnly ? { tool_choice: 'none' } : {}), stream: true, max_tokens: c.outputLimit }),
        signal: ctx.signal, redirect: 'error',
      });
      if (!response.ok) { await response.body?.cancel(); throw new Error(`Cursor model request failed (HTTP ${response.status})`); }
      let result;
      for await (const delta of parseStream(response.body, mountedTools)) {
        if (delta.text) await updates.sendUpdate(ctx, api.updates.textDelta(delta.text));
        if (delta.reasoning) await updates.sendUpdate(ctx, api.updates.thinkingDelta(delta.reasoning));
        if (delta.completed) result = delta;
      }
      if (result.reasoning) await step({ thinkingMessage: { text: result.reasoning } });
      if (result.content) await step({ assistantMessage: { text: result.content } });
      if (!result.call) {
        if (!result.content) throw new Error('Cursor empty assistant completion');
        await save(); await updates.sendUpdate(ctx, api.updates.turnEnded()); return;
      }
      if (finalOnly) throw new Error('Cursor model requested a tool after reaching the step limit');
      const call = result.call;
      const mounted = mountedTools.get(call.name);
      if (!mounted && (typeof cwd !== 'string' || !isAbsolute(cwd)))
        throw new Error('Cursor ACP session working directory is unavailable');
      const args = mounted ? mcpArgs(call, mounted.target) : shellArgs(call, cwd);
      if (!mounted) {
        const parsed = api.parseShell(args.command);
        Object.assign(args, parsed.legacy, { parsingResult: parsed.structured });
      }
      const toolCase = mounted ? 'mcpToolCall' : 'shellToolCall';
      const argsCase = mounted ? 'mcpArgs' : 'shellArgs';
      const resultCase = mounted ? 'mcpResult' : 'shellResult';
      const tc = make('ToolCall', { toolCallId: call.id, [toolCase]: { args } });
      await updates.sendUpdate(ctx, api.updates.toolCallStarted(call.id, tc));
      const request = api.ExecServerMessage.fromJson({ id: index + 1, execId: call.id, [argsCase]: args });
      let toolResult;
      for await (const reply of exec.handle(ctx, request)) {
        if (reply.message.case === 'streamClose' && reply.message.value.id === index + 1 && toolResult) continue;
        if (reply.message.case !== resultCase || reply.id !== index + 1 || toolResult) throw new Error('Cursor native tool response mismatch');
        toolResult = reply.message.value;
      }
      if (!toolResult?.result.case) throw new Error('Cursor missing native tool result');
      tc.tool.value.result = toolResult;
      await step({ toolCall: tc.toJson() }); await save();
      await updates.sendUpdate(ctx, api.updates.toolCallCompleted(call.id, tc));
      messages.push({ role: 'assistant', content: result.content || null, ...(result.reasoning ? { reasoning_content: result.reasoning } : {}),
        tool_calls: [{ id: call.id, type: 'function', function: { name: call.name, arguments: call.arguments } }] },
      { role: 'tool', tool_call_id: call.id, content: JSON.stringify(toolResult.toJson()) });
    }
  } catch (error) { await save(); throw error; }
}

function modelManager(ModelManager, pb, provider) {
  const c = config(), manager = new ModelManager(provider);
  const model = proto(pb, 'ModelDetails').fromJson({ modelId: c.model, displayModelId: c.model, displayName: c.model, displayNameShort: c.model });
  manager.availableModels = [model]; manager.currentModel = model; manager.models = Promise.resolve(model);
  const catalog = [{ name: c.model, clientDisplayName: c.model, parameterDefinitions: [],
    variants: [{ variantStringRepresentation: c.model, legacySlug: c.model, displayName: c.model, parameterValues: [], isDefaultNonMaxConfig: true }] }];
  manager.parameterizedModels = catalog; manager.buildParameterizedModelMap(catalog);
  manager.currentSelectedModel = { modelId: c.model, parameters: [] };
  manager.aiServerClient = { availableModels: async () => ({ models: catalog }) };
  manager.buildAliasMap([model]); return manager;
}
function unavailableServices() {
  return new Proxy({}, { get: (_, name) => async () => { throw new Error(`Cursor remote service unavailable in session model mode: ${String(name)}`); } });
}
function toolCallStatus(call) {
  if (!process.env.SHIJU_CURSOR_MODEL_CONFIG || call.tool.case !== 'mcpToolCall') return 'completed';
  const result = call.tool.value.result?.result;
  return result?.case === 'success' && !result.value.isError ? 'completed' : 'failed';
}
module.exports = { validateConfig, parseStream, shellArgs, history, run, modelManager, config, unavailableServices, toolCallStatus };
