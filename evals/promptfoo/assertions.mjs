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
  const failed = toolCalls(context).filter((tool) => tool.isError);
  return gradingResult(
    failed.length === 0,
    failed.length === 0
      ? '没有失败、拒绝或取消的工具调用'
      : `失败工具：${failed.map((tool) => `${tool.name}(${tool.status})`).join(', ')}`,
  );
}

export function maxToolCalls(_output, context) {
  const calls = toolCalls(context);
  const limit = Number(context?.config?.maxToolCalls ?? 12);
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
