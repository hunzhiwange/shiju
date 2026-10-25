import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';

// Evaluator-only hand-derived answers: retained directory, retained ID mapping,
// protected-file constraint, and latest requirement. Never copy into attempts.
export const cases = [
  {
    "taskId": "state-01",
    "split": "dev",
    "target": "east/settings.txt",
    "fixtureFiles": [
      "east/settings.txt",
      "west/settings.txt"
    ],
    "reference": "mode=ready\n",
    "mutation": "mode=idle\n"
  },
  {
    "taskId": "state-02",
    "split": "dev",
    "target": "result.txt",
    "fixtureFiles": [
      "ids.csv",
      "result.txt"
    ],
    "reference": "K17\n",
    "mutation": "M42\n"
  },
  {
    "taskId": "state-03",
    "split": "dev",
    "target": "allowed.txt",
    "fixtureFiles": [
      "locked.txt",
      "allowed.txt"
    ],
    "reference": "value=new\n",
    "mutation": "value=old\n"
  },
  {
    "taskId": "state-04",
    "split": "holdout",
    "target": "result.txt",
    "fixtureFiles": [
      "result.txt",
      "notes.txt"
    ],
    "reference": "color=green\n",
    "mutation": "color=blue\n"
  }
].map(c => ({ ...c,
  fixtureRoot: fileURLToPath(new URL(`../${c.taskId}/`, import.meta.url)),
  reference: Buffer.from(c.reference), mutation: Buffer.from(c.mutation),
}));

export const definitions = new Map(cases.map(c => [c.taskId, {
  expectedFiles: { [c.target]: c.reference },
  unchangedFiles: Object.fromEntries(c.fixtureFiles.filter(file => file !== c.target)
    .map(file => [file, readFileSync(new URL(`../${c.taskId}/${file}`, import.meta.url))])),
  allowExtraFiles: false,
}]));
