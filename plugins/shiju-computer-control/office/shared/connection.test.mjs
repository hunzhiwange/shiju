import test from 'node:test';
import assert from 'node:assert/strict';
import { connectOffice } from './connection.mjs';

class WebViewSocket {
  static current;
  readyState = 0;
  listeners = new Map();
  sent = [];
  constructor(url) { this.url = url; WebViewSocket.current = this; }
  addEventListener(type, callback) { this.listeners.set(type, callback); }
  emit(type, value) { return this.listeners.get(type)?.(value); }
  open() { this.readyState = 1; this.emit('open'); }
  send(text) { this.sent.push(JSON.parse(text)); }
  close() { this.readyState = 3; this.emit('close'); }
  message(value) { return this.emit('message', { data: JSON.stringify(value) }); }
}
const options = { endpoint: 'wss://localhost:12345/office', token: 'a'.repeat(64), surface: 'excel', documentId: 'book', title: 'Untitled', capabilities: ['excel_read_range'], WebSocketImpl: WebViewSocket, execute: async () => ({ values: [[42]] }) };
const paired = { type: 'paired', surface: 'excel', document_id: 'book', generation: 'generation-a' };
const command = { type: 'command', surface: 'excel', document_id: 'book', generation: 'generation-a', request_id: 'request-a', action: { action: 'excel', command: { kind: 'read_range', sheet: 'Sheet1', range: 'A1' } } };
async function connected(overrides = {}) {
  const connection = connectOffice({ ...options, ...overrides });
  const socket = WebViewSocket.current;
  socket.open();
  await socket.message(paired);
  await connection.ready;
  return { connection, socket };
}

test('cc_control_12_pairing_keeps_secret_out_of_endpoint_and_uses_bound_identity', async () => {
  const { connection, socket } = await connected();
  assert.equal(socket.url, options.endpoint);
  assert.equal(socket.sent[0].token, options.token);
  assert.equal(socket.sent[0].document_id, 'book');
  assert.equal(socket.sent[0].surface, 'excel');
  connection.close();
});

test('cc_control_12_rejects_nonlocal_plaintext_credential_urls', () => {
  for (const endpoint of ['ws://localhost/office', 'wss://evil.example/office', 'wss://localhost/office?token=x', 'wss://user:pass@localhost/office']) {
    assert.throws(() => connectOffice({ ...options, endpoint }));
  }
});

test('cc_control_12_only_bound_advertised_commands_execute_and_reply', async () => {
  const { connection, socket } = await connected();
  await socket.message(command);
  assert.deepEqual(socket.sent[1], { type: 'result', sequence: 1, request_id: 'request-a', value: { values: [[42]] } });
  await socket.message(command);
  assert.equal(socket.readyState, 3);
  connection.close();
});

test('cc_control_12_wrong_document_surface_generation_and_unknown_commands_close', async () => {
  for (const patch of [{ document_id: 'other' }, { surface: 'word' }, { generation: 'old' }, { action: { action: 'excel', command: { kind: 'eval' } } }, { action: { action: 'excel', command: { kind: 'write_range' } } }]) {
    let calls = 0;
    const { connection, socket } = await connected({ execute: () => { calls++; } });
    await socket.message({ ...command, ...patch });
    assert.equal(calls, 0);
    assert.equal(socket.readyState, 3);
    connection.close();
  }
});

test('cc_control_12_disconnect_discards_inflight_result', async () => {
  let finish;
  const { connection, socket } = await connected({ execute: () => new Promise(resolve => { finish = resolve; }) });
  const job = socket.message(command);
  await Promise.resolve();
  connection.close();
  finish({ values: [[42]] });
  await job;
  assert.equal(socket.sent.length, 1);
});

test('cc_control_12_oversized_result_and_command_timeout_close', async () => {
  for (const execute of [async () => 'x'.repeat(65536), () => new Promise(() => {})]) {
    const { connection, socket } = await connected({ execute, timeoutMs: 10 });
    await socket.message(command);
    assert.equal(socket.readyState, 3);
    assert.equal(socket.sent.length, 1);
    connection.close();
  }
});

test('cc_control_12_pairing_timeout_and_wrong_pair_identity_fail', async () => {
  const connection = connectOffice({ ...options, timeoutMs: 10 });
  await assert.rejects(connection.ready);
  const wrong = connectOffice(options);
  const socket = WebViewSocket.current;
  socket.open();
  await socket.message({ ...paired, document_id: 'other' });
  await assert.rejects(wrong.ready);
  assert.equal(socket.readyState, 3);
});

test('cc_control_12_heartbeat_replay_closes_the_connection', async () => {
  const { connection, socket } = await connected();
  await socket.message({ type: 'heartbeat', sequence: 1 });
  assert.equal(socket.readyState, 3);
  connection.close();
});
