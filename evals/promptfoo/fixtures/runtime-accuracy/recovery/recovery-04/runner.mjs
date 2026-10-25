import { readFileSync, writeFileSync, existsSync } from 'node:fs';
const mode = process.argv[2];
const fail = message => { console.error(message); process.exit(2); };
if (mode !== '--format=plain') fail('Unsupported argument; consult README.txt');
writeFileSync('result.txt', 'mode=plain\n');
