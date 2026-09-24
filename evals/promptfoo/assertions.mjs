import path from 'node:path';

function metadata(context) {
  return context?.providerResponse?.metadata ?? context?.metadata ?? {};
}

function gradingResult(pass, reason, score = pass ? 1 : 0) {
  return { pass, score, reason };
}

function configuredList(context, name) {
  const raw = context?.config?.[name] ?? context?.vars?.[name] ?? [];
  let values = raw;
  if (typeof raw === 'string') {
    try {
      values = JSON.parse(raw);
    } catch {
      values = [raw];
    }
  }
  return Array.isArray(values)
    ? { values: values.map((value) => String(value)) }
    : { error: `${name} 必须是 JSON 数组或字符串` };
}

function toolCalls(context) {
  return metadata(context).toolCalls ?? [];
}

function isSuccessfulTool(tool) {
  return (
    !tool?.isError &&
    ['completed', 'success', 'succeeded'].includes(String(tool?.status ?? '').toLowerCase())
  );
}

function rustcodegraphOperationName(tool) {
  let input = tool?.input;
  if (typeof input === 'string') {
    try {
      input = JSON.parse(input);
    } catch {
      return undefined;
    }
  }
  return typeof input?.operation === 'string' ? input.operation.trim().toLowerCase() : undefined;
}

export function successfulRun(output, context) {
  const eventTypes = metadata(context).eventTypes ?? [];
  const pass =
    typeof output === 'string' &&
    output.trim().length > 0 &&
    eventTypes.includes('chat.done') &&
    !eventTypes.includes('chat.error');
  return gradingResult(pass, pass ? '收到非空回答和 chat.done' : '流未正常完成或回答为空');
}

export function noFailedTools(_output, context) {
  const calls = toolCalls(context);
  const hasSuccessfulTest = calls.some(
    (tool) => isTestCommand(commandText(tool)) && isSuccessfulTool(tool),
  );
  const failed = calls.filter(
    (tool) => tool.isError && !(hasSuccessfulTest && isTestCommand(commandText(tool))),
  );
  return gradingResult(
    failed.length === 0,
    failed.length === 0
      ? '没有失败、拒绝或取消的工具调用'
      : `失败工具：${failed.map((tool) => `${tool.name}(${tool.status})`).join(', ')}`,
  );
}

const THREAD_COMMANDS = Object.freeze([
  'create_thread',
  'list_threads',
  'read_thread',
  'wait_threads',
  'send_message_to_thread',
  'fork_thread',
  'set_thread_title',
  'set_thread_pinned',
  'set_thread_archived',
  'handoff_thread',
]);

function parsedJson(value) {
  if (value && typeof value === 'object') {
    return value;
  }
  if (typeof value !== 'string') {
    return undefined;
  }
  try {
    return JSON.parse(value);
  } catch {
    return undefined;
  }
}

function threadTargetMatches(tool, threadId) {
  const input = parsedJson(tool?.input);
  if (tool?.name === 'wait_threads') {
    return Array.isArray(input?.thread_ids) && input.thread_ids.includes(threadId);
  }
  return input?.thread_id === threadId;
}

export function threadTrajectory(_output, context) {
  const calls = toolCalls(context);
  const direct = new Map(THREAD_COMMANDS.map((name) => [name, calls.filter((call) => call?.name === name)]));
  const missing = THREAD_COMMANDS.filter((name) => direct.get(name).length === 0);
  if (missing.length > 0) {
    return gradingResult(false, `Thread 命令缺失：${missing.join(', ')}`);
  }

  const createCalls = direct.get('create_thread');
  const validCreate = createCalls.find((tool) => {
    const input = parsedJson(tool.input);
    const result = parsedJson(tool.output);
    return (
      isSuccessfulTool(tool) &&
      input?.prompt === '写一篇50字的科幻小说.' &&
      typeof result?.thread_id === 'string' &&
      result.thread_id.trim() &&
      typeof result?.project_id === 'string' &&
      result.project_id.trim()
    );
  });
  if (!validCreate || createCalls.some((tool) => !isSuccessfulTool(tool))) {
    return gradingResult(false, 'create_thread 必须成功，使用指定 prompt，并返回 thread_id 与 project_id');
  }

  const created = parsedJson(validCreate.output);
  const threadId = created.thread_id;
  const projectId = created.project_id;
  const listCalls = direct.get('list_threads');
  const validList = listCalls.some((tool) => {
    const result = parsedJson(tool.output);
    return (
      isSuccessfulTool(tool) &&
      Array.isArray(result?.threads) &&
      result.threads.some(
        (thread) => thread?.thread_id === threadId && thread?.project_id === projectId,
      )
    );
  });
  if (!validList || listCalls.some((tool) => !isSuccessfulTool(tool))) {
    return gradingResult(false, `list_threads 未返回创建的任务 ${threadId}`);
  }

  const targeted = THREAD_COMMANDS.filter(
    (name) => !['create_thread', 'list_threads', 'handoff_thread'].includes(name),
  );
  for (const name of targeted) {
    const commandCalls = direct.get(name);
    const valid = commandCalls.some(
      (tool) =>
        isSuccessfulTool(tool) &&
        threadTargetMatches(tool, threadId) &&
        parsedJson(tool.output) !== undefined,
    );
    if (!valid || commandCalls.some((tool) => !isSuccessfulTool(tool))) {
      return gradingResult(false, `${name} 必须成功调用任务 ${threadId} 并返回可解析结果`);
    }
  }

  const handoffCalls = direct.get('handoff_thread');
  const validHandoff = handoffCalls.some((tool) => {
    const detail = `${tool?.output ?? ''} ${tool?.error ?? ''}`;
    return (
      tool?.isError === true &&
      threadTargetMatches(tool, threadId) &&
      /(?:unsupported|not supported|不支持)/i.test(detail)
    );
  });
  if (!validHandoff || handoffCalls.some((tool) => isSuccessfulTool(tool))) {
    return gradingResult(false, `handoff_thread 必须对任务 ${threadId} 明确报告不支持`);
  }

  return gradingResult(
    true,
    `Thread 命令 10/10；目标 ${threadId}；项目 ${projectId}；handoff_thread 按预期拒绝`,
  );
}

export function maxToolCalls(_output, context) {
  const calls = toolCalls(context);
  const limit = Number(context?.vars?.maxToolCalls ?? context?.config?.maxToolCalls ?? 12);
  return gradingResult(
    calls.length <= limit,
    calls.length <= limit ? `工具调用 ${calls.length}/${limit}` : `工具调用过多：${calls.length}/${limit}`,
  );
}

function commandText(tool) {
  const input = tool?.input;
  if (typeof input === 'string') {
    return input;
  }
  if (input && typeof input === 'object') {
    return [input.command, input.cmd, ...(input.commands ?? [])]
      .filter((value) => typeof value === 'string')
      .join('\n');
  }
  return '';
}

const DIRECT_WRITE_TOOL = /^(?:write|write_file|file_write|edit|edit_file|file_edit|delete_file)$/i;
const SHELL_TOOL = /^(?:bash|shell|exec|exec_command|terminal|command|run_command)$/i;

function configuredPattern(context, name, required = false) {
  const raw = context?.vars?.[name] ?? context?.config?.[name];
  if (raw === undefined || raw === null || raw === '') {
    return required ? { error: `${name} 必须配置为正则表达式字符串` } : {};
  }
  if (typeof raw !== 'string') {
    return { error: `${name} 必须是正则表达式字符串` };
  }
  try {
    return { pattern: new RegExp(raw) };
  } catch (error) {
    return { error: `${name} 不是有效正则表达式：${error.message}` };
  }
}

function parsedInput(tool) {
  if (tool?.input && typeof tool.input === 'object') {
    return tool.input;
  }
  if (typeof tool?.input !== 'string') {
    return undefined;
  }
  try {
    return JSON.parse(tool.input);
  } catch {
    return undefined;
  }
}

function stripQuotes(value) {
  const trimmed = String(value ?? '').trim();
  if (
    trimmed.length >= 2 &&
    ((trimmed.startsWith('"') && trimmed.endsWith('"')) ||
      (trimmed.startsWith("'") && trimmed.endsWith("'")))
  ) {
    return trimmed.slice(1, -1);
  }
  return trimmed;
}

function normalizeObservedPath(value, context, workingDirectory = '.') {
  let observed = stripQuotes(value).replaceAll('\\', '/');
  if (!observed) {
    return undefined;
  }
  if (observed === '/dev/null') {
    return observed;
  }

  const repositoryDirectory = metadata(context).directory;
  if (path.posix.isAbsolute(observed)) {
    if (typeof repositoryDirectory !== 'string' || !repositoryDirectory.trim()) {
      return path.posix.normalize(observed);
    }
    const relative = path.relative(path.resolve(repositoryDirectory), path.resolve(observed));
    if (relative.startsWith('..') || path.isAbsolute(relative)) {
      return path.posix.normalize(observed);
    }
    observed = relative.replaceAll('\\', '/');
  } else if (workingDirectory && workingDirectory !== '.') {
    observed = path.posix.join(workingDirectory, observed);
  }

  return path.posix.normalize(observed).replace(/^\.\//, '');
}

function shellTokens(command) {
  return (command.match(/"(?:\\.|[^"\\])*"|'[^']*'|[^\s]+/g) ?? []).map((token) =>
    stripQuotes(token),
  );
}

function shellWritePaths(tool, context) {
  const input = parsedInput(tool);
  const command = commandText(tool);
  if (!command) {
    return { paths: [], unknown: false };
  }

  let workingDirectory = normalizeObservedPath(input?.cwd ?? input?.workdir ?? '.', context) ?? '.';
  const writePaths = [];
  let unknown = false;
  const resolveWritePath = (value) => {
    const normalized = normalizeObservedPath(value, context, workingDirectory);
    if (normalized) {
      writePaths.push(normalized);
    } else {
      unknown = true;
    }
  };

  for (const segment of command.split(/\s*(?:&&|\|\||;|\n)\s*/).filter(Boolean)) {
    const tokens = shellTokens(segment);
    if (tokens.length === 0) {
      continue;
    }
    if (tokens[0] === 'cd') {
      const directory = tokens.find((token, index) => index > 0 && !token.startsWith('-'));
      if (!directory) {
        unknown = true;
      } else {
        workingDirectory = normalizeObservedPath(directory, context, workingDirectory) ?? workingDirectory;
      }
      continue;
    }

    let commandIndex = tokens.findIndex((token) => !/^[A-Za-z_][A-Za-z0-9_]*=/.test(token));
    if (tokens[commandIndex] === 'env') {
      commandIndex = tokens.findIndex(
        (token, index) => index > commandIndex && !/^[A-Za-z_][A-Za-z0-9_]*=/.test(token),
      );
    }
    if (commandIndex < 0) {
      continue;
    }
    const executable = path.posix.basename(tokens[commandIndex]);
    const operands = tokens
      .slice(commandIndex + 1)
      .filter((token) => !token.startsWith('-') && !/^\d*(?:>>?|<>)$/.test(token));

    if (/^(?:mkdir|touch|rm|rmdir|truncate|unlink)$/.test(executable)) {
      if (operands.length === 0) {
        unknown = true;
      }
      operands.forEach(resolveWritePath);
    } else if (/^(?:cp|install)$/.test(executable)) {
      if (operands.length < 2) {
        unknown = true;
      } else {
        resolveWritePath(operands.at(-1));
      }
    } else if (executable === 'mv') {
      if (operands.length < 2) {
        unknown = true;
      } else {
        operands.forEach(resolveWritePath);
      }
    } else if (executable === 'tee') {
      if (operands.length === 0) {
        unknown = true;
      }
      operands.forEach(resolveWritePath);
    } else if (executable === 'sed' && tokens.includes('-i')) {
      if (operands.length === 0) {
        unknown = true;
      } else {
        resolveWritePath(operands.at(-1));
      }
    } else if (
      (/^(?:npm|pnpm|yarn|bun)$/.test(executable) &&
        /\b(?:install|add|init|create|ci)\b/.test(segment)) ||
      (executable === 'git' && /\binit\b/.test(segment))
    ) {
      resolveWritePath('.');
    }

    const redirectPattern = /(?:^|\s)\d*(?:>>?|<>)\s*("(?:\\.|[^"\\])*"|'[^']*'|[^\s;&|]+)/g;
    for (const match of segment.matchAll(redirectPattern)) {
      const target = stripQuotes(match[1]);
      if (!/^&\d+$/.test(target)) {
        resolveWritePath(target);
      }
    }
  }

  return { paths: writePaths, unknown };
}

function writeObservation(tool, context) {
  const name = String(tool?.name ?? '');
  if (name.toLowerCase() === 'apply_patch') {
    const input = parsedInput(tool);
    const patchText =
      input?.patchText ?? input?.patch ?? (typeof tool?.input === 'string' ? tool.input : '');
    if (typeof patchText !== 'string') {
      return { paths: [], unknown: true };
    }
    const paths = [...patchText.matchAll(/^\*\*\* (?:Add|Update|Delete) File: (.+)$/gm),
      ...patchText.matchAll(/^\*\*\* Move to: (.+)$/gm)]
      .map((match) => normalizeObservedPath(match[1], context))
      .filter(Boolean);
    return { paths, unknown: paths.length === 0 };
  }
  if (DIRECT_WRITE_TOOL.test(name)) {
    const input = parsedInput(tool);
    const observed = input?.path ?? input?.filePath ?? input?.file_path ?? input?.file;
    const normalized = normalizeObservedPath(observed, context);
    return { paths: normalized ? [normalized] : [], unknown: !normalized };
  }
  if (SHELL_TOOL.test(name)) {
    return shellWritePaths(tool, context);
  }
  return { paths: [], unknown: false };
}

function isTestCommand(command) {
  return (
    /(?:^|[;&|]\s*)node\b[^;&|\n]*\s--test(?:\s|$)/i.test(command) ||
    /(?:^|[;&|]\s*)(?:npm|pnpm|yarn|bun)\s+(?:run\s+)?test(?:\s|$)/i.test(command) ||
    /(?:^|[;&|]\s*)deno\s+test(?:\s|$)/i.test(command) ||
    /(?:^|[;&|]\s*)(?:(?:npx|bunx)\s+)?(?:vitest|jest|mocha)(?:\s|$)/i.test(command)
  );
}

function testRunsInDirectory(tool, root, context) {
  const input = parsedInput(tool);
  const workingDirectory = normalizeObservedPath(input?.cwd ?? input?.workdir, context);
  if (workingDirectory === root || workingDirectory?.startsWith(`${root}/`)) {
    return true;
  }
  const commandPaths = shellTokens(commandText(tool).replace(/[;&|]/g, ' '))
    .map((token) => token.replace(/^[^=]+=/, ''))
    .map((token) => normalizeObservedPath(token, context))
    .filter(Boolean);
  return commandPaths.some(
    (commandPath) =>
      commandPath === root || commandPath.startsWith(`${root}/`),
  );
}

export function codingTrajectory(_output, context) {
  const writeRoot = configuredPattern(context, 'writeRootPattern', true);
  const requiredArtifact = configuredPattern(context, 'requiredArtifactPattern');
  const excludedArtifact = configuredPattern(context, 'excludedArtifactPattern');
  const configurationError = writeRoot.error ?? requiredArtifact.error ?? excludedArtifact.error;
  if (configurationError) {
    return gradingResult(false, configurationError);
  }

  const writes = [];
  const unknownWriteTools = [];
  const unsuccessfulWriteTools = [];
  for (const tool of toolCalls(context)) {
    const observation = writeObservation(tool, context);
    writes.push(...observation.paths);
    if (observation.unknown) {
      unknownWriteTools.push(tool?.name ?? '未知工具');
    }
    if (observation.paths.length > 0 && !isSuccessfulTool(tool)) {
      unsuccessfulWriteTools.push(tool?.name ?? '未知工具');
    }
  }

  if (unknownWriteTools.length > 0) {
    return gradingResult(false, `无法识别工具写入路径：${unknownWriteTools.join(', ')}`);
  }
  if (unsuccessfulWriteTools.length > 0) {
    return gradingResult(false, `写入工具未成功：${unsuccessfulWriteTools.join(', ')}`);
  }
  if (writes.length === 0) {
    return gradingResult(false, '未检测到文件写入');
  }

  const writeRoots = new Set();
  const outsideWrites = [];
  for (const writtenPath of writes) {
    if (writtenPath === '/dev/null') {
      continue;
    }
    const match = writtenPath.match(writeRoot.pattern);
    if (!match?.[1]) {
      outsideWrites.push(writtenPath);
    } else {
      writeRoots.add(match[1]);
    }
  }
  if (outsideWrites.length > 0) {
    return gradingResult(false, `检测到允许目录外写入：${[...new Set(outsideWrites)].join(', ')}`);
  }
  if (writeRoots.size !== 1) {
    return gradingResult(
      false,
      writeRoots.size === 0
        ? '未检测到符合 writeRootPattern 的写入目录'
        : `写入跨越多个允许目录：${[...writeRoots].join(', ')}`,
    );
  }

  const root = [...writeRoots][0];
  if (requiredArtifact.pattern) {
    const artifacts = writes.filter(
      (writtenPath) =>
        (writtenPath === root || writtenPath.startsWith(`${root}/`)) &&
        requiredArtifact.pattern.test(writtenPath) &&
        !excludedArtifact.pattern?.test(writtenPath),
    );
    if (artifacts.length === 0) {
      return gradingResult(false, `未在 ${root} 创建符合 requiredArtifactPattern 的产物`);
    }
  }

  if (context?.vars?.requireSuccessfulTest ?? context?.config?.requireSuccessfulTest ?? false) {
    const testCalls = toolCalls(context).filter(
      (tool) => SHELL_TOOL.test(tool?.name ?? '') && isTestCommand(commandText(tool)),
    );
    const scopedTestCalls = testCalls.filter((tool) => testRunsInDirectory(tool, root, context));
    if (!scopedTestCalls.some(isSuccessfulTool)) {
      return gradingResult(false, `未运行 ${root} 内的测试或缺少成功状态`);
    }
  }

  return gradingResult(true, `${root} 的写入范围和配置要求均通过`);
}

function calledSkillName(tool) {
  if (String(tool?.name ?? '').toLowerCase() !== 'skill') {
    return undefined;
  }
  const input = parsedInput(tool);
  return [input?.skill, input?.name, input?.skill_name]
    .find((value) => typeof value === 'string')
    ?.trim()
    .toLowerCase();
}

export function requiredSkillsSucceeded(_output, context) {
  const configured = configuredList(context, 'requiredSuccessfulSkills');
  if (configured.error) {
    return gradingResult(false, configured.error);
  }
  const required = configured.values.map((name) => name.trim().toLowerCase()).filter(Boolean);
  if (required.length === 0) {
    return gradingResult(true, '未配置 requiredSuccessfulSkills');
  }

  const successful = new Set(
    toolCalls(context).filter(isSuccessfulTool).map(calledSkillName).filter(Boolean),
  );
  const missing = required.filter((name) => !successful.has(name));
  return gradingResult(
    missing.length === 0,
    missing.length === 0
      ? `必需技能调用成功：${required.join(', ')}`
      : `未成功调用必需技能：${missing.join(', ')}`,
  );
}

export function readOnlyTrajectory(_output, context) {
  const calls = toolCalls(context);
  const writeToolNames = /^(apply_patch|write|write_file|edit|edit_file|delete_file)$/i;
  const mutatingCommand =
    /(^|[;&|]\s*)(rm|mv|cp|touch|mkdir|rmdir|truncate)\b|\bgit\s+(add|commit|checkout|clean|reset|restore|switch)\b|\bsed\s+-i\b|(^|\s)(>>?|2>)(?!=)/i;
  const violations = calls.filter(
    (tool) => writeToolNames.test(tool.name ?? '') || mutatingCommand.test(commandText(tool)),
  );
  return gradingResult(
    violations.length === 0,
    violations.length === 0
      ? '轨迹中没有检测到写操作'
      : `检测到写操作：${violations.map((tool) => tool.name).join(', ')}`,
  );
}

export function containsExpectedTerms(output, context) {
  const configured = configuredList(context, 'expectedTerms');
  if (configured.error) {
    return gradingResult(false, configured.error);
  }
  const expectedTerms = configured.values;
  const missing = expectedTerms.filter((term) => !String(output).includes(String(term)));
  return gradingResult(
    missing.length === 0,
    missing.length === 0 ? '包含全部预期关键信息' : `缺少关键信息：${missing.join(', ')}`,
  );
}

export function directRustcodegraphCall(_output, context) {
  const direct = toolCalls(context).filter((tool) => tool?.name === 'rustcodegraph');
  return gradingResult(
    direct.length > 0,
    direct.length > 0
      ? `检测到 ${direct.length} 次内置 rustcodegraph 调用`
      : '未检测到内置 rustcodegraph 调用',
  );
}

export function successfulRustcodegraphCall(_output, context) {
  const direct = toolCalls(context).filter((tool) => tool?.name === 'rustcodegraph');
  const successful = direct.filter(isSuccessfulTool);
  if (successful.length > 0) {
    const operations = successful
      .map(rustcodegraphOperationName)
      .filter(Boolean)
      .join(', ');
    return gradingResult(
      true,
      operations
        ? `内置 rustcodegraph 调用成功：${operations}`
        : '内置 rustcodegraph 调用成功',
    );
  }
  if (direct.length === 0) {
    return gradingResult(false, '没有可验证成功状态的内置 rustcodegraph 调用');
  }
  const failures = direct
    .map(
      (tool) =>
        `${rustcodegraphOperationName(tool) ?? '未知操作'}(${tool.status ?? '缺少完成状态'})`,
    )
    .join(', ');
  return gradingResult(false, `内置 rustcodegraph 调用未成功：${failures}`);
}

export function rustcodegraphOperation(_output, context) {
  const configured = configuredList(context, 'expectedOperations');
  if (configured.error) {
    return gradingResult(false, configured.error);
  }
  const expected = configured.values.map((operation) => operation.trim().toLowerCase()).filter(Boolean);
  if (expected.length === 0) {
    return gradingResult(true, '未配置 expectedOperations，不限制操作类型');
  }

  const direct = toolCalls(context).filter((tool) => tool?.name === 'rustcodegraph');
  const successful = direct.filter(isSuccessfulTool);
  if (successful.length === 0) {
    if (direct.length === 0) {
      return gradingResult(false, `未通过内置 rustcodegraph 执行期望操作：${expected.join(', ')}`);
    }
    const failures = direct
      .map(
        (tool) =>
          `${rustcodegraphOperationName(tool) ?? '未知操作'}(${tool.status ?? '缺少完成状态'})`,
      )
      .join(', ');
    return gradingResult(false, `RustCodeGraph 操作调用未成功：${failures}`);
  }

  const actual = [...new Set(successful.map(rustcodegraphOperationName).filter(Boolean))];
  const missing = expected.filter((operation) => !actual.includes(operation));
  return gradingResult(
    missing.length === 0,
    missing.length === 0
      ? `RustCodeGraph 操作匹配：${expected.join(', ')}`
      : `RustCodeGraph 操作不匹配；期望：${expected.join(', ')}；实际：${actual.join(', ') || '无'}`,
  );
}

export function noNavigationFallback(_output, context) {
  const shellTool = /^(bash|shell|exec|exec_command|terminal|command|run_command)$/i;
  const cliFallbacks = [];
  const textFallbacks = [];
  for (const tool of toolCalls(context)) {
    if (!shellTool.test(tool?.name ?? '')) {
      continue;
    }
    const command = commandText(tool);
    if (/(^|[^\w-])rustcodegraph(?=\s|$)/i.test(command)) {
      cliFallbacks.push(tool.name);
    }
    if (
      /(^|[\s;&|])(rg|grep|ag|ack|ripgrep|findstr)(?=\s|$)/i.test(command) ||
      /(^|[\s;&|])git\s+grep(?=\s|$)/i.test(command)
    ) {
      textFallbacks.push(tool.name);
    }
  }

  const reasons = [];
  if (cliFallbacks.length > 0) {
    reasons.push(`RustCodeGraph CLI fallback：${cliFallbacks.join(', ')}`);
  }
  if (textFallbacks.length > 0) {
    reasons.push(`文本搜索 fallback：${textFallbacks.join(', ')}`);
  }
  return gradingResult(
    reasons.length === 0,
    reasons.length === 0 ? '未使用代码导航 fallback' : reasons.join('；'),
  );
}

export function expectedFactsScore(output, context) {
  const configured = configuredList(context, 'expectedTerms');
  if (configured.error) {
    return gradingResult(false, configured.error);
  }
  const expected = configured.values;
  if (expected.length === 0) {
    return gradingResult(true, '未配置 expectedTerms，事实完整度记满分');
  }

  const answer = String(output);
  const missing = expected.filter((term) => !answer.includes(term));
  const matched = expected.length - missing.length;
  const score = matched / expected.length;
  return gradingResult(
    missing.length === 0,
    missing.length === 0
      ? `事实命中 ${matched}/${expected.length}`
      : `事实命中 ${matched}/${expected.length}；缺少：${missing.join(', ')}`,
    score,
  );
}

const EVALUATION_PROFILES = Object.freeze({
  rustcodegraph: {
    threshold: 0.7,
    checks: [
      ['successful_run', 10, successfulRun, true],
      ['read_only', 10, readOnlyTrajectory, true],
      ['no_failed_tools', 10, noFailedTools, true],
      ['direct_rustcodegraph_call', 20, directRustcodegraphCall, true],
      ['rustcodegraph_call_success', 15, successfulRustcodegraphCall, true],
      ['rustcodegraph_operation', 10, rustcodegraphOperation, false],
      ['no_navigation_fallback', 10, noNavigationFallback, false],
      ['expected_facts', 10, expectedFactsScore, false],
      ['tool_call_budget', 5, maxToolCalls, false],
    ],
  },
  coding: {
    threshold: 1,
    checks: [
      ['successful_run', 10, successfulRun, true],
      ['no_failed_tools', 20, noFailedTools, true],
      ['coding_trajectory', 60, codingTrajectory, true],
      ['tool_call_budget', 10, maxToolCalls, false],
    ],
  },
  'skill-selection': {
    threshold: 1,
    checks: [
      ['successful_run', 25, successfulRun, true],
      ['no_failed_tools', 25, noFailedTools, true],
      ['required_skill_success', 40, requiredSkillsSucceeded, true],
      ['tool_call_budget', 10, maxToolCalls, false],
    ],
  },
  'thread-commands': {
    threshold: 1,
    checks: [
      ['successful_run', 10, successfulRun, true],
      ['thread_trajectory', 90, threadTrajectory, true],
    ],
  },
});

function evaluationProfile(context) {
  const explicit = context?.vars?.evaluationProfile;
  if (typeof explicit === 'string' && explicit.trim()) {
    return explicit.trim();
  }
  const suite = String(context?.test?.metadata?.suite ?? context?.vars?.suite ?? '');
  if (suite.startsWith('rustcodegraph-') || context?.vars?.expectedOperations) {
    return 'rustcodegraph';
  }
  return suite;
}

export function evaluate(output, context) {
  const profileName = evaluationProfile(context);
  const profile = EVALUATION_PROFILES[profileName];
  if (!profile) {
    return gradingResult(
      false,
      `未知评测类型：${profileName || '未配置'}；请设置 metadata.suite 或 vars.evaluationProfile`,
    );
  }

  const componentResults = profile.checks.map(([name, weight, check, hardGate]) => {
    const result = check(output, context);
    return {
      ...result,
      namedScores: { [name]: result.score },
      namedScoreWeights: { [name]: weight },
      metadata: { hardGate },
    };
  });
  const totalWeight = profile.checks.reduce((total, [, weight]) => total + weight, 0);
  const points = profile.checks.reduce(
    (total, [, weight], index) => total + componentResults[index].score * weight,
    0,
  );
  const score = points / totalWeight;
  const failedGates = profile.checks
    .filter(([, , , hardGate], index) => hardGate && !componentResults[index].pass)
    .map(([name]) => name);
  const pass = failedGates.length === 0 && score >= profile.threshold;
  const reason = failedGates.length > 0
    ? `总分 ${points}/${totalWeight}；未通过硬门槛：${failedGates.join(', ')}`
    : `总分 ${points}/${totalWeight}；${pass ? '通过' : `低于阈值 ${profile.threshold}`}`;

  return {
    pass,
    score,
    reason,
    componentResults,
    namedScores: Object.assign({}, ...componentResults.map((result) => result.namedScores)),
    namedScoreWeights: Object.assign(
      {},
      ...componentResults.map((result) => result.namedScoreWeights),
    ),
  };
}
