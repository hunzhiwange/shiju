// Real Iced/WGPU fixture. WEB_IME_DIST contains wasm-bindgen --target web output.
import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
import { createServer } from 'node:http';
import { readFile } from 'node:fs/promises';
import { resolve, sep } from 'node:path';
const require = createRequire(import.meta.url);
const { chromium } = require('playwright');
assert.ok(process.env.WEB_IME_DIST, 'Set WEB_IME_DIST to the generated fixture directory');
const root = resolve(process.env.WEB_IME_DIST);
const server = createServer(async (req, res) => {
    if (req.url === '/favicon.ico') { res.writeHead(204); res.end(); return; }
    if (req.url === '/') {
        res.setHeader('Content-Type', 'text/html');
        res.end('<body style="margin:0"><script type="module">import init from "/web_ime.js";init().catch(e => window.fixtureError=String(e));</script>');
        return;
    }
    const path = resolve(root, decodeURIComponent(req.url.split('?')[0]).slice(1));
    if (!path.startsWith(root + sep)) { res.writeHead(403); res.end(); return; }
    try {
        res.setHeader('Content-Type', path.endsWith('.wasm') ? 'application/wasm' : 'text/javascript');
        res.end(await readFile(path));
    } catch { res.writeHead(404); res.end(); }
});
await new Promise(resolve => server.listen(0, '127.0.0.1', resolve));
const browser = await chromium.launch({ channel: 'chrome', headless: true, args: ['--enable-unsafe-webgpu', '--use-angle=swiftshader'] });
try {
    const page = await browser.newPage({ viewport: { width: 800, height: 600 } });
    const errors = [];
    page.on('pageerror', e => { errors.push(String(e)); console.error('PAGE ERROR', String(e)); });
    page.on('console', m => { if (m.type() === 'error') errors.push(m.text()); });
    await page.goto(`http://127.0.0.1:${server.address().port}`);
    await page.waitForSelector('canvas', { timeout: 60000 });
    await page.waitForFunction(() => document.querySelector('canvas').width > 0);
    await page.waitForTimeout(1000);
    async function focus(x, y) {
        await page.mouse.click(x, y);
        await page.waitForFunction(() => document.activeElement?.hasAttribute('data-iced-ime'));
    }
    async function chinese(text) {
        const cdp = await page.context().newCDPSession(page);
        await cdp.send('Input.imeSetComposition', { text: 'nihao', selectionStart: 5, selectionEnd: 5 });
        await cdp.send('Input.insertText', { text });
        await cdp.detach();
    }
    await focus(100, 64);
    await chinese('你好');
    await page.waitForFunction(() => document.documentElement.getAttribute("data-ime-fixture") === '你好||0');
    console.log('PASS rendered single-line Chinese');
    await page.keyboard.press('Control+A');
    await chinese('替换');
    await page.waitForTimeout(300);
    await page.waitForFunction(() => document.documentElement.getAttribute("data-ime-fixture") === '替换||0');
    console.log('PASS rendered selection replaced once');
    await focus(100, 150);
    await chinese('多行');
    await page.keyboard.press('Enter');
    await page.keyboard.type('abc');
    await page.waitForTimeout(400);
    await page.waitForFunction(() => document.documentElement.getAttribute("data-ime-fixture") === '替换|多行\nabc|0');
    console.log('PASS rendered multiline editor and Enter');
    await focus(100, 330);
    await chinese('密码');
    await page.waitForFunction(() => document.documentElement.getAttribute("data-ime-fixture") === '替换|多行\nabc|2');
    console.log('PASS rendered secure input without field crossover');
    await page.screenshot({ path: process.env.WEB_IME_SCREENSHOT || '/tmp/shiju-web-ime-rendering.png' });
    assert.deepEqual(errors, [], 'No browser errors or shaping panics');
    console.log('4 rendered Iced tests passed');
} finally {
    await browser.close();
    await new Promise(resolve => server.close(resolve));
}
