// Run manually against a complete WASM build. Canvas captures require visual review.
// Uses an isolated browser context and intercepts all gateway requests.
import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
const require = createRequire(import.meta.url);
const { chromium } = require('playwright');
const browser = await chromium.launch({
    channel: 'chrome', headless: true,
    args: ['--enable-unsafe-webgpu', '--use-angle=swiftshader'],
});
try {
    for (const viewport of [{ width: 360, height: 800 }, { width: 740, height: 360 }]) {
        const context = await browser.newContext({ viewport });
        const page = await context.newPage();
        const errors = [];
        page.on('pageerror', error => errors.push(String(error)));
        page.on('console', message => {
            if (/panicked at|RuntimeError|unreachable executed/.test(message.text())) errors.push(message.text());
        });
        // Failed auth stays at the network boundary; no real settings or models are used.
        let requests = 0;
        await page.route('**/v1/**', async route => {
            requests++;
            await route.fulfill({ status: 401, contentType: 'application/json',
                headers: { 'access-control-allow-origin': '*' },
                body: JSON.stringify({ error: { code: 'unauthorized', message: 'Invalid credentials' } }),
            });
        });
        await page.goto(process.env.SHIJU_WASM_URL || 'http://127.0.0.1:8765/');
        await page.waitForSelector('canvas', { timeout: Number(process.env.SHIJU_WASM_TIMEOUT || 120000) });
        await page.waitForTimeout(10000);
        // Mobile shell footer: settings -> first category (gateway).
        await page.mouse.click(120, viewport.height - 29);
        await page.waitForTimeout(1000);
        await page.screenshot({ path: `/tmp/shiju-mobile-settings-${viewport.width}-categories.png` });
        await page.mouse.click(86, 178);
        await page.waitForTimeout(1000);
        await page.screenshot({ path: `/tmp/shiju-mobile-settings-${viewport.width}-gateway.png` });
        assert.ok(requests > 0, 'gateway fixture must receive real application requests');
        assert.deepEqual(errors, [], 'mobile settings rendering must not panic');
        // Real keyboards, visible labels and error feedback still require manual review.
        await context.close();
    }
} finally {
    await browser.close();
}
