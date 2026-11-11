#!/usr/bin/env node
// Serve the extracted Web archive using only Node.js built-in modules.
import { createServer } from 'node:http';
import { createReadStream } from 'node:fs';
import { realpath, stat } from 'node:fs/promises';
import { dirname, extname, isAbsolute, relative, resolve, sep } from 'node:path';
import { fileURLToPath } from 'node:url';

const args = process.argv.slice(2);
if (args.length && (args.length !== 2 || args[0] !== '--port' || !/^\d+$/.test(args[1]))) {
    console.error('Usage: node serve.mjs [--port 8765]');
    process.exit(1);
}
const port = args.length ? Number(args[1]) : 8765;
if (!Number.isSafeInteger(port) || port > 65535) {
    console.error('Port must be between 0 and 65535');
    process.exit(1);
}
const root = await realpath(dirname(fileURLToPath(import.meta.url)));
for (const name of ['index.html', 'shi_ju.js', 'shi_ju_bg.wasm']) {
    try {
        if (!(await stat(resolve(root, name))).isFile()) throw new Error('not a file');
    } catch {
        console.error(`Missing ${name}; extract the complete ShiJu Web archive`);
        process.exit(1);
    }
}
const mime = {
    '.html': 'text/html; charset=utf-8',
    '.js': 'text/javascript; charset=utf-8',
    '.mjs': 'text/javascript; charset=utf-8',
    '.wasm': 'application/wasm',
    '.json': 'application/json',
    '.css': 'text/css; charset=utf-8',
    '.svg': 'image/svg+xml',
    '.png': 'image/png',
    '.ico': 'image/x-icon',
    '.md': 'text/plain; charset=utf-8',
};
function withinRoot(path) {
    const name = relative(root, path);
    return name !== '..' && !name.startsWith('..' + sep) && !isAbsolute(name);
}
const server = createServer(async (request, response) => {
    if (request.method !== 'GET' && request.method !== 'HEAD') {
        response.writeHead(405, { Allow: 'GET, HEAD' });
        response.end();
        return;
    }
    let pathname;
    try {
        pathname = decodeURIComponent(new URL(request.url, 'http://localhost').pathname);
        if (pathname.includes('\\') || pathname.includes('\0')) throw new Error('invalid path');
    } catch {
        response.writeHead(400); response.end(); return;
    }
    if (pathname === '/favicon.ico') { response.writeHead(204); response.end(); return; }
    if (pathname.endsWith('/')) pathname += 'index.html';
    const path = resolve(root, '.' + pathname);
    if (!withinRoot(path)) { response.writeHead(403); response.end(); return; }
    try {
        const actual = await realpath(path);
        if (!withinRoot(actual)) { response.writeHead(403); response.end(); return; }
        const info = await stat(actual);
        if (!info.isFile()) { response.writeHead(404); response.end(); return; }
        response.writeHead(200, {
            'Content-Type': mime[extname(actual)] || 'application/octet-stream',
            'Content-Length': info.size,
            'X-Content-Type-Options': 'nosniff',
        });
        if (request.method === 'HEAD') { response.end(); return; }
        const stream = createReadStream(actual);
        stream.on('error', () => response.destroy());
        response.on('close', () => stream.destroy());
        stream.pipe(response);
    } catch {
        response.writeHead(404); response.end();
    }
});
server.on('error', error => { console.error(error.message); process.exitCode = 1; });
server.listen(port, '127.0.0.1', () => {
    console.log(`ShiJu Web: http://127.0.0.1:${server.address().port}/`);
});
