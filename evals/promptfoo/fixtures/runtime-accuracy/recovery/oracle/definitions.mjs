import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';

// Evaluator-only hand-derived bytes: 3*4=12, 3*7=21, full ready marker,
// documented plain mode. Answers and recovery exceptions never enter attempts.
export const cases = [
  {
    "taskId": "recovery-01",
    "split": "dev",
    "fixtureFiles": [
      "runner.mjs",
      "README.txt",
      "input.txt"
    ],
    "operation": "node runner.mjs build",
    "recoveryOperation": "node runner.mjs build",
    "repairs": {
      "input.txt": "count=3\n"
    },
    "expectedFiles": {
      "input.txt": "count=3\n",
      "result.txt": "total=12\n"
    }
  },
  {
    "taskId": "recovery-02",
    "split": "dev",
    "fixtureFiles": [
      "runner.mjs",
      "README.txt"
    ],
    "operation": "node runner.mjs build",
    "recoveryOperation": "node runner.mjs build",
    "repairs": {
      "local.txt": "unit=7\n"
    },
    "expectedFiles": {
      "local.txt": "unit=7\n",
      "result.txt": "total=21\n"
    }
  },
  {
    "taskId": "recovery-03",
    "split": "dev",
    "fixtureFiles": [
      "runner.mjs",
      "README.txt",
      "input.txt"
    ],
    "operation": "node runner.mjs build && node runner.mjs check",
    "recoveryOperation": "node runner.mjs build && node runner.mjs check",
    "repairs": {
      "input.txt": "format=full\n"
    },
    "expectedFiles": {
      "input.txt": "format=full\n",
      "result.txt": "status=ready\n"
    }
  },
  {
    "taskId": "recovery-04",
    "split": "holdout",
    "fixtureFiles": [
      "runner.mjs",
      "README.txt"
    ],
    "operation": "node runner.mjs --format=compact",
    "recoveryOperation": "node runner.mjs --format=plain",
    "repairs": {},
    "expectedFiles": {
      "result.txt": "mode=plain\n"
    }
  }
].map(c => ({ ...c,
  fixtureRoot: fileURLToPath(new URL(`../${c.taskId}/`, import.meta.url)),
}));

export const definitions = new Map(cases.map(c => [c.taskId, {
  expectedFiles: Object.fromEntries(Object.entries(c.expectedFiles).map(([file, bytes]) => [file, Buffer.from(bytes)])),
  unchangedFiles: Object.fromEntries(c.fixtureFiles.filter(file => !Object.hasOwn(c.expectedFiles, file))
    .map(file => [file, readFileSync(new URL(`../${c.taskId}/${file}`, import.meta.url))])),
  allowExtraFiles: false,
  expectedFailure: { toolName: 'bash', operation: c.operation, exitCode: 2,
    recoveryOperation: c.recoveryOperation },
}]));
