import test from 'node:test';
import assert from 'node:assert/strict';
import { createWordActions, wordCapabilities } from './actions.mjs';

function fixture({ duplicate = false, protectedText = false, readOnly = false } = {}) {
  let text = '唯一中文段落', outside = '段落外文字';
  const load = function () { return this; };
  const controls = { items: [{ cannotEdit: protectedText }], load };
  function range(get, put) {
    return { get text() { return get(); }, load, contentControls: controls,
      search(needle) {
        return { load, items: get().includes(needle) ? [range(() => needle, value => { text = text.replace(needle, value); })] : [] };
      },
      insertText(value, location) {
        put(location === 'Start' ? value + get() : location === 'End' ? get() + value : value);
        return { text: value, font: {}, load };
      },
    };
  }
  const target = range(() => text, value => { text = value; });
  const paragraph = { get text() { return text; }, getRange: () => target };
  const selection = range(() => '', () => { throw new Error('Unexpected empty selection mutation'); });
  const context = {
    document: { body: { get text() { return text + '\r' + outside; }, load,
      paragraphs: { items: duplicate ? [paragraph, paragraph] : [paragraph], load } }, getSelection: () => selection },
    trackedObjects: { add() {} }, async sync() {},
  };
  const Office = { context: { requirements: { isSetSupported: () => true }, document: {
    mode: readOnly ? 'readOnly' : 'readWrite', getFilePropertiesAsync(callback) { callback({ status: 'succeeded', value: { url: 'file:///temporary.docx' } }); },
  } } };
  const Word = { run: (...args) => args.at(-1)(context) };
  const actions = createWordActions({ Office, Word, approved: { paragraph: text, allowWrite: true, bold: null } });
  const execute = command => actions.execute({ action: 'word', command });
  return { Office, actions, execute, edit: value => { text = value; }, outside: () => outside };
}

test('cc_control_15_reads_body_and_selection', async () => {
  const f = fixture(); await f.actions.initialize();
  const value = await f.execute({ kind: 'read_document' });
  assert.equal(value.paragraph, '唯一中文段落'); assert.equal(value.selection, '');
  assert.match(value.text, /段落外文字/);
});
test('cc_control_15_unique_find_and_chinese_replace_readback', async () => {
  const f = fixture(); await f.actions.initialize();
  await f.execute({ kind: 'find_text', text: '中文' });
  const value = await f.execute({ kind: 'replace_text', search: '中文', replacement: '汉字' });
  assert.equal(value.text, '汉字'); assert.equal(value.paragraph, '唯一汉字段落');
  assert.equal(f.outside(), '段落外文字');
});
test('cc_control_15_inserts_at_approved_paragraph_end', async () => {
  const f = fixture(); await f.actions.initialize(); await f.execute({ kind: 'read_document' });
  const value = await f.execute({ kind: 'insert_text', position: 'end', text: '新增' });
  assert.equal(value.paragraph, '唯一中文段落新增');
  await assert.rejects(f.execute({ kind: 'insert_text', position: 'end', text: '再次' }), /read or locate/);
});
test('cc_control_15_rejects_duplicate_paragraph', async () => {
  const f = fixture({ duplicate: true }); await f.actions.initialize();
  await assert.rejects(f.execute({ kind: 'read_document' }), /unique/);
});
test('cc_control_15_rejects_protected_target', async () => {
  const f = fixture({ protectedText: true }); await f.actions.initialize(); await f.execute({ kind: 'read_document' });
  await assert.rejects(f.execute({ kind: 'replace_text', search: '中文', replacement: '编辑' }), /Protected/);
});
test('cc_control_15_rejects_concurrent_change', async () => {
  const f = fixture(); await f.actions.initialize(); await f.execute({ kind: 'read_document' }); f.edit('外部修改');
  await assert.rejects(f.execute({ kind: 'insert_text', position: 'end', text: '编辑' }), /Document changed/);
});
test('cc_control_15_rejects_empty_selection_and_read_only', async () => {
  const f = fixture(); await f.actions.initialize(); await f.execute({ kind: 'read_document' });
  await assert.rejects(f.execute({ kind: 'insert_text', position: 'selection', text: '编辑' }), /Empty selection/);
  const ro = fixture({ readOnly: true }); await ro.actions.initialize();
  await assert.rejects(ro.execute({ kind: 'insert_text', position: 'end', text: '编辑' }), /read-only/);
  assert.deepEqual(wordCapabilities(ro.Office), ['word_read_document', 'word_find_text']);
});
test('cc_control_15_rejects_missing_api_and_closed_generation', async () => {
  assert.deepEqual(wordCapabilities({}), []);
  const f = fixture(); await f.actions.initialize(); f.actions.close();
  await assert.rejects(f.execute({ kind: 'read_document' }), /closed/);
});
