import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';

// Evaluator only. Hand-derived complete answers: one section value, one CRLF
// insertion, one complete record removal, one UTF-8 path edit. Never copy into attempts.
export const cases = [
  {
    "taskId": "edit-01",
    "split": "dev",
    "target": "settings.ini",
    "fixtureFiles": [
      "settings.ini",
      "notes.txt"
    ],
    "reference": "[server]\nport = 6287\n[backup]\nport = 4100\n",
    "mutation": "[server]\nport = 6287\n[backup]\nport = 6287\n"
  },
  {
    "taskId": "edit-02",
    "split": "dev",
    "target": "routes.txt",
    "fixtureFiles": [
      "routes.txt",
      "notes.txt"
    ],
    "reference": "north\r\n# INSERT ROUTE HERE\r\neast\r\nsouth\r\n",
    "mutation": "north\n# INSERT ROUTE HERE\neast\nsouth\n"
  },
  {
    "taskId": "edit-03",
    "split": "dev",
    "target": "records.csv",
    "fixtureFiles": [
      "records.csv",
      "notes.txt"
    ],
    "reference": "id,value\nkeep-1,7\nkeep-3,11",
    "mutation": "id,value\nkeep-1,7\nkeep-3,11\n"
  },
  {
    "taskId": "edit-04",
    "split": "holdout",
    "target": "资料/生产/设置.txt",
    "fixtureFiles": [
      "资料/生产/设置.txt",
      "资料/备份/设置.txt",
      "设置.txt"
    ],
    "reference": "区域=华东\n重试=5\n",
    "mutation": "区域=华东\n重试=5"
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
