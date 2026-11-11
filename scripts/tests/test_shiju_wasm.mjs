// Smoke-test the complete ShiJu WASM application served by the local developer.
import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
const require = createRequire(import.meta.url);
const { chromium } = require('playwright');
const url = process.env.SHIJU_WASM_URL || 'http://127.0.0.1:8765/';
const browser = await chromium.launch({
    channel: 'chrome',
    headless: true,
    args: ['--enable-unsafe-webgpu', '--use-angle=swiftshader'],
});
try {
    const page = await browser.newPage({ viewport: { width: 1440, height: 900 } });
    const errors = [];
    page.on('pageerror', error => {
        errors.push(String(error));
        console.error(String(error));
    });
    page.on('console', message => {
        if (/panicked at|RuntimeError|unreachable executed/.test(message.text())) {
            errors.push(message.text());
            console.error(message.text());
        }
    });
    await page.goto(url);
    try {
        await page.waitForSelector('canvas', {
            timeout: Number(process.env.SHIJU_WASM_TIMEOUT || 120000),
        });
    } catch (error) {
        console.error(await page.evaluate(() => ({
            startupError: window.shijuInitError,
            canvases: [...document.querySelectorAll('canvas')].map(canvas => ({
                parent: canvas.parentElement.id,
                width: canvas.width,
                height: canvas.height,
            })),
        })));
        console.error(errors);
        await page.screenshot({ path: '/tmp/shiju-wasm-smoke-failed.png' });
        throw error;
    }
    await page.waitForFunction(() => {
        const canvas = document.querySelector('canvas');
        return canvas.width > 0 && canvas.height > 0;
    });
    await page.waitForTimeout(1500);
    assert.equal(await page.title(), '识矩');
    assert.equal(await page.evaluate(() => window.shijuInitError), undefined);
    assert.deepEqual(errors, [], 'No application startup or text-shaping panic');
    await page.screenshot({ path: '/tmp/shiju-wasm-smoke.png' });
    console.log('PASS complete ShiJu application rendered without a panic');
    const beforeAccount = await page.screenshot();
    await page.mouse.click(55, 885);
    await page.waitForTimeout(500);
    const accountMenu = await page.screenshot({ path: '/tmp/shiju-wasm-account-menu.png' });
    assert.notDeepEqual(beforeAccount, accountMenu, 'Bottom account entry opens its menu');
    assert.deepEqual(errors, [], 'Chinese account menu renders without a text-shaping panic');
    await page.mouse.click(700, 450);
    await page.waitForTimeout(300);
    console.log('PASS bottom account menu opens without a panic');
    await page.mouse.move(158, 885);
    await page.waitForTimeout(600);
    await page.screenshot({ path: '/tmp/shiju-wasm-quota-signed-out.png' });
    assert.deepEqual(errors, [], 'Monthly quota label and signed-out tooltip render without a panic');
    await page.mouse.move(700, 450);
    await page.waitForTimeout(200);
    const notificationRegion = { x: 1040, y: 600, width: 380, height: 260 };
    const beforeNotifications = await page.screenshot({ clip: notificationRegion });
    await page.mouse.click(1416, 885);
    await page.waitForTimeout(400);
    await page.screenshot({ path: '/tmp/shiju-wasm-notifications.png' });
    const notifications = await page.screenshot({ clip: notificationRegion });
    assert.notDeepEqual(beforeNotifications, notifications, 'Bottom-right notification button opens the panel');
    assert.deepEqual(errors, [], 'Notification panel renders without a panic');
    await page.mouse.click(1416, 885);
    await page.waitForTimeout(200);
    assert.deepEqual(await page.screenshot({ clip: notificationRegion }), beforeNotifications,
        'Closing notifications restores the content beneath the panel');
    console.log('PASS bottom-right notification panel opens and closes');
    await page.mouse.click(1421, 13);
    await page.waitForTimeout(800);
    await page.screenshot({ path: '/tmp/shiju-wasm-settings.png' });
    assert.deepEqual(errors, [], 'Settings render without a panic');
    console.log('PASS settings opened without a panic');
    await page.mouse.click(100, 90);
    await page.waitForFunction(() => document.activeElement?.hasAttribute('data-iced-ime'));
    const cdp = await page.context().newCDPSession(page);
    await cdp.send('Input.imeSetComposition', {
        text: 'moxing', selectionStart: 6, selectionEnd: 6,
    });
    await cdp.send('Input.insertText', { text: '模型' });
    await page.waitForTimeout(400);
    const chinese = await page.screenshot({ path: '/tmp/shiju-wasm-chinese.png' });
    await page.keyboard.press('Control+A');
    await cdp.send('Input.insertText', { text: '网关' });
    await page.waitForTimeout(400);
    const replaced = await page.screenshot({ path: '/tmp/shiju-wasm-replaced.png' });
    assert.notDeepEqual(chinese, replaced, 'Replacing the search changes the rendered results');
    assert.deepEqual(errors, [], 'Chinese search and replacement do not panic');
    await cdp.detach();
    console.log('PASS Chinese IME search and selection replacement rendered');
} finally {
    await browser.close();
}
