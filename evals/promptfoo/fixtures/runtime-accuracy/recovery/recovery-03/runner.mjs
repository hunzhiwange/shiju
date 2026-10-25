import { readFileSync, writeFileSync, existsSync } from 'node:fs';
const mode = process.argv[2];
const fail = message => { console.error(message); process.exit(2); };
if (mode === 'build') {
  const input = readFileSync('input.txt', 'utf8');
  writeFileSync('result.txt', input === 'format=full\n' ? 'status=ready\n' : 'ready\n');
} else if (mode === 'check') {
  if (!existsSync('result.txt') || readFileSync('result.txt', 'utf8') !== 'status=ready\n') fail('Invalid result; set input.txt to format=full with LF, rebuild and check');
} else fail('Use build or check');
