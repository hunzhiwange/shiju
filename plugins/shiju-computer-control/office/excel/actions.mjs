import { connectOffice } from '../shared/connection.mjs';

const MAX_CELLS = 1024;
const MAX_BYTES = 48 * 1024;
const FORMATS = new Set(['General', '0', '0.00', '0%', '0.00%', 'yyyy-mm-dd', '@']);
const size = value => new TextEncoder().encode(JSON.stringify(value)).length;
const fail = message => { throw new Error(message); };

export function excelCapabilities(Office) {
  return Office?.context?.requirements?.isSetSupported?.('ExcelApi', '1.2')
    ? (Office.context.document?.mode === 'readWrite' ? ['excel_read_range', 'excel_write_range'] : ['excel_read_range']) : [];
}

function rectangle(value) {
  if (typeof value !== 'string' || value.length > 40) fail('Invalid A1 range');
  const match = /^([A-Z]{1,3})([1-9][0-9]{0,6})(?::([A-Z]{1,3})([1-9][0-9]{0,6}))?$/.exec(value);
  if (!match) fail('Only a single bounded A1 rectangle is supported');
  const column = name => [...name].reduce((total, c) => total * 26 + c.charCodeAt(0) - 64, 0);
  const box = { left: column(match[1]), top: Number(match[2]), right: column(match[3] || match[1]), bottom: Number(match[4] || match[2]) };
  if (box.right < box.left || box.bottom < box.top || box.right > 16384 || box.bottom > 1048576) fail('Invalid A1 bounds');
  box.rows = box.bottom - box.top + 1;
  box.columns = box.right - box.left + 1;
  if (box.rows * box.columns > MAX_CELLS) fail('Range exceeds 1024 cells');
  return box;
}
function contains(outer, inner) {
  return inner.left >= outer.left && inner.right <= outer.right && inner.top >= outer.top && inner.bottom <= outer.bottom;
}

// Deliberately small scalar grammar. No external links, names, VBA, spill functions,
// sheet references, locale expressions, DDE, or functions capable of network access.
function safeFormula(value, scope) {
  if (value.length > 512 || /\s/.test(value)) fail('Unsupported formula');
  const expression = value.slice(1).toUpperCase();
  const reference = '[A-Z]{1,3}[1-9][0-9]{0,6}';
  const argument = `(?:${reference}(?::${reference})?|[0-9]+(?:\\.[0-9]+)?)`;
  const aggregate = new RegExp(`^(?:SUM|MIN|MAX|AVERAGE|COUNT)\\(${argument}(?:,${argument})*\\)$`);
  if (!aggregate.test(expression)) {
    const tokens = expression.match(/[A-Z]{1,3}[1-9][0-9]{0,6}|[0-9]+(?:\.[0-9]+)?|[()+*/^\-]/g) || [];
    if (tokens.join('') !== expression || !tokens.length) fail('Unsupported formula');
    let depth = 0, operand = true;
    for (const token of tokens) {
      if (token === '(') { if (!operand) fail('Invalid formula'); depth++; }
      else if (token === ')') { if (operand || --depth < 0) fail('Invalid formula'); operand = false; }
      else if (/^[+*/^\-]$/.test(token)) {
        if (operand && token !== '-' && token !== '+') fail('Invalid formula');
        operand = true;
      } else { if (!operand) fail('Invalid formula'); operand = false; }
    }
    if (depth || operand) fail('Invalid formula');
  }
  for (const ref of expression.match(new RegExp(`${reference}(?::${reference})?`, 'g')) || []) {
    if (!contains(scope, rectangle(ref))) fail('Formula reference outside approved range');
  }
  return value;
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

export function createExcelActions({ Office, Excel, approved }) {
  const policy = structuredClone(approved);
  if (typeof policy?.sheet !== 'string' || !policy.sheet.trim() || policy.sheet.length > 31) fail('Invalid worksheet');
  const scope = rectangle(policy.range);
  if (typeof policy.allowOverwrite !== 'boolean' || typeof policy.allowFormulas !== 'boolean' || (policy.numberFormat !== null && !FORMATS.has(policy.numberFormat))) fail('Invalid range approval');
  let initialUrl, initialized = false, closed = false, busy = false;
  const close = () => { closed = true; };
  async function checkIdentity() {
    if (!initialized || closed) fail('Document session closed');
    try {
      if (await fileUrl(Office) !== initialUrl) { close(); fail('Document changed; pair again'); }
      if (closed) fail('Document session closed');
    } catch (error) { close(); throw error; }
  }
  return {
    close,
    checkIdentity,
    async initialize() {
      if (closed || initialized || !excelCapabilities(Office).length || typeof Excel?.run !== 'function') fail('ExcelApi 1.2 is required');
      initialUrl = await fileUrl(Office);
      if (closed) fail('Document session closed');
      initialized = true;
    },
    async execute(action) {
      if (busy) fail('Excel document busy');
      busy = true;
      try {
        await checkIdentity();
        const command = action?.command;
        if (action?.action !== 'excel' || !command || !['read_range', 'write_range'].includes(command.kind)) fail('Unsupported Excel action');
        const write = command.kind === 'write_range';
        const keys = write ? ['kind', 'sheet', 'range', 'values'] : ['kind', 'sheet', 'range'];
        if (Object.keys(action).some(key => !['action', 'command'].includes(key)) || Object.keys(command).some(key => !keys.includes(key))) fail('Unsupported command fields');
        const box = rectangle(command.range);
        if (command.sheet !== policy.sheet || !contains(scope, box)) fail('Range outside approved scope');
        let values, hasFormula = false;
        if (write) {
          if (Office.context.document.mode !== 'readWrite') fail('Workbook is read-only');
          if (!Array.isArray(command.values) || command.values.length !== box.rows || size(command.values) > MAX_BYTES) fail('Invalid value matrix');
          values = command.values.map(row => {
            if (!Array.isArray(row) || row.length !== box.columns) fail('Invalid value matrix');
            return row.map(value => {
              if (value === null) return '';
              if (typeof value === 'boolean' || (typeof value === 'number' && Number.isFinite(value))) return value;
              if (typeof value !== 'string' || value.length > 4096) fail('Unsupported cell value');
              if (/^[\s\u0000-\u001f]*[=+@-]/.test(value)) {
                if (!value.startsWith('=') || !policy.allowFormulas) fail('Formula not approved');
                hasFormula = true;
                return safeFormula(value, scope);
              }
              return value;
            });
          });
        }
        return await Excel.run(async context => {
          const sheet = context.workbook.worksheets.getItem(policy.sheet);
          const range = sheet.getRange(command.range);
          range.load('address,values,formulas,numberFormat');
          if (write) sheet.protection.load('protected');
          if (hasFormula) context.workbook.application.load('calculationMode');
          await context.sync();
          await checkIdentity();
          if (write) {
            if (sheet.protection.protected) fail('Protected worksheet');
            if (hasFormula && context.workbook.application.calculationMode !== 'Automatic') fail('Formula writes require automatic calculation');
            if (!policy.allowOverwrite && range.formulas.some(row => row.some(value => value !== '' && value !== null))) fail('Overwrite not approved');
            if (hasFormula) range.formulas = values;
            else range.values = values;
            if (policy.numberFormat !== null) range.numberFormat = values.map(row => row.map(() => policy.numberFormat));
            // Commit first, then queue a fresh load and sync for calculated values.
            await context.sync();
            await checkIdentity();
            range.load('address,values,formulas,numberFormat');
            await context.sync();
            await checkIdentity();
          }
          const result = { sheet: policy.sheet, range: command.range, address: range.address, values: range.values, formulas: range.formulas, number_format: range.numberFormat };
          if (size(result) > MAX_BYTES) fail('Excel result exceeds size limit');
          return result;
        });
      } finally { busy = false; }
    },
  };
}

// Task-pane bootstrap is also exported for use by a real Office WebView.
export async function mountExcelTaskpane({ Office, Excel, document, window, crypto }) {
  const status = document.getElementById('status');
  const form = document.getElementById('pair');
  let connection, actions, monitor;
  const stop = () => { clearInterval(monitor); monitor = undefined; actions?.close(); connection?.close(); connection = undefined; status.textContent = '已停止。重新批准并配对后才能继续。'; };
  document.getElementById('stop').addEventListener('click', stop);
  window.addEventListener('pagehide', stop);
  window.addEventListener('beforeunload', stop);
  const ready = await Office.onReady();
  if (ready.host !== Office.HostType.Excel || !excelCapabilities(Office).length) {
    status.textContent = '需要支持 ExcelApi 1.2 的 Microsoft Excel。';
    return;
  }
  document.getElementById('connect').disabled = false;
  status.textContent = '仅在临时工作簿中测试。填写本机宿主配对信息并批准范围。';
  form.addEventListener('submit', async event => {
    event.preventDefault();
    stop();
    const button = document.getElementById('connect');
    button.disabled = true;
    const tokenInput = document.getElementById('token');
    const token = tokenInput.value;
    tokenInput.value = '';
    try {
      actions = createExcelActions({ Office, Excel, approved: {
        sheet: document.getElementById('sheet').value,
        range: document.getElementById('range').value,
        allowOverwrite: document.getElementById('overwrite').checked,
        allowFormulas: document.getElementById('formulas').checked,
        numberFormat: document.getElementById('format').value || null,
      } });
      await actions.initialize();
      const activeActions = actions;
      const documentId = crypto.randomUUID(); // New identity on every pane load/pair.
      connection = connectOffice({ endpoint: document.getElementById('endpoint').value, token, surface: 'excel', documentId, title: 'Excel approved range', capabilities: excelCapabilities(Office), execute: action => activeActions.execute(action), WebSocketImpl: class extends window.WebSocket {
        constructor(url) {
          super(url);
          this.addEventListener('close', () => activeActions.close());
          this.addEventListener('error', () => activeActions.close());
        }
        close(...args) { activeActions.close(); return super.close(...args); }
      } });
      await connection.ready;
      status.textContent = `已配对临时文档 ${documentId}。仅授权连接时批准的范围与操作。`;
      let checking = false;
      monitor = setInterval(async () => {
        if (checking) return;
        checking = true;
        try { await activeActions.checkIdentity(); } catch { if (actions === activeActions) stop(); }
        finally { checking = false; }
      }, 2000);
    } catch { stop(); status.textContent = '配对或执行准备失败；检查 Excel、范围、本机可信 HTTPS 服务与一次性令牌。'; }
    finally { button.disabled = false; }
  });
}

if (typeof window !== 'undefined' && typeof document !== 'undefined') {
  mountExcelTaskpane({ Office: globalThis.Office, Excel: globalThis.Excel, document, window, crypto: globalThis.crypto })
    .catch(() => { document.getElementById('status').textContent = 'Office.js 初始化失败，请在 Excel 中打开此加载项。'; });
}
