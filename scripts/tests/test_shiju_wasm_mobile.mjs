// Complete-product canvas integration. Run only after generating a fresh WASM bundle.
// Screenshots require human review; this does not certify physical phone support.
import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
import { mkdir } from 'node:fs/promises';
import { join } from 'node:path';
const { chromium } = createRequire(import.meta.url)('playwright');
const output = process.env.SHIJU_MOBILE_SCREENSHOTS || '/tmp/shiju-mobile-integration';
await mkdir(output, { recursive: true });
const browser = await chromium.launch({ channel: 'chrome', headless: true,
    args: ['--enable-unsafe-webgpu', '--use-angle=swiftshader'] });
try {
    for (const width of [360, 390, 430, 844, 1440]) {
        const height = ({ 360: 800, 390: 844, 430: 932, 844: 390, 1440: 900 })[width];
        for (const colorScheme of ['light', 'dark']) {
            for (const deviceScaleFactor of [1, 2, 3]) {
                const context = await browser.newContext({ viewport: { width, height },
                    colorScheme, deviceScaleFactor, hasTouch: true });
                try {
                    const page = await context.newPage();
                    const errors = [];
                    page.on('pageerror', error => errors.push(String(error)));
                    page.on('console', message => {
                        if (/panicked at|RuntimeError|unreachable executed/.test(message.text())) errors.push(message.text());
                    });
                    let requests = 0;
                    await page.route('**/v1/**', async route => {
                        requests++;
                        await route.fulfill({ status: 401, contentType: 'application/json',
                            headers: { 'access-control-allow-origin': '*' },
                            body: JSON.stringify({ error: { code: 'unauthorized', message: 'Invalid credentials' } }) });
                    });
                    await page.goto(process.env.SHIJU_WASM_URL || 'http://127.0.0.1:8765/');
                    await page.waitForSelector('canvas', { timeout: Number(process.env.SHIJU_WASM_TIMEOUT || 120000) });
                    await page.waitForFunction(() => document.querySelector('canvas')?.width > 0);
                    await page.waitForTimeout(10000);
                    const prefix = join(output, `${width}-${height}-${colorScheme}-${deviceScaleFactor}`);
                    const capture = async name => page.screenshot({ path: `${prefix}-${name}.png` });
                    const initial = await capture('initial');
                    if (width < 768) {
                        // The mobile shell uses 44px actions and 7px outer padding.
                        await page.touchscreen.tap(width - 35, 29);
                        await page.waitForTimeout(600);
                        const drawer = await capture('drawer');
                        assert.notDeepEqual(drawer, initial, 'Project action must change the rendered canvas');
                        await page.touchscreen.tap(width - 35, 29);
                        await page.waitForTimeout(600);
                        await page.touchscreen.tap(120, height - 29);
                        await page.waitForTimeout(600);
                        const settings = await capture('settings');
                        assert.notDeepEqual(settings, initial, 'Settings action must change the rendered canvas');
                        await page.touchscreen.tap(30, 29);
                        await page.waitForTimeout(600);
                        const returned = await capture('returned');
                        assert.notDeepEqual(returned, settings, 'Back must leave settings');
                        // Explicit mobile layout survives crossing the automatic breakpoint.
                        await page.touchscreen.tap(235, 77);
                        await page.setViewportSize({ width: 844, height: 390 });
                        await page.waitForTimeout(600);
                        await capture('landscape-mobile');
                        await page.setViewportSize({ width, height });
                        await page.waitForTimeout(600);
                        await capture('portrait-restored');
                    }
                    assert.ok(requests > 0, 'The real app must reach the intercepted gateway');
                    assert.equal(await page.evaluate(() => window.shijuInitError), undefined);
                    assert.deepEqual(errors, [], 'Canvas navigation and resizing must not panic');
                    console.log(`PASS canvas navigation ${width}x${height} ${colorScheme} DPR ${deviceScaleFactor}`);
                } finally { await context.close(); }
            }
        }
    }
} finally { await browser.close(); }
