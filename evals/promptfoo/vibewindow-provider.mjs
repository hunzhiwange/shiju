import path from 'node:path';
import { randomUUID } from 'node:crypto';
import { fileURLToPath } from 'node:url';

const DEFAULT_ENDPOINT = 'http://127.0.0.1:42617/v1/chat/stream';
const DEFAULT_MODEL = 'token-plan/deepseek-v4-flash-0731';
const DEFAULT_TIMEOUT_MS = 900_000;
const DEFAULT_MAX_ITERATIONS = 24;
const DEFAULT_MAX_TOOL_CALLS = 32;
const MAX_ERROR_BODY_CHARS = 1_024;
const REPOSITORY_ROOT = path.resolve(fileURLToPath(new URL('../..', import.meta.url)));

function nonEmpty(value) {
  return typeof value === 'string' && value.trim() ? value.trim() : undefined;
}

function setting(config, envName, fallback) {
  return nonEmpty(process.env[envName.environment]) ?? nonEmpty(config?.[envName.config]) ?? fallback;
}

function positiveInteger(value, fallback) {
  const parsed = Number.parseInt(String(value ?? ''), 10);
  return Number.isSafeInteger(parsed) && parsed > 0 ? parsed : fallback;
}

function parseMaybeJson(value) {
  if (typeof value !== 'string') {
    return value;
  }
  try {
    return JSON.parse(value);
  } catch {
    return value;
  }
}

export function resolveTimeoutMs(config = {}, environment = process.env) {
  return positiveInteger(
    environment.VIBEWINDOW_EVAL_TIMEOUT_MS ?? config.timeoutMs,
    DEFAULT_TIMEOUT_MS,
  );
}

function normalizeUsage(usage) {
  return {
    inputTokens: Number(usage?.input_tokens ?? 0),
    outputTokens: Number(usage?.output_tokens ?? 0),
    cachedTokens: Number(usage?.cached_tokens ?? 0),
    reasoningTokens: Number(usage?.reasoning_tokens ?? 0),
  };
}

function reportProgress(event) {
  const type = event?.type;
  let detail = '';
  if (type === 'chat.step_start') {
    detail = `step=${event.step_index}`;
  } else if (type === 'chat.step_finish') {
    detail = `step=${event.step_index} reason=${event.finish_reason ?? 'unknown'}`;
  } else if (type === 'chat.tool') {
    detail = `${event.kind ?? event.status ?? 'update'} ${event.name ?? 'unknown'}`;
  } else if (type === 'chat.error') {
    detail = String(event.error ?? 'unknown error');
  } else if (type !== 'chat.done') {
    return;
  }
  process.stderr.write(`[vibewindow-eval] ${type}${detail ? ` ${detail}` : ''}\n`);
}

function stopEndpoint(endpoint, requestId) {
  const url = new URL(endpoint);
  url.pathname = `${url.pathname.replace(/\/$/, '')}/${encodeURIComponent(requestId)}/stop`;
  url.search = '';
  return url;
}

async function stopStream(endpoint, requestId, headers) {
  const response = await fetch(stopEndpoint(endpoint, requestId), {
    method: 'POST',
    headers,
    signal: AbortSignal.timeout(5_000),
  });
  if (!response.ok) {
    throw new Error(`VibeWindow stop request returned ${response.status} ${response.statusText}`);
  }
  const result = await response.json();
  if (result.cancelled !== true) {
    throw new Error('VibeWindow stop request did not find the active stream');
  }
}

function parseSseFrame(frame) {
  const data = frame
    .split(/\r?\n/)
    .filter((line) => line.startsWith('data:'))
    .map((line) => line.slice(5).replace(/^ /, ''))
    .join('\n');
  if (!data) {
    return undefined;
  }
  try {
    return JSON.parse(data);
  } catch (error) {
    throw new Error(`invalid VibeWindow SSE JSON: ${error.message}`);
  }
}

export async function* parseSseStream(body) {
  if (!body) {
    throw new Error('VibeWindow response did not include an SSE body');
  }

  const reader = body.getReader();
  const decoder = new TextDecoder();
  let buffer = '';

  try {
    while (true) {
      const { value, done } = await reader.read();
      buffer += decoder.decode(value, { stream: !done });

      let separator = buffer.search(/\r?\n\r?\n/);
      while (separator >= 0) {
        const match = buffer.slice(separator).match(/^\r?\n\r?\n/);
        const frame = buffer.slice(0, separator);
        buffer = buffer.slice(separator + match[0].length);
        const event = parseSseFrame(frame);
        if (event !== undefined) {
          yield event;
        }
        separator = buffer.search(/\r?\n\r?\n/);
      }

      if (done) {
        break;
      }
    }

    const event = parseSseFrame(buffer);
    if (event !== undefined) {
      yield event;
    }
  } finally {
    reader.releaseLock();
  }
}

export function aggregateEvents(events) {
  let output = '';
  let doneEvent;
  let finishReason;
  let finalUsage;
  const contextPaths = [];
  const eventTypes = [];
  const steps = new Map();
  const toolCalls = new Map();

  for (const event of events) {
    const type = event?.type;
    if (typeof type === 'string') {
      eventTypes.push(type);
    }

    switch (type) {
      case 'chat.context_loaded':
        for (const contextPath of event.paths ?? []) {
          if (typeof contextPath === 'string' && !contextPaths.includes(contextPath)) {
            contextPaths.push(contextPath);
          }
        }
        break;
      case 'chat.delta':
        if (typeof event.delta === 'string') {
          output += event.delta;
        }
        break;
      case 'chat.step_start': {
        const index = Number(event.step_index);
        steps.set(index, {
          index,
          startedMs: Number(event.created_ms),
          finishedMs: undefined,
          model: event.model ?? undefined,
          finishReason: undefined,
          usage: normalizeUsage(),
        });
        break;
      }
      case 'chat.step_finish': {
        const index = Number(event.step_index);
        const step = steps.get(index) ?? { index };
        step.finishedMs = Number(event.finished_ms);
        step.model = event.model ?? step.model;
        step.finishReason = event.finish_reason ?? undefined;
        step.usage = normalizeUsage(event.usage);
        steps.set(index, step);
        finishReason = step.finishReason ?? finishReason;
        finalUsage = event.usage ?? finalUsage;
        break;
      }
      case 'chat.tool': {
        const id = String(event.call_id ?? '');
        const existing = toolCalls.get(id) ?? {
          id,
          name: String(event.name ?? ''),
          input: undefined,
          output: undefined,
          isError: false,
          status: undefined,
          stepIndex: Number(event.step_index),
          startedMs: undefined,
          finishedMs: undefined,
        };
        const payload = event.payload ?? {};
        if (payload.input !== undefined) {
          existing.input = parseMaybeJson(payload.input);
        }
        if (payload.output !== undefined) {
          existing.output = payload.output;
        }
        existing.status = event.status ?? payload.status ?? existing.status;
        existing.isError = ['failed', 'denied', 'cancelled'].includes(existing.status);
        if (event.kind === 'start' && existing.startedMs === undefined) {
          existing.startedMs = Number(event.occurred_ms);
        }
        if (event.kind === 'complete') {
          existing.startedMs ??= Number(event.occurred_ms);
          existing.finishedMs = Number(event.occurred_ms);
        }
        toolCalls.set(id, existing);
        break;
      }
      case 'chat.error':
        throw new Error(String(event.error || 'VibeWindow chat stream failed'));
      case 'chat.done':
        doneEvent = event;
        finalUsage = event.usage ?? finalUsage;
        break;
      default:
        break;
    }
  }

  if (!doneEvent) {
    throw new Error('VibeWindow chat stream closed before chat.done');
  }

  const usage = normalizeUsage(finalUsage);
  return {
    output,
    tokenUsage: {
      total: usage.inputTokens + usage.outputTokens,
      prompt: usage.inputTokens,
      completion: usage.outputTokens,
    },
    finishReason,
    sessionId: doneEvent.session_id,
    metadata: {
      messageId: doneEvent.message_id ?? undefined,
      parentMessageId: doneEvent.parent_message_id ?? undefined,
      contextPaths,
      eventTypes,
      steps: [...steps.values()].sort((left, right) => left.index - right.index),
      toolCalls: [...toolCalls.values()],
      usage,
    },
  };
}

export default class VibeWindowProvider {
  constructor(options = {}) {
    this.providerId = options.id || 'vibewindow-agent';
    this.config = options.config || {};
  }

  id() {
    return this.providerId;
  }

  async callApi(prompt, context = {}) {
    const endpoint = setting(
      this.config,
      { config: 'endpoint', environment: 'VIBEWINDOW_EVAL_ENDPOINT' },
      DEFAULT_ENDPOINT,
    );
    const directory = path.resolve(
      REPOSITORY_ROOT,
      setting(
        this.config,
        { config: 'directory', environment: 'VIBEWINDOW_EVAL_DIRECTORY' },
        REPOSITORY_ROOT,
      ),
    );
    const model =
      nonEmpty(context.vars?.model) ??
      setting(this.config, { config: 'model', environment: 'VIBEWINDOW_EVAL_MODEL' }, DEFAULT_MODEL);
    const agent =
      nonEmpty(context.vars?.agent) ??
      setting(this.config, { config: 'agent', environment: 'VIBEWINDOW_EVAL_AGENT' }, 'main');
    const platformId = setting(
      this.config,
      { config: 'platformId', environment: 'VIBEWINDOW_PLATFORM_ID' },
      '10000',
    );
    const companyId = setting(
      this.config,
      { config: 'companyId', environment: 'VIBEWINDOW_COMPANY_ID' },
      '100000000',
    );
    const timeoutMs = resolveTimeoutMs(this.config);
    const maxIterations = positiveInteger(this.config.maxIterations, DEFAULT_MAX_ITERATIONS);
    const maxToolCalls = positiveInteger(this.config.maxToolCalls, DEFAULT_MAX_TOOL_CALLS);
    const requestId = randomUUID();
    const tenantHeaders = {
      'X-Platform-ID': platformId,
      'X-Company-ID': companyId,
    };
    const request = {
      ...(model ? { model } : {}),
      ...(agent ? { agent } : {}),
      // DeepSeek V4 Flash 的输出上限高于 gateway 的通用 32000 上限；0 表示不附加
      // 评测层 max_output_tokens，避免长工具调用轨迹被截断为 max_tokens 错误。
      options: { max_output_tokens: 0 },
      messages: [{ role: 'user', content: prompt }],
    };
    const startedAt = performance.now();
    const controller = new AbortController();
    const timeout = setTimeout(() => controller.abort(), timeoutMs);

    try {
      const response = await fetch(endpoint, {
        method: 'POST',
        headers: {
          Accept: 'text/event-stream',
          'Content-Type': 'application/json',
          ...tenantHeaders,
          'X-VibeWindow-Directory': directory,
          'X-VW-Stream-Request-ID': requestId,
        },
        body: JSON.stringify(request),
        signal: controller.signal,
      });

      if (!response.ok) {
        const body = (await response.text()).slice(0, MAX_ERROR_BODY_CHARS);
        throw new Error(
          `VibeWindow gateway returned ${response.status} ${response.statusText}: ${body}`.trim(),
        );
      }

      const events = [];
      const steps = new Set();
      const completedToolCalls = new Set();
      let stopRequested = false;
      for await (const event of parseSseStream(response.body)) {
        reportProgress(event);
        events.push(event);
        if (event?.type === 'chat.step_start') {
          steps.add(Number(event.step_index));
        }
        if (event?.type === 'chat.tool' && event.kind === 'complete') {
          completedToolCalls.add(String(event.call_id ?? ''));
        }
        const reachedIterationLimit =
          event?.type === 'chat.step_finish' && steps.size >= maxIterations;
        const reachedToolCallLimit = completedToolCalls.size >= maxToolCalls;
        if (!stopRequested && (reachedIterationLimit || reachedToolCallLimit)) {
          stopRequested = true;
          const reason = reachedIterationLimit
            ? `${maxIterations} iterations`
            : `${maxToolCalls} tool calls`;
          process.stderr.write(`[vibewindow-eval] stopping stream after ${reason}\n`);
          await stopStream(endpoint, requestId, tenantHeaders);
        }
      }
      const result = aggregateEvents(events);
      return {
        ...result,
        prompt,
        latencyMs: Math.round(performance.now() - startedAt),
        metadata: {
          ...result.metadata,
          endpoint,
          directory,
          model,
          agent,
        },
      };
    } catch (error) {
      const message =
        error?.name === 'AbortError'
          ? `VibeWindow request timed out after ${timeoutMs}ms`
          : error instanceof Error
            ? error.message
            : String(error);
      return {
        error: message,
        latencyMs: Math.round(performance.now() - startedAt),
      };
    } finally {
      clearTimeout(timeout);
    }
  }
}
