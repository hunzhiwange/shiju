// Native Messaging v1: only this extension opens com.shiju.computer_control.
// A toolbar click grants one current top-level document and opens the native port.
// hello {version, challenge, extensionId} -> paired {version, challenge, sessionId}.
// request {version, sessionId, id, seq, action, target?, args?}; seq strictly increases.
// response {version, sessionId, id, ok, data?|error?}. Handles are connection-local.
// Native host registration / authenticated ControlHost binding belongs to task 9.
export const HOST_NAME = 'com.shiju.computer_control';
const VERSION = 1;
const ACTIONS = new Set(['enumerate', 'open', 'read', 'click', 'input', 'screenshot', 'navigate', 'upload', 'download']);
const fail = code => { throw new Error(code); };
const token = value => typeof value === 'string' && value.length > 0 && value.length <= 128;

function httpUrl(value) {
  if (typeof value !== 'string' || value.length > 8192) fail('invalid_url');
  let url;
  try { url = new URL(value); } catch { fail('invalid_url'); }
  if (!['http:', 'https:'].includes(url.protocol) || url.username || url.password) fail('invalid_url');
  return url.href;
}

// Serialized by Chrome into the isolated world; never accepts source code.
// All selectors address the top-level document only. Sensitive/file inputs fail closed.
async function pageAction(action, args, expectedUrl) {
  if (location.href !== expectedUrl) throw new Error('stale_target');
  if (action === 'identify') return { url: location.href };
  if (action === 'read') return { text: (document.body?.innerText ?? '').slice(0, 64000) };
  const element = document.querySelector(args.selector);
  if (!(element instanceof HTMLElement) || !element.isConnected || !element.getClientRects().length ||
      element.closest('[inert]') || element.matches(':disabled')) throw new Error('invalid_element');
  if (action === 'upload') {
    if (!(element instanceof HTMLInputElement) || element.type !== 'file' || args.approved !== true ||
        !/^[A-Za-z0-9][A-Za-z0-9._-]{0,127}$/.test(args.fileName) || args.contentBase64.length > 349528) throw new Error('restricted_element');
    const bytes = Uint8Array.from(atob(args.contentBase64), c => c.charCodeAt(0));
    if (bytes.length > 262144) throw new Error('transfer_too_large');
    const transfer = new DataTransfer();
    transfer.items.add(new File([bytes], args.fileName, {type:'application/octet-stream'}));
    element.files = transfer.files;
    element.dispatchEvent(new Event('input', {bubbles:true}));
    element.dispatchEvent(new Event('change', {bubbles:true}));
    return {uploaded:true};
  }
  if (action === 'download') {
    if (!(element instanceof HTMLAnchorElement) || args.approved !== true) throw new Error('restricted_element');
    const url = new URL(element.href);
    if (!['http:', 'https:'].includes(url.protocol) || url.origin !== location.origin || url.href !== args.url || url.username || url.password) throw new Error('restricted_element');
    const response = await fetch(url.href, {redirect:'error',credentials:'same-origin',signal:AbortSignal.timeout(5000)});
    if (!response.ok || !response.body) throw new Error('download_failed');
    const reader=response.body.getReader();
    const chunks=[];let size=0;
    try {
      while (true) {
        const {done,value}=await reader.read();if(done)break;
        size+=value.length;if(size>262144)throw new Error('transfer_too_large');chunks.push(value);
      }
    } finally { await reader.cancel(); }
    const bytes=new Uint8Array(size);let offset=0;
    for(const chunk of chunks){bytes.set(chunk,offset);offset+=chunk.length;}
    let binary='';for(const byte of bytes)binary+=String.fromCharCode(byte);
    return {contentBase64:btoa(binary)};
  }
  if (action === 'click') {
    if (element.closest('input[type=file],input[type=password],a[download]') || element.closest('form')?.querySelector('input[type=file]')) throw new Error('restricted_element');
    element.click();
    return { clicked: true };
  }
  if (action === 'input') {
    let prototype;
    if (element instanceof HTMLTextAreaElement) prototype = HTMLTextAreaElement.prototype;
    else if (element instanceof HTMLInputElement && ['text', 'search', 'email', 'url', 'tel'].includes(element.type)) {
      prototype = HTMLInputElement.prototype;
    } else throw new Error('restricted_element');
    if (element.readOnly) throw new Error('restricted_element');
    element.focus();
    Object.getOwnPropertyDescriptor(prototype, 'value').set.call(element, args.text);
    element.dispatchEvent(new Event('input', { bubbles: true }));
    element.dispatchEvent(new Event('change', { bubbles: true }));
    return { entered: true };
  }
  throw new Error('unsupported_action');
}

export function createController(api, { nonce = () => crypto.randomUUID(), timeoutMs = 15000 } = {}) {
  const grants = new Map(), versions = new Map(), pending = new Map(), usedIds = new Set();
  let port, challenge, sessionId, sequence = 0, epoch = 0;
  let pairingTimer;

  function invalidate(tabId) {
    versions.set(tabId, (versions.get(tabId) ?? 0) + 1);
    grants.delete(tabId);
    for (const entry of pending.values()) if (entry.tabId === tabId && !entry.navigation) entry.reject(new Error('stale_target'));
  }
  function clear() {
    epoch++;
    clearTimeout(pairingTimer);
    port = undefined; sessionId = undefined; sequence = 0;
    grants.clear(); versions.clear(); usedIds.clear();
    for (const entry of pending.values()) entry.reject(new Error('disconnected'));
    pending.clear();
  }
  function disconnect() {
    const old = port;
    clear();
    old?.disconnect();
  }
  function connect() {
    if (port) return;
    challenge = nonce();
    const current = api.runtime.connectNative(HOST_NAME);
    port = current;
    current.onDisconnect.addListener(() => {
      // Consume lastError so Chrome reports no unchecked runtime error; never log payloads.
      void api.runtime.lastError;
      if (port === current) clear();
    });
    current.onMessage.addListener(message => receive(current, message));
    pairingTimer = setTimeout(() => { if (port === current && !sessionId) disconnect(); }, timeoutMs);
    current.postMessage({ type: 'hello', version: VERSION, challenge, extensionId: api.runtime.id });
  }
  async function grant(tab) {
    try {
      const url = httpUrl(tab.url), grantEpoch = epoch;
      const navigationVersion = versions.get(tab.id) ?? 0;
      versions.set(tab.id, navigationVersion);
      const results = await api.scripting.executeScript({
        target: { tabId: tab.id, frameIds: [0] }, world: 'ISOLATED',
        func: pageAction, args: ['identify', {}, url],
      });
      const result = results[0];
      if (grantEpoch !== epoch || navigationVersion !== (versions.get(tab.id) ?? 0) ||
          results.length !== 1 || result?.frameId !== 0 || !result?.documentId || result.result?.url !== url) fail('stale_target');
      connect();
      grants.set(tab.id, { tabId: tab.id, documentId: result.documentId, navigationVersion, url });
      await api.action.setBadgeText({ tabId: tab.id, text: 'ON' });
    } catch {
      // Failure is visible at the action; never fall back to a broader tab or site grant.
      await api.action.setBadgeText({ tabId: tab.id, text: '!' });
    }
  }
  function bound(target) {
    const known = grants.get(target?.tabId);
    if (!known || target.documentId !== known.documentId || target.navigationVersion !== known.navigationVersion ||
        target.url !== known.url) fail('stale_target');
    return known;
  }
  async function currentTab(target) {
    bound(target);
    let tab;
    try { tab = await api.tabs.get(target.tabId); } catch { invalidate(target.tabId); fail('stale_target'); }
    bound(target);
    if (tab.url !== target.url || tab.status === 'loading') { invalidate(target.tabId); fail('stale_target'); }
    return tab;
  }
  function argsFor(action, args = {}) {
    if (!args || typeof args !== 'object' || Array.isArray(args)) fail('invalid_arguments');
    if (['upload','download'].includes(action) && args.approved !== true) fail('transfer_not_approved');
    const keys = ['open','navigate'].includes(action) ? ['url'] : action === 'upload' ? ['selector','fileName','contentBase64','approved'] : action === 'download' ? ['selector','url','approved'] : action === 'input' ? ['selector', 'text'] : action === 'click' ? ['selector'] : [];
    if (Object.keys(args).some(key => !keys.includes(key))) fail('invalid_arguments');
    if (keys.includes('selector') && (typeof args.selector !== 'string' || !args.selector || args.selector.length > 2048)) fail('invalid_arguments');
    if (action === 'input' && (typeof args.text !== 'string' || args.text.length > 16000)) fail('invalid_arguments');
    if (action === 'upload' && (typeof args.fileName !== 'string' || !/^[A-Za-z0-9][A-Za-z0-9._-]{0,127}$/.test(args.fileName) || typeof args.contentBase64 !== 'string' || args.contentBase64.length > 349528)) fail('invalid_arguments');
    return args;
  }
  async function execute(message, requestEpoch) {
    const { action, target } = message, args = argsFor(action, message.args);
    if (action === 'enumerate') return [...grants.values()].map(target => ({ ...target }));
    if (action === 'open') {
      const tab = await api.tabs.create({ url: httpUrl(args.url), active: true });
      return { tabId: tab.id, requiresUserGrant: true };
    }
    const tab = await currentTab(target);
    if (requestEpoch !== epoch) fail('disconnected');
    if (action === 'navigate') {
      const url=httpUrl(args.url);
      invalidate(target.tabId);
      await api.tabs.update(target.tabId,{url});
      return {navigated:true};
    }
    if (action === 'screenshot') {
      if (!tab.active || !(await api.windows.get(tab.windowId)).focused) fail('target_not_visible');
      bound(target);
      if (requestEpoch !== epoch) fail('disconnected');
      const dataUrl = await api.tabs.captureVisibleTab(tab.windowId, { format: 'png' });
      const after = await currentTab(target);
      if (!after.active || !(await api.windows.get(tab.windowId)).focused) fail('target_not_visible');
      if (!dataUrl.startsWith('data:image/png;base64,') || dataUrl.length > 900000) fail('screenshot_too_large');
      return { dataUrl };
    }
    const results = await api.scripting.executeScript({
      target: { tabId: target.tabId, documentIds: [target.documentId] },
      world: 'ISOLATED', func: pageAction, args: [action, args, target.url],
    });
    bound(target);
    if (results.length !== 1 || results[0].documentId !== target.documentId || results[0].frameId !== 0 || results[0].error) fail('stale_target');
    const result = results[0].result;
    if (!result || (action === 'read' && typeof result.text !== 'string') ||
        (action === 'click' && result.clicked !== true) || (action === 'input' && result.entered !== true) || (action === 'upload' && result.uploaded !== true) || (action === 'download' && (typeof result.contentBase64 !== 'string' || result.contentBase64.length > 349528))) fail('chrome_api_error');
    return result;
  }
  async function receive(current, message) {
    if (port !== current || !message || typeof message !== 'object') return;
    if (message.type === 'paired') {
      if (!sessionId && message.version === VERSION && message.challenge === challenge && token(message.sessionId)) {
        sessionId = message.sessionId; clearTimeout(pairingTimer);
      }
      return;
    }
    if (message.type !== 'request' || !token(message.id)) return;
    const requestEpoch = epoch, requestSession = sessionId;
    let timer, entryKey, timedOut = false;
    const response = { type: 'response', version: VERSION, sessionId: requestSession ?? null, id: message.id };
    try {
      if (!sessionId) fail('unpaired');
      if (message.version !== VERSION) fail('wrong_version');
      if (message.sessionId !== sessionId) fail('wrong_session');
      if (!Number.isSafeInteger(message.seq) || message.seq <= sequence) fail('invalid_sequence');
      sequence = message.seq;
      if (usedIds.has(message.id)) fail('duplicate_id');
      if (usedIds.size >= 4096) { disconnect(); return; }
      usedIds.add(message.id);
      if (!ACTIONS.has(message.action)) fail('unsupported_action');
      if (pending.size >= 16 || (message.target && [...pending.values()].some(entry => entry.tabId === message.target.tabId))) fail('busy');
      if (message.target) bound(message.target);
      entryKey = message.seq;
      const ended = new Promise((_, reject) => {
        pending.set(entryKey, { tabId: message.target?.tabId, screenshot: message.action === 'screenshot', navigation: message.action === 'navigate', reject });
        timer = setTimeout(() => { timedOut = true; reject(new Error('timeout')); }, timeoutMs);
      });
      const data = await Promise.race([execute(message, requestEpoch), ended]);
      response.ok = true; response.data = data;
    } catch (error) {
      response.ok = false;
      // Chrome / page exceptions can contain sensitive URLs or page text.
      const errors = new Set(['unpaired', 'wrong_version', 'wrong_session', 'invalid_sequence', 'unsupported_action',
        'transfer_not_approved', 'duplicate_id', 'busy', 'stale_target', 'invalid_arguments', 'invalid_url', 'disconnected', 'timeout', 'target_not_visible', 'screenshot_too_large']);
      response.error = errors.has(error.message) ? error.message : 'chrome_api_error';
    } finally {
      clearTimeout(timer);
      if (entryKey !== undefined) pending.delete(entryKey);
    }
    if (port === current && epoch === requestEpoch) {
      try { current.postMessage(response); } catch { disconnect(); }
      // A timed-out write may still finish at the API boundary. Drop all handles;
      // never retry it or accept another operation on this connection.
      if (timedOut) disconnect();
    }
  }
  api.action.onClicked.addListener(grant);
  api.tabs.onRemoved.addListener(invalidate);
  api.tabs.onUpdated.addListener((tabId, change) => {
    if (versions.has(tabId) && (change.status === 'loading' || change.url !== undefined)) {
      invalidate(tabId);
      void api.action.setBadgeText({ tabId, text: '' }).catch(() => {});
    }
  });
  // captureVisibleTab has no tabId argument. Any activation/focus transition during
  // capture cancels delivery, including switching away and back to the same tab.
  function visibilityChanged() {
    for (const entry of pending.values()) if (entry.screenshot) entry.reject(new Error('target_not_visible'));
  }
  api.tabs.onActivated.addListener(visibilityChanged);
  api.windows.onFocusChanged.addListener(visibilityChanged);
  return { disconnect };
}

if (globalThis.chrome?.runtime?.id) createController(globalThis.chrome);
