import test from 'node:test';
import assert from 'node:assert/strict';
import { createExcelActions, excelCapabilities } from './actions.mjs';
import { connectOffice } from '../shared/connection.mjs';

// Office.js is the only mocked boundary: loads and writes become visible at sync.
function fixture(options = {}) {
  let url = 'file:///temporary.xlsx', closed = false, syncs = 0, writes = 0;
  const data = { address: 'Sheet1!A1:B2', values: [[1, 2], ['', '']], formulas: [[1, 2], ['', '']], numberFormat: [['General', 'General'], ['General', 'General']] };
  const queue = [];
  const snapshots = new Map();
  const range = { load(keys) { queue.push(() => {
    for (const key of keys.split(',')) snapshots.set(key, structuredClone(data[key]));
  }); } };
  for (const key of ['address', 'values', 'formulas', 'numberFormat']) {
    Object.defineProperty(range, key, { get() {
      if (!snapshots.has(key)) throw new Error('PropertyNotLoaded');
      return structuredClone(snapshots.get(key));
    }, set(value) {
      writes++;
      queue.push(() => {
        data[key] = structuredClone(value);
        if (key === 'values') data.formulas = structuredClone(value);
        if (key === 'formulas') data.values = value.map(row => row.map(cell => cell === '=SUM(A1:B1)' ? 3 : cell));
      });
    } });
  }
  const protection = { load() { queue.push(() => { protection.protected = !!options.protected; }); } };
  const context = { workbook: { application: { load() { queue.push(() => { context.workbook.application.calculationMode = options.manualCalculation ? 'Manual' : 'Automatic'; }); } }, worksheets: { getItem(name) { assert.equal(name, 'Sheet1'); return { protection, getRange() { return range; } }; } } }, async sync() { syncs++; while (queue.length) queue.shift()(); options.onSync?.(syncs); } };
  const Office = { context: { requirements: { isSetSupported: () => !options.unsupported }, document: { mode: options.readOnly ? 'readOnly' : 'readWrite', getFilePropertiesAsync(callback) { callback({ status: 'succeeded', value: { url } }); } } } };
  const Excel = { run: async callback => { if (closed) throw new Error('closed'); return callback(context); } };
  const actions = createExcelActions({ Office, Excel, approved: { sheet: 'Sheet1', range: 'A1:B2', allowOverwrite: false, allowFormulas: false, numberFormat: null, ...options.approved } });
  const command = (kind, fields = {}) => ({ action: 'excel', command: { kind, sheet: 'Sheet1', range: 'A1:B2', ...fields } });
  return { actions, command, data, Office, Excel, get syncs() { return syncs; }, get writes() { return writes; }, saveAs() { url = 'file:///renamed.xlsx'; }, close() { closed = true; actions.close(); } };
}

test('cc_control_13_read_loads_and_syncs_values_formulas_and_format', async () => {
  const f = fixture(); await f.actions.initialize();
  const result = await f.actions.execute(f.command('read_range'));
  assert.deepEqual(result.values, [[1, 2], ['', '']]);
  assert.deepEqual(result.formulas, result.values);
  assert.equal(result.address, 'Sheet1!A1:B2');
  assert.equal(result.number_format[0][0], 'General');
  assert.ok(f.syncs >= 1);
});
test('cc_control_13_write_reads_back_values_after_sync', async () => {
  const f = fixture({ approved: { allowOverwrite: true } }); await f.actions.initialize();
  const result = await f.actions.execute(f.command('write_range', { values: [[4, true], ['text', null]] }));
  assert.deepEqual(result.values, [[4, true], ['text', '']]); assert.ok(f.syncs >= 3);
});
test('cc_control_13_approved_formula_and_number_format_are_read_back', async () => {
  const f = fixture({ approved: { allowOverwrite: true, allowFormulas: true, numberFormat: '0.00' } }); await f.actions.initialize();
  const result = await f.actions.execute(f.command('write_range', { values: [[1, 2], ['=SUM(A1:B1)', '']] }));
  assert.equal(result.values[1][0], 3); assert.equal(result.formulas[1][0], '=SUM(A1:B1)'); assert.equal(result.number_format[0][0], '0.00');
});
test('cc_control_13_outside_scope_and_oversized_ranges_never_write', async () => {
  const f = fixture(); await f.actions.initialize();
  for (const fields of [{ range: 'C1' }, { sheet: 'Other' }, { range: 'A1:XFD1048576' }, { range: 'A:A' }, { range: 'A1,B1' }, { range: 'A0' }]) {
    await assert.rejects(f.actions.execute(f.command('read_range', fields)));
  }
  assert.equal(f.writes, 0);
});
test('cc_control_13_protected_readonly_and_unapproved_overwrite_refuse_writes', async () => {
  for (const options of [{ protected: true }, { readOnly: true }, {}]) {
    const f = fixture(options); await f.actions.initialize();
    await assert.rejects(f.actions.execute(f.command('write_range', { values: [[3, 4], [5, 6]] })));
    assert.equal(f.writes, 0);
  }
});
test('cc_control_13_formula_injection_external_references_and_macros_are_rejected', async () => {
  for (const allowFormulas of [false, true]) {
    const f = fixture({ approved: { allowOverwrite: true, allowFormulas } }); await f.actions.initialize();
    for (const value of ['=WEBSERVICE("https://example.com")', '=Other!A1', '=SUM(C1:C2)', '=CALL("x")', '=HYPERLINK("x")', '+1', '@SUM(A1)', '=SUM(A1:B1)+evil', '=A1 B1', '=1 2']) {
      await assert.rejects(f.actions.execute(f.command('write_range', { values: [[value, 1], [2, 3]] })));
    }
    assert.equal(f.writes, 0);
  }
});
test('cc_control_13_invalid_shape_nonfinite_and_unknown_commands_refuse', async () => {
  const f = fixture({ approved: { allowOverwrite: true } }); await f.actions.initialize();
  for (const values of [[[1]], [[{}, 1], [2, 3]], [[Infinity, 1], [2, 3]]]) await assert.rejects(f.actions.execute(f.command('write_range', { values })));
  await assert.rejects(f.actions.execute(f.command('macro')));
  await assert.rejects(f.actions.execute(f.command('write_range', { values: [[1, 2], [3, 4]], surprise: true })));
  assert.equal(f.writes, 0);
});
test('cc_control_13_save_as_and_close_invalidate_old_actions', async () => {
  const f = fixture(); await f.actions.initialize(); f.saveAs();
  await assert.rejects(f.actions.execute(f.command('read_range')));
  const g = fixture(); await g.actions.initialize(); g.close();
  await assert.rejects(g.actions.execute(g.command('read_range')));
});
test('cc_control_13_unsupported_requirement_set_does_not_advertise_actions', async () => {
  assert.deepEqual(excelCapabilities({ context: { requirements: { isSetSupported: () => false } } }), []);
  const f = fixture({ unsupported: true }); await assert.rejects(f.actions.initialize());
  assert.equal(f.writes, 0);
});
test('cc_control_13_close_during_preflight_prevents_queued_write', async () => {
  let f; f = fixture({ approved: { allowOverwrite: true }, onSync: () => f.actions.close() }); await f.actions.initialize();
  await assert.rejects(f.actions.execute(f.command('write_range', { values: [[1, 2], [3, 4]] })));
  assert.equal(f.writes, 0);
});

class OfficeSocket {
  static current;
  listeners = new Map();
  readyState = 0;
  sent = [];
  constructor() { OfficeSocket.current = this; }
  addEventListener(kind, callback) { this.listeners.set(kind, callback); }
  send(value) { this.sent.push(JSON.parse(value)); }
  close() { this.readyState = 3; this.listeners.get('close')?.(); }
  async message(value) { await this.listeners.get('message')({ data: JSON.stringify(value) }); }
}
test('cc_control_13_shared_transport_executes_bound_excel_action_and_discards_old_generation', async () => {
  const f = fixture({ approved: { allowOverwrite: true } }); await f.actions.initialize();
  const connection = connectOffice({ endpoint: 'wss://localhost:1234/office', token: 'a'.repeat(64), surface: 'excel', documentId: 'temporary-book', title: 'Temporary', capabilities: ['excel_read_range', 'excel_write_range'], execute: action => f.actions.execute(action), WebSocketImpl: OfficeSocket });
  try {
    const socket = OfficeSocket.current;
    socket.readyState = 1; socket.listeners.get('open')();
    await socket.message({ type: 'paired', surface: 'excel', document_id: 'temporary-book', generation: 'current' });
    await connection.ready;
    const message = { type: 'command', surface: 'excel', document_id: 'temporary-book', generation: 'current', request_id: 'one', action: f.command('write_range', { values: [[7, 8], [9, 10]] }) };
    await socket.message(message);
    assert.deepEqual(socket.sent[1].value.values, [[7, 8], [9, 10]]);
    assert.equal(socket.sent[1].request_id, 'one');
    const writes = f.writes;
    await socket.message({ ...message, generation: 'old', request_id: 'two' });
    assert.equal(socket.readyState, 3); assert.equal(f.writes, writes); assert.equal(socket.sent.length, 2);
  } finally { connection.close(); f.close(); }
});
test('cc_control_13_request_and_response_size_limits_refuse_oversized_payloads', async () => {
  const f = fixture({ approved: { allowOverwrite: true } }); await f.actions.initialize();
  await assert.rejects(f.actions.execute(f.command('write_range', { values: [['x'.repeat(5000), 1], [2, 3]] })));
  assert.equal(f.writes, 0);
  f.data.values = [['x'.repeat(50000), 1], [2, 3]];
  await assert.rejects(f.actions.execute(f.command('read_range')), /size limit/);
});

test('cc_control_13_readonly_capability_and_invalid_approval_are_explicit', () => {
  const f = fixture({ readOnly: true });
  assert.deepEqual(excelCapabilities(f.Office), ['excel_read_range']);
  assert.deepEqual(excelCapabilities({}), []);
  for (const patch of [{ range: 'XFE1' }, { range: 'A1048577' }, { range: 'B2:A1' }, { numberFormat: 'arbitrary' }, { sheet: '' }]) {
    assert.throws(() => createExcelActions({ Office: f.Office, Excel: f.Excel, approved: { sheet: 'Sheet1', range: 'A1:B2', allowOverwrite: false, allowFormulas: false, numberFormat: null, ...patch } }));
  }
});

test('cc_control_13_manual_calculation_refuses_formula_before_mutation', async () => {
  const f = fixture({ manualCalculation: true, approved: { allowOverwrite: true, allowFormulas: true } }); await f.actions.initialize();
  await assert.rejects(f.actions.execute(f.command('write_range', { values: [[1, 2], ['=SUM(A1:B1)', '']] })), /automatic calculation/);
  assert.equal(f.writes, 0);
});
