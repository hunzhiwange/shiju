// DOM viewport contract only: no WASM renderer or physical keyboard is exercised.
// visualViewport, safe-area and lifecycle changes below are explicitly simulated.
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { createServer } from 'node:http';
import { createRequire } from 'node:module';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
const require = createRequire(import.meta.url);
const { chromium } = require('playwright');
const html = await readFile(new URL('../../crates/vw-desktop/index.html', import.meta.url), 'utf8');
const server = createServer((request, response) => {
    response.writeHead(200, { 'content-type': 'text/html; charset=utf-8' });
    response.end(html);
});
await new Promise(resolve => server.listen(0, '127.0.0.1', resolve));
let browser;
let passed = 0;
const near = (actual, expected) => assert.ok(Math.abs(actual - expected) < 1,
    `Expected ${actual} to be within one CSS pixel of ${expected}`);
try {
    browser = await chromium.launch({ channel: 'chrome', headless: true });
    for (const deviceScaleFactor of [1, 2, 3]) {
        for (const [width, height] of [[360, 800], [390, 844], [430, 932], [844, 390]]) {
            const context = await browser.newContext({
                viewport: { width, height }, deviceScaleFactor, hasTouch: true, isMobile: true,
            });
            const page = await context.newPage();
            try {
                await page.goto(`http://127.0.0.1:${server.address().port}/`);
                await page.evaluate(() => {
                    const canvas = document.createElement('canvas');
                    // Mirror iced_winit's replacement and inline style without faking
                    // pixel buffer sizing: only winit can validate the renderer chain.
                    canvas.style.cssText = 'display: block; width: 100%; height: 100%';
                    document.querySelector('#vibe-window-app-container').replaceWith(canvas);
                    window.hits = [];
                    canvas.addEventListener('pointerdown', event => {
                        const rect = canvas.getBoundingClientRect();
                        window.hits.push([event.clientX - rect.left, event.clientY - rect.top]);
                    });
                });
                const geometry = () => page.evaluate(() => {
                    const rect = document.querySelector('canvas').getBoundingClientRect();
                    return { x: rect.x, y: rect.y, width: rect.width, height: rect.height };
                });
                let rect = await geometry();
                near(rect.width, width);
                near(rect.height, height);
                await page.touchscreen.tap(rect.x + 40, rect.y + 50);
                assert.deepEqual(await page.evaluate(() => window.hits.at(-1)), [40, 50]);
                // Simulate safe-area values, since Chromium emulation has no notch.
                await page.addStyleTag({ content: '#shiju-viewport { padding: 20px 7px 12px 9px; }' });
                rect = await geometry();
                near(rect.x, 9);
                near(rect.y, 20);
                near(rect.width, width - 16);
                near(rect.height, height - 32);
                // Simulate keyboard viewport resize/scroll at scale 1.
                await page.evaluate(() => {
                    for (const [key, value] of Object.entries({
                        width: 320, height: 240, pageLeft: 3, pageTop: 25, scale: 1,
                    })) Object.defineProperty(visualViewport, key, { configurable: true, value });
                    visualViewport.dispatchEvent(new Event('resize'));
                    visualViewport.dispatchEvent(new Event('scroll'));
                });
                await page.waitForFunction(() => document.documentElement.style
                    .getPropertyValue('--viewport-height') === '240px');
                rect = await geometry();
                near(rect.x, 12);
                near(rect.y, 45);
                near(rect.width, 304);
                near(rect.height, 208);
                await page.touchscreen.tap(rect.x + 40, rect.y + 50);
                assert.deepEqual(await page.evaluate(() => window.hits.at(-1)), [40, 50]);
                await page.evaluate(() => {
                    window.dispatchEvent(new PageTransitionEvent('pagehide'));
                    Object.defineProperty(visualViewport, 'height', { configurable: true, value: 210 });
                    visualViewport.dispatchEvent(new Event('resize'));
                });
                // Two animation frames allow any incorrectly retained callback to run.
                await page.evaluate(() => new Promise(resolve => requestAnimationFrame(() => requestAnimationFrame(resolve))));
                near((await geometry()).height, 208);
                await page.evaluate(() => window.dispatchEvent(new PageTransitionEvent('pageshow')));
                near((await geometry()).height, 178);
                await page.evaluate(() => {
                    Object.defineProperty(visualViewport, 'scale', { configurable: true, value: 2 });
                    Object.defineProperty(visualViewport, 'height', { configurable: true, value: 100 });
                    visualViewport.dispatchEvent(new Event('resize'));
                });
                await page.evaluate(() => new Promise(resolve => requestAnimationFrame(() => requestAnimationFrame(resolve))));
                near((await geometry()).height, 178);
                await page.screenshot({ path: join(tmpdir(), `shiju-viewport-${width}-${deviceScaleFactor}.png`) });
                passed++;
            } finally {
                await context.close();
            }
        }
    }
    // Exercise the innerWidth/innerHeight fallback with no visualViewport API.
    const page = await browser.newPage({ viewport: { width: 390, height: 844 } });
    await page.addInitScript(() => Object.defineProperty(window, 'visualViewport', { value: undefined }));
    await page.goto(`http://127.0.0.1:${server.address().port}/`);
    await page.setViewportSize({ width: 844, height: 390 });
    await page.waitForFunction(() => document.documentElement.style
        .getPropertyValue('--viewport-height') === '390px');
    assert.equal(await page.locator('#shiju-viewport').evaluate(node => node.clientWidth), 844);
    passed++;
    console.log(`PASS ${passed} DOM viewport cases; viewport/safe-area/lifecycle events simulated; Iced and physical devices not verified`);
} finally {
    await browser?.close();
    await new Promise((resolve, reject) => server.close(error => error ? reject(error) : resolve()));
}
