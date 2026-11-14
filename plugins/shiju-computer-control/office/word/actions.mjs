import { connectOffice } from '../shared/connection.mjs';

const fail = message => { throw new Error(message); };
const bounded = value => {
  if (new TextEncoder().encode(JSON.stringify(value)).length > 48 * 1024) fail('Word result exceeds size limit');
  return value;
};
const textValue = (value, empty = false) => {
  if (typeof value !== 'string' || (!empty && !value.length) || value.length > 8192 || /[\r\n\u0000-\u0008\u000b\u000c\u000e-\u001f]/.test(value)) fail('Expected bounded single-paragraph text');
  return value;
};
export function wordCapabilities(Office) {
  if (!Office?.context?.requirements?.isSetSupported?.('WordApi', '1.3')) return [];
  return ['word_read_document', 'word_find_text', ...(Office.context.document?.mode === 'readWrite' ? ['word_replace_text', 'word_insert_text'] : [])];
}
function fileUrl(Office) {
  return new Promise((resolve, reject) => {
    const timer = setTimeout(() => reject(new Error('Document identity timed out')), 3000);
    try {
      Office.context.document.getFilePropertiesAsync(result => {
        clearTimeout(timer);
        if (result.status !== 'succeeded' || typeof result.value?.url !== 'string') reject(new Error('Document identity unavailable'));
        else resolve(result.value.url);
      });
    } catch { clearTimeout(timer); reject(new Error('Document identity unavailable')); }
  });
}

export function createWordActions({ Office, Word, approved }) {
  const policy = structuredClone(approved);
  textValue(policy?.paragraph);
  if (typeof policy.allowWrite !== 'boolean' || ![null, true, false].includes(policy.bold)) fail('Invalid paragraph approval');
  let initialized = false, closed = false, busy = false, url, snapshot, revision = 0;
  // A tracked paragraph range survives edits without reusing mutable character offsets.
  let target;
  const close = () => { closed = true; snapshot = undefined; };
  async function checkIdentity() {
    if (!initialized || closed) fail('Document session closed');
    try {
      if (await fileUrl(Office) !== url || closed) fail('Document changed; pair again');
    } catch (error) { close(); throw error; }
  }
  return {
    close, checkIdentity,
    async initialize() {
      if (initialized || closed || !wordCapabilities(Office).length || typeof Word?.run !== 'function') fail('WordApi 1.3 is required');
      url = await fileUrl(Office);
      if (closed) fail('Document session closed');
      initialized = true;
    },
    async execute(action) {
      if (busy) fail('Word document busy');
      busy = true;
      try {
        await checkIdentity();
        const command = action?.command;
        const fields = { read_document: ['kind'], find_text: ['kind', 'text'], replace_text: ['kind', 'search', 'replacement'], insert_text: ['kind', 'position', 'text'] }[command?.kind];
        if (action?.action !== 'word' || !fields || Object.keys(action).some(k => !['action', 'command'].includes(k)) || Object.keys(command).some(k => !fields.includes(k))) fail('Unsupported Word action');
        const write = ['replace_text', 'insert_text'].includes(command.kind);
        if (write && (!policy.allowWrite || Office.context.document.mode !== 'readWrite')) fail('Document write not approved or read-only');
        if (command.kind === 'find_text') textValue(command.text);
        if (command.kind === 'replace_text') { textValue(command.search); textValue(command.replacement, true); }
        if (command.kind === 'insert_text') {
          textValue(command.text);
          if (!['start', 'end', 'selection'].includes(command.position)) fail('Unsupported insert position');
        }
        const run = async context => {
          const body = context.document.body;
          const selection = context.document.getSelection();
          body.load('text'); selection.load('text');
          const paragraphs = body.paragraphs;
          paragraphs.load('items/text');
          await context.sync();
          await checkIdentity();
          bounded(body.text);
          if (write && (snapshot === undefined || snapshot !== body.text)) fail('Document changed; read or locate again before writing');
          if (!target) {
            const matches = paragraphs.items.filter(p => p.text === policy.paragraph);
            if (matches.length !== 1) fail('Approved paragraph must be unique');
            target = matches[0].getRange('Content');
            context.trackedObjects.add(target);
          }
          target.load('text');
          const controls = target.contentControls;
          controls.load('items/cannotEdit');
          await context.sync();
          await checkIdentity();
          const oldText = target.text;
          if (!oldText) fail('Approved paragraph is empty or stale; pair again');
          if (command.kind === 'read_document') {
            snapshot = body.text;
            return bounded({ revision, text: body.text, selection: selection.text, paragraph: oldText });
          }
          const needle = command.kind === 'find_text' ? command.text : command.search;
          let range = target;
          if (needle !== undefined) {
            const first = oldText.indexOf(needle);
            if (first < 0 || oldText.indexOf(needle, first + 1) >= 0) fail('Text target must be unique in approved paragraph');
            const results = target.search(needle, { matchCase: true, matchWholeWord: false, matchWildcards: false, ignorePunct: false, ignoreSpace: false });
            results.load('items/text');
            await context.sync();
            if (results.items.length !== 1 || results.items[0].text !== needle) fail('Text target must be unique');
            range = results.items[0];
            if (!write) { snapshot = body.text; return { revision, text: needle, paragraph: oldText }; }
          }
          if (controls.items.some(c => c.cannotEdit)) fail('Protected content control');
          if (command.position === 'selection') {
            if (!selection.text) fail('Empty selection is ambiguous');
            const relation = selection.compareLocationWith(target);
            await context.sync();
            if (!['Inside', 'Equal'].includes(relation.value)) fail('Selection outside approved paragraph');
            range = selection;
          }
          const selectedText = selection.text;
          // Reload immediately before mutation; never retry a failed write or replace the body.
          body.load('text'); target.load('text'); range.load('text');
          await context.sync();
          await checkIdentity();
          if (body.text !== snapshot || target.text !== oldText || (needle !== undefined && range.text !== needle)) fail('Concurrent document edit; read again');
          if (command.position === 'selection' && range.text !== selectedText) fail('Selection changed; read again');
          const previous = range.text;
          const location = command.kind === 'replace_text' || command.position === 'selection' ? 'Replace' : command.position === 'start' ? 'Start' : 'End';
          const value = command.kind === 'replace_text' ? command.replacement : command.text;
          const offset = snapshot.indexOf(oldText);
          if (offset < 0 || snapshot.indexOf(oldText, offset + 1) >= 0) fail('Paragraph identity is ambiguous');
          let expectedParagraph;
          if (location === 'Start') expectedParagraph = value + oldText;
          else if (location === 'End') expectedParagraph = oldText + value;
          else {
            const index = oldText.indexOf(previous);
            if (index < 0 || oldText.indexOf(previous, index + 1) >= 0) fail('Replacement target is ambiguous');
            expectedParagraph = oldText.slice(0, index) + value + oldText.slice(index + previous.length);
          }
          const expectedBody = snapshot.slice(0, offset) + expectedParagraph + snapshot.slice(offset + oldText.length);
          const inserted = range.insertText(value, location);
          if (policy.bold !== null) inserted.font.bold = policy.bold;
          await context.sync();
          snapshot = undefined; // A failed readback must not authorize another write.
          await checkIdentity();
          target.load('text'); body.load('text'); inserted.load('text');
          if (policy.bold !== null) inserted.font.load('bold');
          await context.sync();
          await checkIdentity();
          if (inserted.text !== value || body.text !== expectedBody || (policy.bold !== null && inserted.font.bold !== policy.bold)) fail('Word write readback mismatch or concurrent edit');
          revision++;
          return bounded({ revision, text: inserted.text, paragraph: target.text, previous, bold: policy.bold });
        };
        return await (target ? Word.run(target, run) : Word.run(run));
      } finally { busy = false; }
    },
  };
}

// Task-pane bootstrap is also exported for use by a real Office WebView.
export async function mountWordTaskpane({ Office, Word, document, window, crypto }) {
  const status = document.getElementById('status');
  const form = document.getElementById('pair');
  const button = document.getElementById('connect');
  const identity = document.getElementById('document-id');
  let connection, actions, monitor, epoch = 0, ready = false;
  let documentId;
  const renewIdentity = () => { documentId = crypto.randomUUID(); identity.value = documentId; };
  renewIdentity(); // The trusted host must bind its token before the pane submits it.
  const closeConnection = () => {
    clearInterval(monitor); monitor = undefined;
    actions?.close(); connection?.close(); connection = undefined;
  };
  const stop = () => {
    epoch++;
    closeConnection();
    renewIdentity();
    button.disabled = !ready;
    status.textContent = '已停止。使用新的文档标识重新批准并配对。';
  };
  document.getElementById('stop').addEventListener('click', stop);
  window.addEventListener('pagehide', stop);
  window.addEventListener('beforeunload', stop);
  const officeReady = await Office.onReady();
  if (officeReady.host !== Office.HostType.Word || !wordCapabilities(Office).length) {
    status.textContent = '需要支持 WordApi 1.3 的 Microsoft Word。';
    return;
  }
  ready = true;
  button.disabled = false;
  status.textContent = '仅在临时文档中测试。先将文档标识交给本机宿主批准配对，再填写一次性令牌并批准段落。';
  form.addEventListener('submit', async event => {
    event.preventDefault();
    if (button.disabled || connection) return;
    const attempt = ++epoch;
    closeConnection();
    button.disabled = true;
    const pairedDocumentId = documentId;
    const tokenInput = document.getElementById('token');
    const token = tokenInput.value;
    tokenInput.value = '';
    try {
      const activeActions = createWordActions({ Office, Word, approved: {
        paragraph: document.getElementById('paragraph').value,
        allowWrite: document.getElementById('write').checked,
        bold: document.getElementById('bold').checked ? true : null,
      } });
      actions = activeActions;
      await activeActions.initialize();
      if (attempt !== epoch) { activeActions.close(); return; }
      const activeConnection = connectOffice({ endpoint: document.getElementById('endpoint').value, token, surface: 'word', documentId: pairedDocumentId, title: 'Word approved range', capabilities: wordCapabilities(Office), execute: action => activeActions.execute(action), WebSocketImpl: class extends window.WebSocket {
        constructor(url) {
          super(url);
          this.addEventListener('close', () => activeActions.close());
          this.addEventListener('error', () => activeActions.close());
        }
        close(...args) { activeActions.close(); return super.close(...args); }
      } });
      connection = activeConnection;
      await activeConnection.ready;
      if (attempt !== epoch) { activeConnection.close(); return; }
      status.textContent = `已配对临时文档 ${pairedDocumentId}。仅授权连接时批准的段落与操作。`;
      let checking = false;
      monitor = setInterval(async () => {
        if (checking) return;
        checking = true;
        try { await activeActions.checkIdentity(); } catch { if (attempt === epoch) stop(); }
        finally { checking = false; }
      }, 2000);
    } catch {
      if (attempt === epoch) {
        stop();
        status.textContent = '配对或执行准备失败；检查 Word、段落、本机可信 HTTPS 服务与一次性令牌，按新文档标识重新批准。';
      }
    } finally { if (attempt === epoch) button.disabled = !!connection; }
  });
}

if (typeof window !== 'undefined' && typeof document !== 'undefined') {
  mountWordTaskpane({ Office: globalThis.Office, Word: globalThis.Word, document, window, crypto: globalThis.crypto })
    .catch(() => { document.getElementById('status').textContent = 'Office.js 初始化失败，请在 Word 中打开此加载项。'; });
}
