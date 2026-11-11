// Shared Office task-pane transport. Credentials stay in memory, never URLs/storage.
// A trusted localhost certificate is required in the Office WebView. The desktop
// supplies endpoint + one-time token after explicit approval for this document.
const MAX_BYTES = 64 * 1024;
const COMMANDS = {
  excel: new Set(['read_range', 'write_range']),
  word: new Set(['read_document', 'find_text', 'replace_text', 'insert_text']),
};
const bytes = value => new TextEncoder().encode(value).length;

export function connectOffice({ endpoint, token, surface, documentId, title, capabilities, execute, WebSocketImpl = globalThis.WebSocket, timeoutMs = 15000 }) {
  const url = new URL(endpoint);
  if (url.protocol !== 'wss:' || !['localhost', '127.0.0.1', '[::1]'].includes(url.hostname) || url.pathname !== '/office' || url.search || url.hash || url.username || url.password) throw new Error('Invalid Office endpoint');
  if (!COMMANDS[surface] || typeof token !== 'string' || !/^[a-f0-9]{64}$/.test(token) || typeof documentId !== 'string' || !documentId.trim() || bytes(documentId) > 256 || typeof execute !== 'function' || !Array.isArray(capabilities) || !Number.isFinite(timeoutMs) || timeoutMs <= 0 || timeoutMs > 15000) throw new Error('Invalid Office pairing');
  const socket = new WebSocketImpl(url.href);
  let sequence = 0, lastAck = 0, generation, stopped = false, busy = false, heartbeat, deadline;
  const seen = new Set();
  let resolveReady, rejectReady;
  const ready = new Promise((resolve, reject) => { resolveReady = resolve; rejectReady = reject; });
  // A caller may close before awaiting ready; retain rejection for the caller.
  ready.catch(() => {});
  const close = () => {
    if (stopped) return;
    stopped = true;
    token = undefined;
    clearTimeout(deadline);
    clearInterval(heartbeat);
    rejectReady(new Error('Office disconnected'));
    socket.close();
  };
  const send = message => {
    if (stopped || socket.readyState !== 1) throw new Error('Office disconnected');
    const text = JSON.stringify(message);
    if (bytes(text) > MAX_BYTES) throw new Error('Office message too large');
    socket.send(text);
  };
  const armDeadline = () => {
    clearTimeout(deadline);
    deadline = setTimeout(close, timeoutMs);
  };
  armDeadline();
  socket.addEventListener('open', () => {
    try {
      send({ type: 'pair', token, surface, document_id: documentId, title, capabilities });
      token = undefined;
    } catch { close(); }
  });
  socket.addEventListener('close', close);
  socket.addEventListener('error', close);
  socket.addEventListener('message', async event => {
    try {
      if (stopped || typeof event.data !== 'string' || bytes(event.data) > MAX_BYTES) throw new Error('Invalid Office message');
      const message = JSON.parse(event.data);
      if (!generation) {
        if (message.type !== 'paired' || message.surface !== surface || message.document_id !== documentId || typeof message.generation !== 'string' || !message.generation) throw new Error('Office identity mismatch');
        generation = message.generation;
        clearTimeout(deadline);
        deadline = undefined;
        heartbeat = setInterval(() => {
          try { send({ type: 'heartbeat', sequence: ++sequence }); if (!deadline) armDeadline(); } catch { close(); }
        }, 10000);
        resolveReady({ generation, surface, documentId });
        return;
      }
      if (message.type === 'heartbeat') {
        if (!Number.isSafeInteger(message.sequence) || message.sequence <= lastAck || message.sequence > sequence) throw new Error('Invalid heartbeat');
        lastAck = message.sequence;
        clearTimeout(deadline);
        deadline = undefined;
        return;
      }
      const command = message.action?.command;
      if (message.type !== 'command' || message.surface !== surface || message.document_id !== documentId || message.generation !== generation || message.action?.action !== surface || !COMMANDS[surface].has(command?.kind) || !capabilities.includes(`${surface}_${command.kind}`) || typeof message.request_id !== 'string' || !message.request_id || seen.has(message.request_id) || busy || seen.size >= 1024) throw new Error('Invalid Office command');
      seen.add(message.request_id);
      busy = true;
      let timer;
      const value = await Promise.race([
        Promise.resolve().then(() => execute(message.action)),
        new Promise((_, reject) => { timer = setTimeout(() => reject(new Error('Office command timed out')), timeoutMs); }),
      ]).finally(() => clearTimeout(timer));
      if (stopped) return; // Never send a late result after close/reload/revoke.
      send({ type: 'result', sequence: ++sequence, request_id: message.request_id, value });
      busy = false;
    } catch { close(); }
  });
  return { ready, close };
}
