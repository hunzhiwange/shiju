import { readFileSync, writeFileSync, existsSync } from 'node:fs';
const mode = process.argv[2];
const fail = message => { console.error(message); process.exit(2); };
if (mode !== 'build') fail('Use build');
const input = readFileSync('input.txt', 'utf8');
if (input !== 'count=3\n') fail('Expected count=3 with LF; correct input.txt then retry');
writeFileSync('result.txt', `total=${Number(input.trim().split('=')[1]) * 4}\n`);
