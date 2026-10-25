import { readFileSync, writeFileSync, existsSync } from 'node:fs';
const mode = process.argv[2];
const fail = message => { console.error(message); process.exit(2); };
if (mode !== 'build') fail('Use build');
if (!existsSync('local.txt')) fail('Missing local.txt; use the literal line from README.txt');
if (readFileSync('local.txt', 'utf8') !== 'unit=7\n') fail('Invalid local dependency');
const unit = Number(readFileSync('local.txt', 'utf8').trim().split('=')[1]);
writeFileSync('result.txt', `total=${unit * 3}\n`);
