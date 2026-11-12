// Exercise real Iced canvas controls against a freshly built complete WASM app.
// Only gateway HTTP boundaries are mocked; Rust clocks and ID generation are real.
import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
import { mkdir, writeFile } from 'node:fs/promises';
const require = createRequire(import.meta.url);
const { chromium } = require('playwright');
const url = process.env.SHIJU_WASM_URL || 'http://127.0.0.1:8765/';
const origin = new URL(url).origin;
const artifacts = process.env.SHIJU_WASM_ARTIFACT_DIR || '/tmp/shiju-wasm-workflow-time';
await mkdir(artifacts, { recursive: true });
const browser = await chromium.launch({
    channel: 'chrome', headless: true,
    args: ['--enable-unsafe-webgpu', '--use-angle=swiftshader'],
});
const errors = [];
const saves = [];
const requests = [];
let page;
try {
    page = await browser.newPage({ viewport: { width: 1440, height: 1000 } });
    await page.addInitScript(() => { Error.stackTraceLimit = 50; });
    page.on('pageerror', error => { errors.push(String(error)); console.error(String(error)); });
    page.on('console', message => {
        if (/panicked at|RuntimeError|unreachable executed/.test(message.text())) {
            errors.push(message.text());
            console.error(message.text());
        }
    });
    const project = {
        id: 'workflow-time-project', name: 'Workflow time fixture',
        directory: '/workflow-time', display_path: '/workflow-time', status: 'ready',
        created_at_ms: 1, updated_at_ms: 1,
        git: { is_repo: false, has_uncommitted_changes: false },
    };
    await page.route('**/*', async route => {
        const request = route.request();
        const target = new URL(request.url());
        if (target.origin === origin && !target.pathname.startsWith('/v1/')) {
            return route.continue();
        }
        requests.push({ path: target.pathname, method: request.method() });
        let data = { items: [] };
        if (target.pathname === '/v1/projects') data = { items: [project] };
        if (target.pathname.startsWith('/v1/projects/')) data = { project };
        if (target.pathname.startsWith('/v1/workflow/applications')) {
            data = [];
            if (['POST', 'PUT'].includes(request.method())) {
                const body = request.postDataJSON();
                saves.push(body);
                data = { ...body, uuid: 'workflow-time-saved', created_at_ms: 1, updated_at_ms: 1 };
            }
        }
        await route.fulfill({
            contentType: 'application/json', headers: { 'access-control-allow-origin': '*' },
            body: JSON.stringify(data),
        });
    });
    const settle = () => page.waitForTimeout(300);
    const click = async (x, y) => { await page.mouse.click(x, y); await settle(); };
    const fill = async (x, y, text) => {
        await click(x, y);
        await page.keyboard.press('Control+A');
        await page.keyboard.type(text);
        await settle();
    };
    const healthy = label => assert.deepEqual(errors, [], `${label}: no Rust or rendering panic`);
    const save = async label => {
        const count = saves.length;
        await click(35, 98); // Workflow action menu.
        await click(100, 282); // Save YAML through the gateway; no local file picker.
        healthy(label);
        assert.equal(saves.length, count + 1, `${label}: exactly one gateway save`);
        assert.equal(saves.at(-1).name, 'Workflow time regression');
        await page.screenshot({ path: `${artifacts}/${label}.png` });
        return saves.at(-1).workflow_yaml;
    };
    const expectId = (yaml, pattern, label) => {
        const values = yaml.match(pattern);
        assert.ok(values?.length, `${label} appears in the serialized Rust workflow`);
        for (const value of values) {
            const timestamp = Number(value.match(/\d+$/)[0]);
            assert.ok(timestamp >= started && timestamp <= Date.now(), `${label}: current Unix milliseconds`);
        }
        console.log(`PASS ${label}: ${[...new Set(values)].join(', ')}`);
    };

    await page.goto(url);
    await page.waitForSelector('canvas', { timeout: Number(process.env.SHIJU_WASM_TIMEOUT || 120000) });
    await page.waitForTimeout(2500);
    // Iced uses one canvas rather than DOM form controls. Coordinates are fixed to
    // the viewport above and were inspected along the actual UI route.
    await click(1375, 425); // Open the fixture's recent-project card.
    await click(68, 311); // More.
    await click(380, 420); // Workflow.
    assert.ok(requests.some(request => request.path === '/v1/workflow/applications'),
        'The real workflow list path reaches the mocked gateway');
    await click(720, 570); // Create application in the empty list.
    await fill(650, 366, 'Workflow time regression');
    healthy('before application submission');
    const started = await page.evaluate(() => Date.now());
    await page.screenshot({ path: `${artifacts}/before-create.png` });
    await click(974, 785); // Submit metadata -> apply_loaded -> generate_app_id.
    healthy('application creation');

    // The runtime application ID is intentionally absent from persisted YAML.
    // Read the real WASM export without changing the application or its clock.
    const appIds = await page.evaluate(async since => {
        const module = await import('./shi_ju.js');
        const wasm = await module.default(); // Already initialized: returns the same exports.
        const bytes = new Uint8Array(wasm.memory.buffer);
        const ids = new Set();
        for (let start = 0; start < bytes.length; start += 1048576) {
            const chunk = new TextDecoder().decode(bytes.subarray(start, start + 1048576 + 64));
            for (const match of chunk.matchAll(/workflow-app-\d+/g)) {
                const timestamp = Number(match[0].slice('workflow-app-'.length));
                if (timestamp >= since && timestamp <= Date.now()) ids.add(match[0]);
            }
        }
        return [...ids];
    }, started);
    assert.ok(appIds.length, 'generate_app_id uses a numeric current Unix millisecond suffix');
    console.log(`PASS application: ${appIds.join(', ')}`);
    await save('application');

    await click(654, 98); // Environment variables.
    await click(670, 490); // Add environment variable.
    await fill(180, 204, 'time_env');
    await click(180, 442);
    await page.keyboard.type('fixture-value');
    await click(682, 615); // Submit -> generate_variable_id.
    await click(754, 300); // Close the now populated variable panel.
    expectId(await save('environment'), /env-var-\d+/g, 'variable');

    await click(100, 360); // Select the existing start node.
    await click(773, 98); // Edit selected node.
    await click(1381, 193); // Add input -> generate_start_variable_name.
    await page.screenshot({ path: `${artifacts}/input-draft.png` });
    await click(498, 695); // Keep the generated input name and save the input.
    await click(1358, 101); // Save node.
    expectId(await save('input'), /input_\d+/g, 'input');

    await click(615, 98); // Insert node palette.
    await click(739, 333); // If/else -> generate_node_id.
    await click(773, 98); // Edit inserted node.
    await click(1073, 148); // Advanced DSL: remove static default cases.
    await fill(1100, 240, 'type: if-else\ntitle: Time branch\ncases: []\n');
    await click(904, 148); // Return to visual editing.
    await click(1370, 199); // Add branch -> generate_case_id and generate_condition_id.
    await fill(1100, 537, 'sys.query');
    await fill(1100, 611, 'fixture');
    await page.screenshot({ path: `${artifacts}/branch-draft.png` });
    await click(1358, 101);
    const branchYaml = await save('branch');
    expectId(branchYaml, /if_else-node-\d+/g, 'node');
    expectId(branchYaml, /case-\d+/g, 'case');
    expectId(branchYaml, /condition-\d+/g, 'condition');

    await page.mouse.move(220, 364); // Real source handle on start.
    await page.mouse.down();
    await page.mouse.move(805, 710, { steps: 20 }); // Real target handle on if/else.
    await page.mouse.up();
    await settle();
    expectId(await save('edge'), /manual-start-node-source-if_else-node-\d+-target-\d+/g, 'edge');

    await click(615, 98);
    await click(590, 237); // Insert LLM.
    await click(773, 98);
    await fill(1010, 224, 'fixture-provider');
    await fill(1280, 224, 'fixture-model');
    await page.mouse.move(1260, 800);
    await page.mouse.wheel(0, 620);
    await settle();
    await click(1100, 510);
    await page.keyboard.type('Fixture system prompt');
    await click(1100, 730);
    await page.keyboard.type('Fixture user prompt');
    await click(1358, 101); // Sync prompt YAML -> generate_prompt_item_id.
    const finalYaml = await save('prompt');
    expectId(finalYaml, /system-prompt-\d+/g, 'system prompt');
    expectId(finalYaml, /user-prompt-\d+/g, 'user prompt');
    assert.ok(finalYaml.includes('Fixture system prompt') && finalYaml.includes('Fixture user prompt'));
    assert.ok(finalYaml.includes('time_env') && finalYaml.includes('sys'));
    assert.equal(saves.length, 6, 'Each edited state persists through the real gateway save path');
    assert.ok(!requests.some(request => /chat\/stream|chat-messages|diagnostics|oauth/.test(request.path)),
        'No model, diagnostic or authorization service is invoked');
    healthy('all eight generation paths');
    await writeFile(`${artifacts}/results.json`, JSON.stringify({ appIds, saves, requests, errors }, null, 2));
    console.log('PASS 8/8 workflow ID generators through real browser UI; six gateway saves; no panic');
} catch (error) {
    if (page) await page.screenshot({ path: `${artifacts}/failed.png` }).catch(() => {});
    await writeFile(`${artifacts}/failure.json`, JSON.stringify({ error: String(error), errors, saves, requests }, null, 2));
    throw error;
} finally {
    await browser.close();
}
