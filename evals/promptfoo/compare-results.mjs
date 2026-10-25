#!/usr/bin/env node

import { assertComparable } from './accuracy-manifest.mjs';
import { readFile } from 'node:fs/promises';
import path from 'node:path';
import process from 'node:process';
import { pathToFileURL } from 'node:url';

function nonEmptyString(value) {
  return typeof value === 'string' && value.trim() ? value.trim() : undefined;
}

function sorted(values) {
  return [...values].sort((left, right) => (left < right ? -1 : left > right ? 1 : 0));
}

function round(value) {
  return Number(value.toFixed(12));
}

function extractRows(document, sourceName) {
  const rows = Array.isArray(document?.results?.results)
    ? document.results.results
    : Array.isArray(document?.results)
      ? document.results
      : undefined;
  if (!rows || rows.length === 0) {
    throw new Error(`${sourceName} 未包含 Promptfoo 结果记录`);
  }
  return rows;
}

function normalizeRow(row, sourceName, index) {
  const provider = nonEmptyString(row?.provider?.label) ?? nonEmptyString(row?.provider?.id);
  const task =
    nonEmptyString(row?.testCase?.description) ?? nonEmptyString(row?.description);
  if (!provider) {
    throw new Error(`${sourceName} 第 ${index + 1} 条记录缺少 provider label/id`);
  }
  if (!task) {
    throw new Error(`${sourceName} 第 ${index + 1} 条记录缺少测试描述`);
  }

  const score = Number(row?.score);
  if (!Number.isFinite(score)) {
    throw new Error(`${sourceName} 记录 ${provider} :: ${task} 的 score 不是有限数字`);
  }
  if (!row?.namedScores || typeof row.namedScores !== 'object' || Array.isArray(row.namedScores)) {
    throw new Error(`${sourceName} 记录 ${provider} :: ${task} 缺少 namedScores`);
  }

  const namedScores = {};
  for (const metric of sorted(Object.keys(row.namedScores))) {
    const value = row.namedScores[metric];
    const metricScore = typeof value === 'number' || (typeof value === 'string' && value.trim())
      ? Number(value)
      : NaN;
    if (!Number.isFinite(metricScore)) {
      throw new Error(
        `${sourceName} 记录 ${provider} :: ${task} 的 metric ${metric} 不是有限数字`,
      );
    }
    namedScores[metric] = metricScore;
  }

  const pass =
    typeof row?.gradingResult?.pass === 'boolean'
      ? row.gradingResult.pass
      : typeof row?.success === 'boolean'
        ? row.success
        : false;
  const reason =
    nonEmptyString(row?.gradingResult?.reason) ??
    nonEmptyString(row?.error) ??
    '未提供失败原因';

  return {
    raw: row,
    rowIndex: index + 1,
    key: `${provider}\u0000${task}`,
    displayKey: `${provider} :: ${task}`,
    provider,
    task,
    score,
    namedScores,
    pass,
    reason,
  };
}

function accuracyMetadata(row, sourceName) {
  const metadata = row.raw.metadata ?? row.raw.response?.metadata;
  const profile = row.raw.testCase?.metadata?.evaluationProfile;
  if (profile !== 'runtime-accuracy' && row.raw.testCase?.metadata?.suite !== 'runtime-accuracy'
      && metadata?.evaluationProfile !== 'runtime-accuracy') return undefined;
  const fail = (field) => { throw new Error(`${sourceName} 契约 ${field} 无效：${row.displayKey}`); };
  let digest;
  try { digest = assertComparable(metadata?.manifest); } catch { fail('manifest'); }
  if (metadata.manifestDigest !== digest) fail('manifestDigest');
  if (metadata.expectedAttempts !== metadata.manifest.contract.experiment.execution.repeats) fail('expectedAttempts');
  if (!metadata.manifest.contract.tasks.some(task => task.description === row.task)) fail('tasks');
  const { sourceCommit, sourceDiffDigest } = metadata.manifest.run;
  if (!sourceCommit || !sourceDiffDigest) fail('run.sourceIdentity');
  return { digest, expectedAttempts: metadata.expectedAttempts, sourceIdentity: { sourceCommit, sourceDiffDigest } };
}

function assertAccuracyContract(before, after) {
  if (!before.accuracy && !after.accuracy) return;
  if (!before.accuracy || !after.accuracy || before.accuracy.digest !== after.accuracy.digest) {
    throw new Error(`契约 manifestDigest 不一致：${before.displayKey}`);
  }
}

function indexRows(document, sourceName) {
  const rows = extractRows(document, sourceName).map((row, index) =>
    normalizeRow(row, sourceName, index),
  );
  const indexed = new Map();
  for (const row of rows) row.accuracy = accuracyMetadata(row, sourceName);
  const accuracyRow = rows.find(row => row.accuracy);
  if (accuracyRow) for (const row of rows) assertAccuracyContract(accuracyRow, row);
  for (const row of rows) {
    const group = indexed.get(row.key);
    if (group) {
      assertSameMetrics(group, row);
      assertAccuracyContract(group, row);
      group.attempts.push(row);
    } else {
      indexed.set(row.key, { ...row, attempts: [row] });
    }
  }
  // The export provides no guaranteed attempt identity across imports. Count every
  // record, including identical records; this does not detect duplicate imports.
  for (const group of indexed.values()) {
    group.attemptCount = group.attempts.length;
    if (group.accuracy && group.attemptCount !== group.accuracy.expectedAttempts) {
      throw new Error(`${sourceName} 缺样或样本数超出 expectedAttempts：${group.displayKey}`);
    }
    if (group.accuracy) {
      group.sourceIdentities = [...new Map(group.attempts.map(row =>
        [JSON.stringify(row.accuracy.sourceIdentity), row.accuracy.sourceIdentity])).values()];
    }
    group.passCount = group.attempts.filter((row) => row.pass).length;
    group.passRate = group.passCount / group.attemptCount;
    group.pass = group.passCount === group.attemptCount;
    group.score = average(group.attempts.reduce((sum, row) => sum + row.score, 0), group.attemptCount);
    group.namedScores = Object.fromEntries(Object.keys(group.namedScores).map((metric) => [
      metric,
      average(group.attempts.reduce((sum, row) => sum + row.namedScores[metric], 0), group.attemptCount),
    ]));
  }
  return indexed;
}

function assertSameKeys(before, after) {
  for (const [key, row] of before) {
    if (!after.has(key)) {
      throw new Error(`after 结果缺少记录：${row.displayKey}`);
    }
  }
  for (const [key, row] of after) {
    if (!before.has(key)) {
      throw new Error(`before 结果缺少记录：${row.displayKey}`);
    }
  }
}

function assertSameMetrics(before, after) {
  const beforeMetrics = sorted(Object.keys(before.namedScores));
  const afterMetrics = sorted(Object.keys(after.namedScores));
  if (
    beforeMetrics.length !== afterMetrics.length ||
    beforeMetrics.some((metric, index) => metric !== afterMetrics[index])
  ) {
    const missingAfter = beforeMetrics.filter((metric) => !afterMetrics.includes(metric));
    const missingBefore = afterMetrics.filter((metric) => !beforeMetrics.includes(metric));
    const details = [
      missingAfter.length ? `after 缺少 ${missingAfter.join(', ')}` : '',
      missingBefore.length ? `before 缺少 ${missingBefore.join(', ')}` : '',
    ]
      .filter(Boolean)
      .join('；');
    throw new Error(`metric 集不一致：${before.displayKey}（${details}）`);
  }
  return beforeMetrics;
}

function average(total, count) {
  return round(total / count);
}

export function compareResults(beforeDocument, afterDocument) {
  const before = indexRows(beforeDocument, 'before');
  const after = indexRows(afterDocument, 'after');
  assertSameKeys(before, after);

  const tasks = [];
  const providerTotals = new Map();
  const metricTotals = new Map();
  const failures = [];

  for (const key of sorted(before.keys())) {
    const beforeRow = before.get(key);
    const afterRow = after.get(key);
    assertAccuracyContract(beforeRow, afterRow);
    const metrics = assertSameMetrics(beforeRow, afterRow);
    tasks.push({
      provider: beforeRow.provider,
      task: beforeRow.task,
      beforeSourceIdentities: beforeRow.sourceIdentities,
      afterSourceIdentities: afterRow.sourceIdentities,
      beforeScore: beforeRow.score,
      afterScore: afterRow.score,
      delta: round(afterRow.score - beforeRow.score),
      beforeAttemptCount: beforeRow.attemptCount,
      afterAttemptCount: afterRow.attemptCount,
      beforePassCount: beforeRow.passCount,
      afterPassCount: afterRow.passCount,
      beforePassRate: beforeRow.passRate,
      afterPassRate: afterRow.passRate,
      beforeRowIndices: beforeRow.attempts.map((row) => row.rowIndex),
      afterRowIndices: afterRow.attempts.map((row) => row.rowIndex),
      beforeRows: beforeRow.attempts.map((row) => row.raw),
      afterRows: afterRow.attempts.map((row) => row.raw),
      beforePass: beforeRow.pass,
      afterPass: afterRow.pass,
    });

    const providerTotal = providerTotals.get(beforeRow.provider) ?? {
      before: 0,
      after: 0,
      count: 0,
    };
    providerTotal.before += beforeRow.score;
    providerTotal.after += afterRow.score;
    providerTotal.count += 1;
    providerTotals.set(beforeRow.provider, providerTotal);

    for (const metric of metrics) {
      const metricKey = `${beforeRow.provider}\u0000${metric}`;
      const metricTotal = metricTotals.get(metricKey) ?? {
        provider: beforeRow.provider,
        metric,
        before: 0,
        after: 0,
        count: 0,
      };
      metricTotal.before += beforeRow.namedScores[metric];
      metricTotal.after += afterRow.namedScores[metric];
      metricTotal.count += 1;
      metricTotals.set(metricKey, metricTotal);
    }

    for (const attempt of afterRow.attempts.filter((row) => !row.pass)) {
      failures.push({
        provider: afterRow.provider,
        task: afterRow.task,
        reason: attempt.reason,
      });
    }
  }

  const providers = sorted(providerTotals.keys()).map((provider) => {
    const total = providerTotals.get(provider);
    const beforeScore = average(total.before, total.count);
    const afterScore = average(total.after, total.count);
    return {
      provider,
      beforeScore,
      afterScore,
      delta: round(afterScore - beforeScore),
      taskCount: total.count,
    };
  });
  const metrics = sorted(metricTotals.keys()).map((key) => {
    const total = metricTotals.get(key);
    const beforeScore = average(total.before, total.count);
    const afterScore = average(total.after, total.count);
    return {
      provider: total.provider,
      metric: total.metric,
      beforeScore,
      afterScore,
      delta: round(afterScore - beforeScore),
    };
  });

  return { providers, tasks, metrics, failures };
}

function tableCell(value) {
  return String(value).replaceAll('|', '\\|').replaceAll('\n', '<br>');
}

function points(value) {
  return (value * 100).toFixed(2);
}

function deltaPoints(value) {
  const score = points(value);
  return value > 0 ? `+${score}` : score;
}

// Promptfoo 0.122.0 ResultFailureReason: ASSERT=1, ERROR=2.
// Unclassified failures remain unknown; error text alone is not infrastructure evidence.
function failureCategory(row) {
  return row.failureReason === 2 ? 'infrastructure'
    : row.failureReason === 1 ? 'assertion' : 'unknown';
}

function completeNumbers(rows, select, summarize) {
  const values = rows.map(select);
  if (values.some(value => typeof value !== 'number' || !Number.isFinite(value) || value < 0)) {
    return '不可用';
  }
  return summarize(values);
}

function diagnosticLines(comparison) {
  const groups = [];
  for (const provider of comparison.providers) {
    const tasks = comparison.tasks.filter(task => task.provider === provider.provider);
    groups.push({ provider: provider.provider, task: '全部', tasks });
    for (const task of tasks) groups.push({ provider: task.provider, task: task.task, tasks: [task] });
  }
  const accuracy = [];
  const efficiency = [];
  const failures = [];
  for (const group of groups) {
    for (const side of ['before', 'after']) {
      const rows = group.tasks.flatMap(task => task[`${side}Rows`]);
      const passed = group.tasks.reduce((sum, task) => sum + task[`${side}PassCount`], 0);
      const allThree = group.tasks.filter(task => task[`${side}AttemptCount`] === 3
        && task[`${side}PassCount`] === 3).length;
      const threeTasks = group.tasks.filter(task => task[`${side}AttemptCount`] === 3).length;
      const failed = rows.filter(row => !normalizeRow(row, side, 0).pass);
      const counts = ['infrastructure', 'assertion', 'unknown'].map(category =>
        failed.filter(row => failureCategory(row) === category).length);
      const prefix = `| ${tableCell(group.provider)} | ${tableCell(group.task)} | ${side} |`;
      accuracy.push(`${prefix} ${passed}/${rows.length} (${points(passed / rows.length)}%) | ${allThree}/${threeTasks} | ${counts.join(' | ')} |`);
      const sum = values => values.reduce((total, value) => total + value, 0);
      const tools = completeNumbers(rows, row => {
        const calls = row.metadata?.toolCalls ?? row.response?.metadata?.toolCalls;
        return Array.isArray(calls) ? calls.length : undefined;
      }, sum);
      // Provider usage excludes assertion/judge usage; never substitute grading tokens.
      const input = completeNumbers(rows, row => row.response?.tokenUsage?.prompt, sum);
      const output = completeNumbers(rows, row => row.response?.tokenUsage?.completion, sum);
      const median = completeNumbers(rows, row => row.latencyMs, values => {
        values.sort((a, b) => a - b);
        const middle = Math.floor(values.length / 2);
        return values.length % 2 ? values[middle] : (values[middle - 1] + values[middle]) / 2;
      });
      const maximum = completeNumbers(rows, row => row.latencyMs, values => Math.max(...values));
      efficiency.push(`${prefix} ${tools} | ${input} | ${output} | ${median} | ${maximum} |`);
    }
  }
  for (const task of comparison.tasks) {
    for (const side of ['before', 'after']) {
      for (const [index, raw] of task[`${side}Rows`].entries()) {
        const row = normalizeRow(raw, side, index);
        if (!row.pass) failures.push(`- ${side} 行 ${task[`${side}RowIndices`][index]} / ${tableCell(task.provider)} / ${tableCell(task.task)} / ${failureCategory(raw)}：${tableCell(row.reason)}`);
      }
    }
  }
  return [
    '## 尝试成功率与错误分类', '',
    '| 模型 | 任务 | 侧 | 通过/全部尝试 | 三次全成功题/三次题 | infrastructure | assertion | unknown |',
    '| --- | --- | --- | ---: | ---: | ---: | ---: | ---: |', ...accuracy, '',
    '## 效率（用量合计；延迟 ms）', '',
    '| 模型 | 任务 | 侧 | 工具数 | 输入 token | 输出 token | 延迟中位数 | 延迟最大值 |',
    '| --- | --- | --- | ---: | ---: | ---: | ---: | ---: |', ...efficiency, '',
    '## 原始失败行（各侧导出中从 1 起算）', '', ...(failures.length ? failures : ['无。']), '',
  ];
}

export function renderMarkdownReport(comparison, options = {}) {
  const beforeName = options.beforeName ?? 'before';
  const afterName = options.afterName ?? 'after';
  const lines = [
    '# Promptfoo 前后结果对比',
    '',
    `- before：\`${beforeName}\``,
    `- after：\`${afterName}\``,
    '- 分数单位：百分制；delta = after - before。',
    '',
    '## 模型汇总',
    '',
    '| 模型 | before | after | delta | 任务数 |',
    '| --- | ---: | ---: | ---: | ---: |',
    ...comparison.providers.map(
      (row) =>
        `| ${tableCell(row.provider)} | ${points(row.beforeScore)} | ${points(row.afterScore)} | ${deltaPoints(row.delta)} | ${row.taskCount} |`,
    ),
    '',
    '## 逐任务总分',
    '',
    '| 模型 | 任务 | before | after | delta |',
    '| --- | --- | ---: | ---: | ---: |',
    ...comparison.tasks.map(
      (row) =>
        `| ${tableCell(row.provider)} | ${tableCell(row.task)} | ${points(row.beforeScore)} | ${points(row.afterScore)} | ${deltaPoints(row.delta)} |`,
    ),
    '',
    '## 命名 metric 汇总',
    '',
    '| 模型 | metric | before | after | delta |',
    '| --- | --- | ---: | ---: | ---: |',
    ...comparison.metrics.map(
      (row) =>
        `| ${tableCell(row.provider)} | ${tableCell(row.metric)} | ${points(row.beforeScore)} | ${points(row.afterScore)} | ${deltaPoints(row.delta)} |`,
    ),
    '',
    ...diagnosticLines(comparison),
    '## 源码身份',
    '',
  ];

  for (const task of comparison.tasks.filter(row => row.beforeSourceIdentities)) {
    lines.push(`- 源码身份 ${tableCell(task.provider)} / ${tableCell(task.task)}：before ${tableCell(JSON.stringify(task.beforeSourceIdentities))}；after ${tableCell(JSON.stringify(task.afterSourceIdentities))}`);
  }

  return `${lines.join('\n')}\n`;
}

async function readJson(filePath) {
  try {
    return JSON.parse(await readFile(filePath, 'utf8'));
  } catch (error) {
    throw new Error(`无法读取 ${filePath}：${error.message}`);
  }
}

async function main(args) {
  if (args.length !== 2) {
    throw new Error('用法：node evals/promptfoo/compare-results.mjs <before.json> <after.json>');
  }
  const [beforePath, afterPath] = args;
  const [beforeDocument, afterDocument] = await Promise.all([
    readJson(beforePath),
    readJson(afterPath),
  ]);
  const comparison = compareResults(beforeDocument, afterDocument);
  process.stdout.write(
    renderMarkdownReport(comparison, {
      beforeName: beforePath,
      afterName: afterPath,
    }),
  );
}

const entryPath = process.argv[1] ? pathToFileURL(path.resolve(process.argv[1])).href : undefined;
if (entryPath === import.meta.url) {
  main(process.argv.slice(2)).catch((error) => {
    process.stderr.write(`对比失败：${error.message}\n`);
    process.exitCode = 1;
  });
}
