// Run against a complete WASM build served by scripts/serve_shiju_wasm.py.
// All gateway requests are intercepted; this test never calls a real model.
import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
const require = createRequire(import.meta.url);
const { chromium } = require('playwright');
const browser = await chromium.launch({
    channel: 'chrome',
    headless: true,
    args: ['--enable-unsafe-webgpu', '--use-angle=swiftshader'],
});
try {
    const page = await browser.newPage({ viewport: { width: 1440, height: 900 } });
    const errors = [];
    const chats = [];
    page.on('pageerror', error => errors.push(String(error)));
    page.on('console', message => {
        if (/panicked at|RuntimeError|unreachable executed/.test(message.text())) {
            errors.push(message.text());
        }
    });
    const project = {
        id: 'wasm-project', name: 'WASM chat test',
        directory: '/wasm-test', display_path: '/wasm-test', status: 'ready',
        created_at_ms: 1, updated_at_ms: 1,
        git: { is_repo: false, has_uncommitted_changes: false },
    };
    await page.route('**/v1/**', async route => {
        const request = route.request();
        const path = new URL(request.url()).pathname;
        const headers = { 'access-control-allow-origin': '*' };
        if (path === '/v1/chat/stream') {
            assert.equal(request.method(), 'POST');
            assert.ok(request.headers()['x-vw-stream-request-id']);
            chats.push(request.postDataJSON());
            await route.fulfill({
                contentType: 'text/event-stream', headers,
                body: 'id: 1\ndata: {"type":"chat.delta","delta":"WASM response"}\n\n'
                    + 'id: 2\ndata: {"type":"chat.done","finish_reason":"stop"}\n\n',
            });
            return;
        }
        let data = { items: [] };
        if (path === '/v1/projects') data = { items: [project] };
        if (path === '/v1/projects/resolve' || path === '/v1/projects/wasm-project') {
            data = { project };
        }
        await route.fulfill({ contentType: 'application/json', headers, body: JSON.stringify(data) });
    });
    await page.goto(process.env.SHIJU_WASM_URL || 'http://127.0.0.1:8765/');
    await page.waitForSelector('canvas', {
        timeout: Number(process.env.SHIJU_WASM_TIMEOUT || 120000),
    });
    // Iced paints asynchronously after the gateway project list arrives.
    await page.waitForTimeout(10000);
    await page.mouse.click(1375, 425); // Open the fixture's recent-project card.
    await page.waitForTimeout(5000);
    for (const prompt of ['WASM first request', 'WASM second request']) {
        await page.mouse.click(650, 770);
        await page.keyboard.type(prompt);
        const sent = page.waitForRequest(request =>
            new URL(request.url()).pathname === '/v1/chat/stream', { timeout: 15000 });
        await page.mouse.click(1285, 843);
        try {
            await sent;
        } catch (error) {
            assert.deepEqual(errors, [], 'Sending must not panic before reaching the gateway');
            throw error;
        }
        await page.waitForTimeout(2000);
        assert.deepEqual(errors, [], 'SSE response and terminal event must not panic');
    }
    assert.equal(chats.length, 2, 'A completed response must allow another turn');
    assert.equal(chats[0].session_id, chats[1].session_id, 'Both turns stay in the same session');
    for (const [index, prompt] of ['WASM first request', 'WASM second request'].entries()) {
        assert.equal(chats[index].messages.at(-1).content, prompt);
    }
    await page.screenshot({ path: '/tmp/shiju-wasm-chat-smoke.png' });
    console.log('PASS WASM sends two chat turns and consumes SSE responses without a panic');
} finally {
    await browser.close();
}
