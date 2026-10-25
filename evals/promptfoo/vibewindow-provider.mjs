import path from 'node:path';
import { mkdir, writeFile } from 'node:fs/promises';
import { createArtifactOracle } from './artifact-oracle.mjs';
import { prepare, cleanup } from './attempt-workspace.mjs';
import { validateManifest, manifestDigest } from './accuracy-manifest.mjs';
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

function followupPrompts(value) {
  if (value === undefined) {
    return [];
  }
  if (!Array.isArray(value)) {
    throw new Error('VibeWindow evaluation turns must be an array of non-empty strings');
  }
  const prompts = value.map(nonEmpty);
  if (prompts.some((prompt) => prompt === undefined)) {
    throw new Error('VibeWindow evaluation turns must be an array of non-empty strings');
  }
  return prompts;
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
          error: undefined,
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
        if (payload.error !== undefined) {
          existing.error = payload.error;
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
    this.attempts = new Map();
  }

  id() {
    return this.providerId;
  }

  // The verifier calls this only after all directory users and descendants exit.
  async cleanupAttempt(attemptId) {
    const workspace = this.attempts.get(attemptId);
    if (!workspace) throw new Error('unknown provider attempt');
    await cleanup(workspace);
    this.attempts.delete(attemptId);
  }

  // runtime-accuracy requires evaluator-owned accuracyAcceptance configuration:
  // definitions is the taskId -> trusted byte definition Map consumed by the
  // artifact oracle. admit({attemptId, workRoot}) returns the environment proof
  // summary below; settle({attemptId, workRoot, sessionId, admission}) attests
  // that all remote writers/descendants exited and reports non_target_files
  // checks linked to that proof's digest. Neither callback reads model claims.
  // Missing/failed settlement retains the workspace for supervised recovery.
  async callApi(prompt, context = {}) {
    const result = await this.runApi(prompt, context);
    const data = result.metadata;
    if (data?.evaluationProfile !== 'runtime-accuracy') return result;
    const acceptance = this.config.accuracyAcceptance;
    data.oracle = { pass: false, checks: [], reason: 'artifact acceptance unavailable' };
    data.boundary = { pass: false };
    data.cleanup = 'cleanup_pending';
    const turns = data.turns ?? [result];
    data.terminal = { pass: !result.error && turns.every(turn =>
      turn.metadata.eventTypes?.includes('chat.done') &&
      !turn.metadata.eventTypes?.includes('chat.error') &&
      [undefined, 'stop', 'end_turn', 'completed'].includes(turn.finishReason)) };
    const budget = data.manifest.contract.experiment.budget;
    data.budget = { pass: !result.error && !data.budgetStopped && result.latencyMs <= budget.wallTimeMs &&
      turns.reduce((n, turn) => n + (turn.metadata.steps?.length ?? 0), 0) <= budget.iterations &&
      turns.reduce((n, turn) => n + (turn.metadata.toolCalls?.length ?? 0), 0) <= budget.toolCalls };
    let safeToClean = data.remoteStarted !== true;
    try {
      // SSE completion is not proof that remote children have exited. Only the
      // trusted environment supervisor can attest their complete lifecycle.
      if (!safeToClean && typeof acceptance?.settle === 'function') {
        const settled = await acceptance.settle({ attemptId: data.attemptId,
          workRoot: data.workRoot, sessionId: result.sessionId,
          admission: structuredClone(data.admission) });
        safeToClean = settled?.writersExited === true;
        data.boundary = { pass: safeToClean && settled.admissionDigest === data.admission.digest &&
          Array.isArray(settled.checks) && settled.checks.length > 0 &&
          settled.checks.some(check => check.kind === 'non_target_files') &&
          settled.checks.every(check => check.pass === true),
          admissionDigest: data.admission.digest };
      }
      if (safeToClean && data.admission) {
        const verify = createArtifactOracle(acceptance?.definitions);
        data.oracle = await verify({ taskId: data.taskId, workRoot: data.workRoot, response: result.output });
      }
    } catch {
      // Preserve the transport error and never include supervisor payloads or
      // expected file contents in diagnostics.
      data.acceptanceError = 'trusted acceptance unavailable';
    }
    try {
      const evidenceDirectory = path.join(REPOSITORY_ROOT, 'evals/promptfoo/results/accuracy-evidence');
      await mkdir(evidenceDirectory, { recursive: true });
      const evidencePath = path.join(evidenceDirectory, `${data.attemptId}.json`);
      await writeFile(evidencePath, JSON.stringify({ attemptId: data.attemptId,
        taskId: data.taskId, manifestDigest: data.manifestDigest, admission: data.admission,
        oracle: data.oracle, boundary: data.boundary, terminal: data.terminal, budget: data.budget,
        failureKind: data.failureKind, acceptanceError: data.acceptanceError,
        cleanup: safeToClean ? 'eligible_after_archive' : 'cleanup_pending' }), { flag: 'wx' });
      data.evidencePath = evidencePath;
      if (safeToClean) {
        await this.cleanupAttempt(data.attemptId);
        data.cleanup = 'cleaned';
      }
    } catch {
      data.cleanup = 'cleanup_pending';
    }
    return result;
  }

  async runApi(prompt, context = {}) {
    const endpoint = setting(
      this.config,
      { config: 'endpoint', environment: 'VIBEWINDOW_EVAL_ENDPOINT' },
      DEFAULT_ENDPOINT,
    );
    let directory = path.resolve(
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
    const maxIterations = positiveInteger(
      context.vars?.maxIterations ?? this.config.maxIterations,
      DEFAULT_MAX_ITERATIONS,
    );
    const maxToolCalls = positiveInteger(
      context.vars?.maxToolCalls ?? this.config.maxToolCalls,
      DEFAULT_MAX_TOOL_CALLS,
    );
    const tenantHeaders = {
      'X-Platform-ID': platformId,
      'X-Company-ID': companyId,
    };
    const startedAt = performance.now();

    let attemptMetadata;
    const turns = [];
    const receivedEvents = [];
    let failureKind = 'configuration';
    let sessionId;
    try {
      if (this.config.evaluationProfile === 'runtime-accuracy') {
        // Only trusted provider configuration supplies identity and public inputs.
        const manifest = validateManifest(structuredClone(this.config.manifest));
        const task = manifest.contract.tasks.find(task => task.taskId === this.config.taskId);
        if (!task) throw new Error('runtime-accuracy task is missing from manifest');
        const workspace = await prepare({ fixtureRoot: this.config.fixtureRoot,
          fixtureFiles: this.config.fixtureFiles });
        this.attempts.set(workspace.attemptId, workspace);
        directory = workspace.workRoot;
        attemptMetadata = { evaluationProfile: 'runtime-accuracy', manifest,
          manifestDigest: manifestDigest(manifest), taskId: task.taskId, split: task.split,
          expectedAttempts: manifest.contract.experiment.execution.repeats,
          fixtureDigest: workspace.inputDigest, attemptId: workspace.attemptId,
          workRoot: workspace.workRoot, cleanupPolicy: workspace.cleanupPolicy };
        if (task.fixtureDigest !== workspace.inputDigest) {
          throw new Error('runtime-accuracy fixtureDigest mismatch');
        }
      }
      if (attemptMetadata) {
        const admit = this.config.accuracyAcceptance?.admit;
        if (typeof admit !== 'function') throw new Error('runtime-accuracy admission evidence required');
        const proof = structuredClone(await admit({ attemptId: attemptMetadata.attemptId, workRoot: directory }));
        if (!proof || proof.workRoot !== directory ||
            !['restricted-sandbox', 'pollution-only-directory'].includes(proof.mode) ||
            !/^[a-f0-9]{64}$/.test(proof.digest ?? '') ||
            !/^[a-f0-9]{64}$/.test(proof.configurationDigest ?? '') ||
            !Array.isArray(proof.forbiddenWriteScopes) || !proof.forbiddenWriteScopes.length ||
            !proof.forbiddenWriteScopes.every(scope => typeof scope === 'string' && path.isAbsolute(scope)) ||
            typeof proof.evidencePath !== 'string' || !path.isAbsolute(proof.evidencePath) ||
            typeof proof.verifiedAt !== 'string' || !Number.isFinite(Date.parse(proof.verifiedAt))) {
          throw new Error('runtime-accuracy admission evidence invalid');
        }
        // Keep only the declared proof summary; supervisor payloads are not logs.
        attemptMetadata.admission = Object.fromEntries([
          'workRoot', 'mode', 'configurationDigest', 'forbiddenWriteScopes',
          'evidencePath', 'verifiedAt', 'digest',
        ].map(key => [key, proof[key]]));
      }
      const prompts = [prompt, ...followupPrompts(context.vars?.turns)];
      const messages = [];

      for (const turnPrompt of prompts) {
        messages.push({ role: 'user', content: turnPrompt });
        const requestId = randomUUID();
        const request = {
          ...(sessionId ? { session_id: sessionId } : {}),
          ...(model ? { model } : {}),
          ...(agent ? { agent } : {}),
          // DeepSeek V4 Flash 的输出上限高于 gateway 的通用 32000 上限；0 表示不附加
          // 评测层 max_output_tokens，避免长工具调用轨迹被截断为 max_tokens 错误。
          options: { max_output_tokens: 0 },
          messages,
        };
        const turnStartedAt = performance.now();
        const controller = new AbortController();
        const timeout = setTimeout(() => controller.abort(), timeoutMs);

        try {
          failureKind = 'transport';
          if (attemptMetadata) attemptMetadata.remoteStarted = true;
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
            failureKind = 'http';
            const body = (await response.text()).slice(0, MAX_ERROR_BODY_CHARS);
            throw new Error(
              `VibeWindow gateway returned ${response.status} ${response.statusText}: ${body}`.trim(),
            );
          }

          failureKind = 'stream';
          const events = [];
          const steps = new Set();
          const completedToolCalls = new Set();
          let stopRequested = false;
          for await (const event of parseSseStream(response.body)) {
            reportProgress(event);
            events.push(event);
            receivedEvents.push(event);
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
              if (attemptMetadata) attemptMetadata.budgetStopped = true;
              const reason = reachedIterationLimit
                ? `${maxIterations} iterations`
                : `${maxToolCalls} tool calls`;
              process.stderr.write(`[vibewindow-eval] stopping stream after ${reason}\n`);
              await stopStream(endpoint, requestId, tenantHeaders);
            }
          }

          const result = aggregateEvents(events);
          sessionId = result.sessionId;
          const turn = {
            ...result,
            prompt: turnPrompt,
            latencyMs: Math.round(performance.now() - turnStartedAt),
            metadata: {
              ...result.metadata,
              ...attemptMetadata,
              endpoint,
              directory,
              model,
              agent,
            },
          };
          turns.push(turn);
          messages.push({ role: 'assistant', content: result.output });
        } finally {
          clearTimeout(timeout);
        }
      }

      if (turns.length === 1) {
        return turns[0];
      }

      const finalTurn = turns.at(-1);
      const usage = turns.reduce(
        (total, turn) => ({
          inputTokens: total.inputTokens + turn.metadata.usage.inputTokens,
          outputTokens: total.outputTokens + turn.metadata.usage.outputTokens,
          cachedTokens: total.cachedTokens + turn.metadata.usage.cachedTokens,
          reasoningTokens: total.reasoningTokens + turn.metadata.usage.reasoningTokens,
        }),
        normalizeUsage(),
      );
      return {
        output: finalTurn.output,
        tokenUsage: {
          total: usage.inputTokens + usage.outputTokens,
          prompt: usage.inputTokens,
          completion: usage.outputTokens,
        },
        finishReason: finalTurn.finishReason,
        sessionId,
        prompt,
        latencyMs: Math.round(performance.now() - startedAt),
        metadata: {
          ...attemptMetadata,
          messageId: finalTurn.metadata.messageId,
          parentMessageId: finalTurn.metadata.parentMessageId,
          contextPaths: [...new Set(turns.flatMap((turn) => turn.metadata.contextPaths))],
          eventTypes: turns.flatMap((turn) => turn.metadata.eventTypes),
          steps: turns.flatMap((turn, turnIndex) =>
            turn.metadata.steps.map((step) => ({ ...step, turnIndex })),
          ),
          toolCalls: turns.flatMap((turn, turnIndex) =>
            turn.metadata.toolCalls.map((toolCall) => ({ ...toolCall, turnIndex })),
          ),
          usage,
          endpoint,
          directory,
          model,
          agent,
          turns,
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
        ...(attemptMetadata ? { sessionId, metadata: { ...attemptMetadata,
          endpoint, directory, model, agent, turns, events: receivedEvents,
          failureKind: error?.name === 'AbortError' ? 'timeout' : failureKind } } : {}),
        latencyMs: Math.round(performance.now() - startedAt),
      };
    }
  }
}
