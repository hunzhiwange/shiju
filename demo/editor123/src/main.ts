import * as monaco from 'monaco-editor';

// 配置 Monaco Editor workers
(self as any).MonacoEnvironment = {
  getWorker(_: unknown, label: string) {
    const getWorkerModule = (url: string, label: string) => {
      return new Worker(new URL(url, import.meta.url), { type: 'module' });
    };
    switch (label) {
      case 'json': return getWorkerModule('monaco-editor/esm/vs/language/json/json.worker.js?worker', label);
      case 'css': case 'scss': case 'less': return getWorkerModule('monaco-editor/esm/vs/language/css/css.worker.js?worker', label);
      case 'html': case 'handlebars': case 'razor': return getWorkerModule('monaco-editor/esm/vs/language/html/html.worker.js?worker', label);
      case 'typescript': case 'javascript': return getWorkerModule('monaco-editor/esm/vs/language/typescript/ts.worker.js?worker', label);
      default: return getWorkerModule('monaco-editor/esm/vs/editor/editor.worker.js?worker', label);
    }
  }
};

// 创建编辑器实例
const editor = monaco.editor.create(document.getElementById('editor-container')!, {
  value: `// 欢迎使用 TypeScript 编辑器
// 在这里编写你的代码

interface User {
  id: number;
  name: string;
  email: string;
}

function greet(user: User): string {
  return \`Hello, \${user.name}!\`;
}

const user: User = {
  id: 1,
  name: "Alice",
  email: "alice@example.com"
};

console.log(greet(user));
`,
  language: 'typescript',
  theme: 'vs-dark',
  automaticLayout: true,
  fontSize: 14,
  minimap: { enabled: true },
  scrollBeyondLastLine: false,
  wordWrap: 'on',
  tabSize: 2,
  insertSpaces: true,
  renderWhitespace: 'selection',
  bracketPairColorization: { enabled: true },
  guides: { bracketPairs: true }
});

// 状态栏更新
const statusRight = document.getElementById('status-right')!;
editor.onDidChangeCursorPosition((e) => {
  statusRight.textContent = `Ln ${e.position.lineNumber}, Col ${e.position.column}`;
});

// 工具栏按钮
document.getElementById('btn-format')!.addEventListener('click', () => {
  editor.getAction('editor.action.formatDocument')?.run();
});

document.getElementById('btn-clear')!.addEventListener('click', () => {
  editor.setValue('');
});

document.getElementById('btn-run')!.addEventListener('click', () => {
  const code = editor.getValue();
  try {
    // 使用 TypeScript 编译器转译
    const result = (window as any).ts.transpileModule(code, {
      compilerOptions: { module: 'commonjs', target: 'es2015' }
    });
    // 执行转译后的代码
    const output = new Function('console', result.outputText);
    const logs: string[] = [];
    const mockConsole = {
      log: (...args: unknown[]) => logs.push(args.map(a => String(a)).join(' ')),
      error: (...args: unknown[]) => logs.push('[ERROR] ' + args.map(a => String(a)).join(' ')),
      warn: (...args: unknown[]) => logs.push('[WARN] ' + args.map(a => String(a)).join(' '))
    };
    output(mockConsole);
    alert('输出:\n' + (logs.join('\n') || '(无输出)'));
  } catch (e) {
    alert('执行错误: ' + (e as Error).message);
  }
});

// 加载 TypeScript 编译器用于运行时转译
const tsScript = document.createElement('script');
tsScript.src = 'https://cdn.jsdelivr.net/npm/typescript@5.3.3/lib/typescript.min.js';
document.head.appendChild(tsScript);
