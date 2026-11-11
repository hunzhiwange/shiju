import test from 'node:test';
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { createController, HOST_NAME } from './background.mjs';

function event() {
  const listeners = new Set();
  return { addListener: f => listeners.add(f), emit: (...args) => [...listeners].map(f => f(...args)) };
}
function fixture() {
  const sent = [], scripts = [], created = [];
  const tab = { id: 7, windowId: 3, active: true, status: 'complete', url: 'https://example.test/' };
  const port = { onMessage: event(), onDisconnect: event(), postMessage: m => sent.push(m), disconnect() { this.onDisconnect.emit(); } };
  const api = {
    runtime: { id: 'extension-id', connectNative: name => { assert.equal(name, HOST_NAME); return port; } },
    action: { onClicked: event(), setBadgeText: async () => {} },
    tabs: {
      onUpdated: event(), onRemoved: event(), onActivated: event(), get: async id => { assert.equal(id, 7); return { ...tab }; },
      create: async args => { created.push(args); return { ...tab, id: 8, url: args.url }; },
      captureVisibleTab: async windowId => { assert.equal(windowId, 3); return 'data:image/png;base64,eA=='; },
    },
    windows: { onFocusChanged: event(), get: async () => ({ focused: true }) },
    scripting: { executeScript: async args => {
      scripts.push(args);
      return [{ documentId: 'document-1', frameId: 0, result: ({ read: { text: 'fixture' }, click: { clicked: true }, input: { entered: true } })[args.args?.[0]] ?? { url: tab.url } }];
    } },
  };
  const controller = createController(api, { nonce: () => 'challenge-1', timeoutMs: 50 });
  async function grant() { await Promise.all(api.action.onClicked.emit({ ...tab })); }
  function pair(sessionId = 'session-1') {
    port.onMessage.emit({ type: 'paired', version: 1, challenge: 'challenge-1', sessionId });
  }
  let seq = 0;
  async function request(action, extra = {}) {
    const id = `request-${++seq}`;
    await Promise.all(port.onMessage.emit({ type: 'request', version: 1, sessionId: 'session-1', id, seq, action, ...extra }));
    return sent.find(m => m.type === 'response' && m.id === (extra.id ?? id));
  }
  async function setup() { await grant(); pair(); return (await request('enumerate')).data[0]; }
  return { api, port, tab, sent, scripts, created, controller, grant, pair, request, setup };
}

test('cc_control_08_manifest_has_no_permanent_site_or_external_access', async () => {
  const manifest = JSON.parse(await readFile(new URL('./manifest.json', import.meta.url)));
  assert.equal(manifest.manifest_version, 3);
  assert.deepEqual(manifest.permissions, ['activeTab', 'scripting', 'nativeMessaging']);
  assert.equal(manifest.host_permissions, undefined);
  assert.equal(manifest.externally_connectable, undefined);
  assert.equal(manifest.background.type, 'module');
});

test('cc_control_08_unpaired_and_wrong_session_requests_are_rejected', async () => {
  const f = fixture();
  await f.grant();
  assert.equal(f.sent[0].type, 'hello');
  assert.equal((await f.request('enumerate')).error, 'unpaired');
  f.pair();
  assert.equal((await f.request('enumerate', { sessionId: 'wrong' })).error, 'wrong_session');
  assert.equal(f.scripts.length, 1);
});

test('cc_control_08_wrong_challenge_cannot_pair', async () => {
  const f = fixture(); await f.grant();
  f.port.onMessage.emit({ type: 'paired', version: 1, challenge: 'wrong', sessionId: 'session-1' });
  assert.equal((await f.request('enumerate')).error, 'unpaired');
});

test('cc_control_08_granted_document_reads_clicks_inputs_and_captures', async () => {
  const f = fixture(), target = await f.setup();
  assert.equal(target.tabId, 7); assert.equal(target.documentId, 'document-1');
  for (const [action, args] of [['read', {}], ['click', { selector: '#button' }], ['input', { selector: '#name', text: '中文' }], ['screenshot', {}]]) {
    const result = await f.request(action, { target, args });
    assert.equal(result.ok, true, JSON.stringify(result));
  }
  assert.deepEqual(f.scripts.at(-1).target, { tabId: 7, documentIds: ['document-1'] });
  assert.equal(f.scripts.at(-1).world, 'ISOLATED');
  assert.equal(f.scripts.at(-1).args[0], 'input');
});

test('cc_control_08_closed_tab_is_not_retargeted', async () => {
  const f = fixture(), target = await f.setup(), count = f.scripts.length;
  f.api.tabs.onRemoved.emit(7);
  assert.equal((await f.request('click', { target, args: { selector: '#button' } })).error, 'stale_target');
  assert.equal(f.scripts.length, count);
});

test('cc_control_08_navigation_invalidates_handle_even_if_url_returns', async () => {
  const f = fixture(), target = await f.setup();
  f.api.tabs.onUpdated.emit(7, { status: 'loading' });
  await f.grant();
  assert.equal((await f.request('read', { target })).error, 'stale_target');
  const fresh = (await f.request('enumerate')).data[0];
  assert.ok(fresh.navigationVersion > target.navigationVersion);
});

test('cc_control_08_requests_correlate_and_duplicate_sequence_is_rejected', async () => {
  const f = fixture(); await f.setup();
  const request = { type: 'request', version: 1, sessionId: 'session-1', seq: 2, id: 'unique', action: 'enumerate' };
  await Promise.all(f.port.onMessage.emit(request));
  await Promise.all(f.port.onMessage.emit(request));
  const results = f.sent.filter(m => m.id === 'unique');
  assert.equal(results[0].ok, true); assert.equal(results[1].error, 'invalid_sequence');
});

test('cc_control_08_unknown_actions_and_ungranted_targets_are_denied', async () => {
  const f = fixture(), target = await f.setup();
  assert.equal((await f.request('executeScript', { args: { code: 'alert(1)' } })).error, 'unsupported_action');
  assert.equal((await f.request('read', { target: { ...target, tabId: 9 } })).error, 'stale_target');
  assert.equal((await f.request('input', { target, args: { selector: '#file', text: 'x', code: 'x' } })).error, 'invalid_arguments');
});

test('cc_control_08_open_is_http_only_and_new_tab_requires_user_grant', async () => {
  const f = fixture(); await f.setup();
  assert.equal((await f.request('open', { args: { url: 'file:///etc/passwd' } })).error, 'invalid_url');
  const opened = await f.request('open', { args: { url: 'https://example.test/new' } });
  assert.equal(opened.data.tabId, 8); assert.equal(opened.data.requiresUserGrant, true);
  assert.deepEqual(f.created, [{ url: 'https://example.test/new', active: true }]);
  assert.equal((await f.request('enumerate')).data.length, 1);
});

test('cc_control_08_inactive_screenshot_never_captures_other_tab', async () => {
  const f = fixture(), target = await f.setup();
  f.tab.active = false;
  f.api.tabs.captureVisibleTab = () => assert.fail('wrong tab capture');
  assert.equal((await f.request('screenshot', { target })).error, 'target_not_visible');
});

test('cc_control_08_navigation_ends_pending_request_and_discards_late_result', async () => {
  const f = fixture(), target = await f.setup();
  let finish;
  f.api.scripting.executeScript = () => new Promise(resolve => { finish = resolve; });
  const pending = f.request('read', { target });
  await new Promise(resolve => setImmediate(resolve));
  f.api.tabs.onUpdated.emit(7, { url: 'https://other.test/' });
  assert.equal((await pending).error, 'stale_target');
  finish([{ documentId: 'document-1', result: { text: 'late' } }]);
});

test('cc_control_08_disconnect_clears_grants_and_drops_inflight_reply', async () => {
  const f = fixture(), target = await f.setup();
  let finish;
  f.api.scripting.executeScript = () => new Promise(resolve => { finish = resolve; });
  const pending = f.request('read', { target });
  await new Promise(resolve => setImmediate(resolve));
  f.port.disconnect();
  assert.equal(await pending, undefined);
  finish([{ documentId: 'document-1', result: { text: 'late' } }]);
  assert.equal(f.sent.filter(m => m.type === 'response').length, 1);
});

test('cc_control_08_hung_boundary_times_out_and_requires_new_pairing', async () => {
  const f = fixture(), target = await f.setup();
  f.api.scripting.executeScript = () => new Promise(() => {});
  assert.equal((await f.request('read', { target })).error, 'timeout');
  assert.equal((await f.request('enumerate')), undefined);
});


test('cc_control_08_duplicate_id_cannot_ambiguously_correlate_responses', async () => {
  const f = fixture(); await f.setup();
  assert.equal((await f.request('enumerate', { id: 'same' })).ok, true);
  await f.request('enumerate', { id: 'same' });
  const responses = f.sent.filter(m => m.id === 'same');
  assert.equal(responses[1].error, 'duplicate_id');
});

test('cc_control_08_switch_away_and_back_discards_captured_pixels', async () => {
  const f = fixture(), target = await f.setup();
  let finish;
  f.api.tabs.captureVisibleTab = () => new Promise(resolve => { finish = resolve; });
  const pending = f.request('screenshot', { target });
  await new Promise(resolve => setImmediate(resolve));
  assert.equal(typeof finish, 'function');
  f.api.tabs.onActivated.emit({ tabId: 8, windowId: 3 });
  f.api.tabs.onActivated.emit({ tabId: 7, windowId: 3 });
  const result = await pending;
  assert.equal(result.error, 'target_not_visible');
  assert.equal(result.data, undefined);
  finish('data:image/png;base64,eA==');
});

test('cc_control_08_navigation_during_grant_never_authorizes_new_document', async () => {
  const f = fixture();
  let finish;
  f.api.scripting.executeScript = () => new Promise(resolve => { finish = resolve; });
  const pending = f.grant();
  f.api.tabs.onUpdated.emit(7, { status: 'loading' });
  finish([{ documentId: 'document-1', result: { url: f.tab.url } }]);
  await pending;
  assert.equal(f.sent.length, 0);
});

test('cc_control_08_external_errors_do_not_expose_page_payloads', async () => {
  const f = fixture(), target = await f.setup();
  f.api.scripting.executeScript = async () => { throw new Error('secret page payload'); };
  const result = await f.request('read', { target });
  assert.equal(result.error, 'chrome_api_error');
  assert.ok(!JSON.stringify(result).includes('secret'));
});


test('cc_control_08_page_refusal_cannot_be_reported_as_success', async () => {
  const f = fixture(), target = await f.setup();
  f.api.scripting.executeScript = async () => [{ documentId: target.documentId, frameId: 0 }];
  assert.equal((await f.request('click', { target, args: { selector: '#password' } })).error, 'chrome_api_error');
});
