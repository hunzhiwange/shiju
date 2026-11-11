// Run with Playwright installed (or NODE_PATH pointing to its node_modules).
import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
import { createServer } from 'node:http';
import { readFile } from 'node:fs/promises';
const require = createRequire(import.meta.url);
const { chromium } = require('playwright');
const source = new URL('../../vendor/iced_winit/src/web_ime.js', import.meta.url);
const server = createServer(async (req, res) => {
    if (req.url === '/web_ime.js') {
        try { res.setHeader('Content-Type', 'text/javascript'); res.end(await readFile(source)); }
        catch { res.writeHead(404); res.end(); }
    } else {
        res.setHeader('Content-Type', 'text/html');
        res.end('<canvas tabindex="0" style="position:absolute;left:40px;top:30px;width:600px;height:400px" width="1200" height="800"></canvas><input id="outside">');
    }
});
await new Promise(resolve => server.listen(0, '127.0.0.1', resolve));
const browser = await chromium.launch({ channel: 'chrome', headless: true });
let passed = 0;
async function test(name, fn) {
    const page = await browser.newPage();
    try {
        await page.goto(`http://127.0.0.1:${server.address().port}`);
        await page.evaluate(async () => {
            const { createImeBridge } = await import('/web_ime.js');
            window.events = [];
            window.canvas = document.querySelector('canvas');
            canvas.focus();
            window.bridge = createImeBridge(canvas, (kind, text) => events.push([kind, text]));
            bridge.update(true, 20, 40, 1, 20, false);
        });
        await fn(page);
        passed++;
        console.log(`PASS ${name}`);
    } finally { await page.close(); }
}
try {
    await test('ordinary input commits once', async page => {
        await page.keyboard.type('hello');
        assert.equal(await page.evaluate(() => events.filter(e => e[0] === 'commit').map(e => e[1]).join('')), 'hello');
    });
    await test('real Chromium IME commits Chinese once', async page => {
        const cdp = await page.context().newCDPSession(page);
        await cdp.send('Input.imeSetComposition', { text: 'ni', selectionStart: 2, selectionEnd: 2 });
        assert.equal(await page.evaluate(() => events.filter(e => e[0] === 'commit').length), 0);
        await cdp.send('Input.insertText', { text: '你好' });
        assert.deepEqual(await page.evaluate(() => events.filter(e => e[0] === 'commit')), [['commit', '你好']]);
    });
    await test('compositionend followed by input does not duplicate', async page => {
        await page.evaluate(() => {
            const target = document.activeElement;
            target.dispatchEvent(new CompositionEvent('compositionstart'));
            target.value = '中文';
            target.dispatchEvent(new CompositionEvent('compositionupdate', { data: '中文' }));
            target.dispatchEvent(new CompositionEvent('compositionend', { data: '中文' }));
            target.value = '中文';
            target.dispatchEvent(new InputEvent('input', { data: '中文', inputType: 'insertCompositionText' }));
        });
        assert.deepEqual(await page.evaluate(() => events.filter(e => e[0] === 'commit')), [['commit', '中文']]);
        await page.keyboard.insertText('中文');
        assert.equal(await page.evaluate(() => events.filter(e => e[0] === 'commit').length), 2);
    });
    await test('cancelled composition does not commit', async page => {
        await page.evaluate(() => {
            const target = document.activeElement;
            target.dispatchEvent(new CompositionEvent('compositionstart'));
            target.value = 'ni';
            target.dispatchEvent(new CompositionEvent('compositionupdate', { data: 'ni' }));
            target.dispatchEvent(new CompositionEvent('compositionend', { data: '' }));
        });
        assert.equal(await page.evaluate(() => events.filter(e => e[0] === 'commit').length), 0);
    });
    await test('navigation forwarded; IME Enter does not submit', async page => {
        await page.evaluate(() => {
            window.keys = [];
            canvas.addEventListener('keydown', e => keys.push(e.key));
        });
        await page.keyboard.press('ArrowLeft');
        await page.keyboard.press('Enter');
        await page.evaluate(() => document.activeElement.dispatchEvent(new KeyboardEvent('keydown', { key: 'Enter', isComposing: true })));
        assert.deepEqual(await page.evaluate(() => keys), ['ArrowLeft', 'Enter']);
    });
    await test('caret uses canvas CSS scale and resize position', async page => {
        const box = await page.evaluate(() => {
            const r = document.activeElement.getBoundingClientRect(); return { x: r.x, y: r.y };
        });
        assert.deepEqual(box, { x: 50, y: 50 });
        await page.evaluate(() => {
            canvas.style.left = '100px';
            bridge.update(true, 20, 40, 1, 20, false);
        });
        assert.equal(await page.evaluate(() => document.activeElement.getBoundingClientRect().x), 110);
    });
    await test('scroll repositions the IME proxy without stealing page focus', async page => {
        await page.evaluate(() => {
            canvas.style.top = '90px';
            document.dispatchEvent(new Event('scroll'));
        });
        assert.equal(await page.evaluate(() => document.activeElement.getBoundingClientRect().y), 110);
    });
    await test('paste preserves multiple lines and Unicode', async page => {
        await page.keyboard.insertText('中文\n🙂');
        assert.deepEqual(await page.evaluate(() => events.filter(e => e[0] === 'commit')), [['commit', '中文\n🙂']]);
    });
    await test('secure input and blur clear temporary text', async page => {
        await page.evaluate(() => bridge.update(true, 0, 0, 1, 20, true));
        assert.equal(await page.evaluate(() => document.activeElement.type), 'password');
        await page.keyboard.insertText('密钥');
        assert.equal(await page.evaluate(() => document.activeElement.value), '');
        await page.locator('#outside').focus();
        assert.equal(await page.evaluate(() => bridge.hasFocus()), false);
        assert.equal(await page.evaluate(() => events.at(-1)[0]), 'blur');
    });
    await test('disabled bridge does not steal focus; dispose removes listeners', async page => {
        await page.evaluate(() => {
            bridge.update(false, 0, 0, 0, 0, false);
            document.querySelector('#outside').focus();
            bridge.update(false, 0, 0, 0, 0, false);
        });
        assert.equal(await page.evaluate(() => document.activeElement.id), 'outside');
        await page.evaluate(() => { bridge.dispose(); events.length = 0; });
        assert.equal(await page.locator('[data-iced-ime]').count(), 0);
        await page.keyboard.type('abc');
        assert.deepEqual(await page.evaluate(() => events), []);
    });
    console.log(`${passed} browser tests passed`);
} finally {
    await browser.close();
    await new Promise(resolve => server.close(resolve));
}
