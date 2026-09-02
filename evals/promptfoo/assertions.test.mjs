import assert from 'node:assert/strict';
import test from 'node:test';

import {
  containsExpectedTerms,
  directRustcodegraphCall,
  editorCodingTrajectory,
  expectedFactsScore,
  maxToolCalls,
  noNavigationFallback,
  noFailedTools,
  readOnlyTrajectory,
  rustcodegraphOperation,
  successfulRustcodegraphCall,
  successfulRun,
} from './assertions.mjs';

function context(metadata, extra = {}) {
  return { providerResponse: { metadata }, ...extra };
}

test('successfulRun requires output and chat.done', () => {
  assert.equal(successfulRun('ok', context({ eventTypes: ['chat.done'] })).pass, true);
  assert.equal(successfulRun('', context({ eventTypes: ['chat.done'] })).pass, false);
  assert.equal(successfulRun('ok', context({ eventTypes: ['chat.error'] })).pass, false);
});

test('tool assertions report failures and enforce limits', () => {
  const toolCalls = [
    { name: 'read', isError: false },
    { name: 'shell', status: 'failed', isError: true },
  ];
  assert.equal(noFailedTools('', context({ toolCalls })).pass, false);
  assert.equal(
    maxToolCalls('', { ...context({ toolCalls }), config: { maxToolCalls: 1 } }).pass,
    false,
  );
});

test('noFailedTools allows failed test attempts when a later test succeeds', () => {
  const result = noFailedTools(
    '',
    context({
      toolCalls: [
        { name: 'shell', input: { command: 'npm test' }, status: 'failed', isError: true },
        { name: 'shell', input: { command: 'npm test' }, status: 'completed', isError: false },
      ],
    }),
  );

  assert.equal(result.pass, true);
});

test('readOnlyTrajectory rejects write tools and mutating shell commands', () => {
  assert.equal(
    readOnlyTrajectory('', context({ toolCalls: [{ name: 'read_file', input: { path: 'x' } }] }))
      .pass,
    true,
  );
  assert.equal(
    readOnlyTrajectory('', context({ toolCalls: [{ name: 'apply_patch', input: {} }] })).pass,
    false,
  );
  assert.equal(
    readOnlyTrajectory(
      '',
      context({ toolCalls: [{ name: 'shell', input: { command: 'git reset --hard HEAD' } }] }),
    ).pass,
    false,
  );
});

test('containsExpectedTerms lists missing terms', () => {
  const result = containsExpectedTerms('alpha beta', {
    vars: { expectedTerms: '["alpha", "gamma"]' },
  });
  assert.equal(result.pass, false);
  assert.match(result.reason, /gamma/);
});

test('directRustcodegraphCall only accepts the builtin tool name', () => {
  const direct = directRustcodegraphCall(
    '',
    context({
      toolCalls: [
        {
          name: 'rustcodegraph',
          input: { operation: 'callers', symbol: 'assemble_session_protocol_messages' },
          status: 'completed',
        },
      ],
    }),
  );
  const cli = directRustcodegraphCall(
    '',
    context({
      toolCalls: [
        {
          name: 'bash',
          input: { command: 'rustcodegraph callers assemble_session_protocol_messages' },
          status: 'completed',
        },
      ],
    }),
  );

  assert.equal(direct.pass, true);
  assert.equal(cli.pass, false);
  assert.match(cli.reason, /内置 rustcodegraph/);
});

test('successfulRustcodegraphCall requires a completed builtin call', () => {
  const failed = successfulRustcodegraphCall(
    '',
    context({
      toolCalls: [
        {
          name: 'rustcodegraph',
          input: { operation: 'callers' },
          status: 'failed',
          isError: true,
        },
      ],
    }),
  );

  assert.equal(failed.pass, false);
  assert.match(failed.reason, /callers.*failed/);
});

test('rustcodegraphOperation matches configured operations and explains mismatches', () => {
  const toolCalls = [
    {
      name: 'rustcodegraph',
      input: { operation: 'callers' },
      status: 'completed',
      isError: false,
    },
  ];
  const matched = rustcodegraphOperation('', {
    ...context({ toolCalls }),
    vars: { expectedOperations: '["callers"]' },
  });
  const mismatched = rustcodegraphOperation('', {
    ...context({ toolCalls }),
    config: { expectedOperations: ['impact'] },
  });

  assert.equal(matched.pass, true);
  assert.equal(mismatched.pass, false);
  assert.match(mismatched.reason, /期望.*impact/);
  assert.match(mismatched.reason, /实际.*callers/);
});

test('rustcodegraphOperation explains failed builtin calls', () => {
  const result = rustcodegraphOperation('', {
    ...context({
      toolCalls: [
        {
          name: 'rustcodegraph',
          input: { operation: 'callers' },
          status: 'failed',
          isError: true,
        },
      ],
    }),
    vars: { expectedOperations: ['callers'] },
  });

  assert.equal(result.pass, false);
  assert.match(result.reason, /未成功.*callers\(failed\)/);
});

test('noNavigationFallback identifies CLI and text-search fallbacks', () => {
  const nativeOnly = noNavigationFallback(
    '',
    context({
      toolCalls: [
        {
          name: 'rustcodegraph',
          input: { operation: 'search', query: 'runner' },
          status: 'completed',
        },
      ],
    }),
  );
  const fallbacks = noNavigationFallback(
    '',
    context({
      toolCalls: [
        { name: 'bash', input: { command: 'rustcodegraph callers runner' }, status: 'completed' },
        { name: 'shell', input: { cmd: 'rg "runner" crates/' }, status: 'completed' },
      ],
    }),
  );

  assert.equal(nativeOnly.pass, true);
  assert.equal(fallbacks.pass, false);
  assert.match(fallbacks.reason, /CLI fallback/);
  assert.match(fallbacks.reason, /文本搜索 fallback/);
});

test('expectedFactsScore awards partial credit and lists missing facts', () => {
  const result = expectedFactsScore('alpha and gamma', {
    vars: { expectedTerms: '["alpha", "beta", "gamma"]' },
  });

  assert.equal(result.pass, false);
  assert.equal(result.score, 2 / 3);
  assert.match(result.reason, /2\/3/);
  assert.match(result.reason, /beta/);
});

const EDITOR_ROOT = 'examples/editor2026-08-26_12_34_56';

function completedTool(name, input) {
  return { name, input, status: 'completed', isError: false };
}

function editorPatch(files) {
  return files.map((file) => `*** Add File: ${file}\n+fixture`).join('\n');
}

function successfulEditorTest(root = EDITOR_ROOT) {
  return completedTool('shell', { command: `node --test ${root}/editor.test.ts` });
}

test('editorCodingTrajectory accepts scoped TypeScript writes and a successful test', () => {
  const result = editorCodingTrajectory(
    '',
    context({
      toolCalls: [
        completedTool('apply_patch', {
          patchText: editorPatch([
            `${EDITOR_ROOT}/editor.ts`,
            `${EDITOR_ROOT}/editor.test.ts`,
          ]),
        }),
        successfulEditorTest(),
      ],
    }),
  );

  assert.equal(result.pass, true);
  assert.match(result.reason, /editor2026-08-26_12_34_56/);
});

test('editorCodingTrajectory accepts file_write path aliases and scoped shell writes', () => {
  for (const pathField of ['path', 'filePath', 'file_path']) {
    const result = editorCodingTrajectory(
      '',
      context({
        toolCalls: [
          completedTool('file_write', {
            [pathField]: `${EDITOR_ROOT}/editor.ts`,
            content: 'export class Editor {}',
          }),
          completedTool('shell', {
            command: `mkdir -p ${EDITOR_ROOT}/fixtures && touch ${EDITOR_ROOT}/fixtures/empty.txt`,
          }),
          successfulEditorTest(),
        ],
      }),
    );

    assert.equal(result.pass, true, pathField);
  }
});

test('editorCodingTrajectory rejects writes outside the timestamp editor directory', () => {
  const result = editorCodingTrajectory(
    '',
    context({
      toolCalls: [
        completedTool('apply_patch', {
          patch: editorPatch([`${EDITOR_ROOT}/editor.ts`, 'README.md']),
        }),
        successfulEditorTest(),
      ],
    }),
  );

  assert.equal(result.pass, false);
  assert.match(result.reason, /README\.md/);
});

test('editorCodingTrajectory rejects shell writes outside the editor directory', () => {
  const result = editorCodingTrajectory(
    '',
    context({
      toolCalls: [
        completedTool('shell', {
          command: `touch ${EDITOR_ROOT}/editor.ts README.md`,
        }),
        successfulEditorTest(),
      ],
    }),
  );

  assert.equal(result.pass, false);
  assert.match(result.reason, /README\.md/);
});

test('editorCodingTrajectory requires a TypeScript artifact', () => {
  const result = editorCodingTrajectory(
    '',
    context({
      toolCalls: [
        completedTool('file_write', {
          path: `${EDITOR_ROOT}/package.json`,
          content: '{}',
        }),
        successfulEditorTest(),
      ],
    }),
  );

  assert.equal(result.pass, false);
  assert.match(result.reason, /TypeScript/);
});

test('editorCodingTrajectory requires a successful test command', () => {
  const writes = completedTool('file_write', {
    path: `${EDITOR_ROOT}/editor.ts`,
    content: 'export class Editor {}',
  });
  const failed = editorCodingTrajectory(
    '',
    context({
      toolCalls: [
        writes,
        {
          name: 'shell',
          input: { command: `node --test ${EDITOR_ROOT}/editor.test.ts` },
          status: 'failed',
          isError: true,
        },
      ],
    }),
  );
  const missing = editorCodingTrajectory('', context({ toolCalls: [writes] }));

  assert.equal(failed.pass, false);
  assert.match(failed.reason, /未运行.*测试/);
  assert.equal(missing.pass, false);
  assert.match(missing.reason, /未运行.*测试/);
});

test('editorCodingTrajectory accepts a later successful test after an earlier failed attempt', () => {
  const result = editorCodingTrajectory(
    '',
    context({
      toolCalls: [
        completedTool('file_write', {
          path: `${EDITOR_ROOT}/editor.ts`,
          content: 'export class Editor {}',
        }),
        {
          name: 'shell',
          input: { command: 'npm test' },
          status: 'failed',
          isError: true,
        },
        successfulEditorTest(),
      ],
    }),
  );

  assert.equal(result.pass, true);
});

test('editorCodingTrajectory does not accept an unobservable write target', () => {
  const result = editorCodingTrajectory(
    '',
    context({
      toolCalls: [
        completedTool('file_write', { content: 'export class Editor {}' }),
        successfulEditorTest(),
      ],
    }),
  );

  assert.equal(result.pass, false);
  assert.match(result.reason, /无法识别.*写入路径/);
});

test('editorCodingTrajectory requires writes to complete successfully', () => {
  const result = editorCodingTrajectory(
    '',
    context({
      toolCalls: [
        {
          name: 'file_write',
          input: { path: `${EDITOR_ROOT}/editor.ts`, content: 'export class Editor {}' },
          status: 'failed',
          isError: true,
        },
        successfulEditorTest(),
      ],
    }),
  );

  assert.equal(result.pass, false);
  assert.match(result.reason, /写入工具未成功/);
});
