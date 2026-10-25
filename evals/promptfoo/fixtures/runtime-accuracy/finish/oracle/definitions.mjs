import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';

// Trusted, hand-derived answers: literal ready marker, already-ready marker,
// requested count, and both ordered stage outputs. Never copy into attempts.
export const cases = [
  {
    "taskId": "finish-01",
    "split": "dev",
    "fixtureFiles": [
      "notes.txt"
    ],
    "referenceFiles": {
      "result.txt": "status=ready\n"
    }
  },
  {
    "taskId": "finish-02",
    "split": "dev",
    "fixtureFiles": [
      "notes.txt",
      "result.txt"
    ],
    "referenceFiles": {
      "result.txt": "status=ready\n"
    }
  },
  {
    "taskId": "finish-03",
    "split": "dev",
    "fixtureFiles": [
      "notes.txt",
      "result.txt"
    ],
    "referenceFiles": {
      "result.txt": "count=3\n"
    }
  },
  {
    "taskId": "finish-04",
    "split": "holdout",
    "fixtureFiles": [
      "notes.txt"
    ],
    "referenceFiles": {
      "result.txt": "stage=prepared\n",
      "receipt.txt": "stage=finished\n"
    },
    "missingFile": "receipt.txt"
  }
].map(c => ({ ...c,
  fixtureRoot: fileURLToPath(new URL(`../${c.taskId}/`, import.meta.url)),
}));

export const definitions = new Map(cases.map(c => [c.taskId, {
  expectedFiles: Object.fromEntries(Object.entries(c.referenceFiles).map(([file, bytes]) => [file, Buffer.from(bytes)])),
  unchangedFiles: Object.fromEntries(c.fixtureFiles.filter(file => !Object.hasOwn(c.referenceFiles, file))
    .map(file => [file, readFileSync(new URL(`../${c.taskId}/${file}`, import.meta.url))])),
  allowExtraFiles: false,
}]));
