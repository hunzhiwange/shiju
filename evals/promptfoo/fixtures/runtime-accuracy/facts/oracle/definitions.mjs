import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';

// Evaluator only: references are manually derived, never copied into attempts.
// 01: version 2.1; 02: 17 + 23 + 31; 03: storage declaration; 04: literal UTF-8 value.
export const cases = [
  {
    "taskId": "facts-01",
    "split": "dev",
    "fixtureFiles": [
      "release-old.json",
      "release-current.json"
    ],
    "reference": {
      "port": 6287
    },
    "mutation": {
      "port": 4100
    }
  },
  {
    "taskId": "facts-02",
    "split": "dev",
    "fixtureFiles": [
      "records.csv"
    ],
    "reference": {
      "total": 71
    },
    "mutation": {
      "total": 221
    }
  },
  {
    "taskId": "facts-03",
    "split": "dev",
    "fixtureFiles": [
      "modules/network.txt",
      "modules/storage.txt"
    ],
    "reference": {
      "modulePath": "services/storage"
    },
    "mutation": {
      "modulePath": "services/network"
    }
  },
  {
    "taskId": "facts-04",
    "split": "holdout",
    "fixtureFiles": [
      "设置.txt"
    ],
    "reference": {
      "监听区域": "华东二区"
    },
    "mutation": {
      "监听区域": "星河"
    }
  }
].map(c => ({ ...c, fixtureRoot: fileURLToPath(new URL(`../${c.taskId}/`, import.meta.url)) }));

export const definitions = new Map(cases.map(c => [c.taskId, {
  expectedFiles: {},
  unchangedFiles: Object.fromEntries(c.fixtureFiles.map(file => [file, readFileSync(new URL(`../${c.taskId}/${file}`, import.meta.url))])),
  expectedJson: c.reference,
  allowExtraFiles: false,
}]));
