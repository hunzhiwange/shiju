/**
 * Simple TypeScript Editor
 * A lightweight code editor with syntax highlighting and basic IDE features
 */

interface EditorState {
  fileName: string;
  content: string;
  isDirty: boolean;
  fontSize: number;
}

class SimpleEditor {
  private editor: HTMLTextAreaElement;
  private lineNumbers: HTMLElement;
  private cursorPos: HTMLElement;
  private charCount: HTMLElement;
  private lineCount: HTMLElement;
  private fileNameDisplay: HTMLElement;
  private fileInput: HTMLInputElement;
  private fontSizeSelect: HTMLSelectElement;
  private state: EditorState;
  private undoStack: string[] = [];
  private redoStack: string[] = [];
  private lastContent: string = '';

  constructor() {
    this.editor = document.getElementById('editor') as HTMLTextAreaElement;
    this.lineNumbers = document.getElementById('line-numbers') as HTMLElement;
    this.cursorPos = document.getElementById('cursor-pos') as HTMLElement;
    this.charCount = document.getElementById('char-count') as HTMLElement;
    this.lineCount = document.getElementById('line-count') as HTMLElement;
    this.fileNameDisplay = document.getElementById('file-name') as HTMLElement;
    this.fileInput = document.getElementById('file-input') as HTMLInputElement;
    this.fontSizeSelect = document.getElementById('font-size') as HTMLSelectElement;

    this.state = {
      fileName: 'untitled.txt',
      content: '',
      isDirty: false,
      fontSize: 14,
    };

    this.init();
  }

  private init(): void {
    this.bindEvents();
    this.updateLineNumbers();
    this.updateStats();
    this.editor.focus();
  }

  private bindEvents(): void {
    // Editor events
    this.editor.addEventListener('input', () => this.handleInput());
    this.editor.addEventListener('scroll', () => this.syncScroll());
    this.editor.addEventListener('keydown', (e) => this.handleKeydown(e));
    this.editor.addEventListener('click', () => this.updateCursorPos());
    this.editor.addEventListener('keyup', () => this.updateCursorPos());

    // Toolbar events
    document.getElementById('btn-new')?.addEventListener('click', () => this.newFile());
    document.getElementById('btn-save')?.addEventListener('click', () => this.saveFile());
    document.getElementById('btn-open')?.addEventListener('click', () => this.openFile());
    document.getElementById('btn-undo')?.addEventListener('click', () => this.undo());
    document.getElementById('btn-redo')?.addEventListener('click', () => this.redo());

    this.fontSizeSelect.addEventListener('change', (e) => {
      const size = parseInt((e.target as HTMLSelectElement).value);
      this.setFontSize(size);
    });

    // File input
    this.fileInput.addEventListener('change', (e) => this.handleFileSelect(e));

    // Keyboard shortcuts
    document.addEventListener('keydown', (e) => this.handleGlobalKeydown(e));

    // Before unload warning
    window.addEventListener('beforeunload', (e) => {
      if (this.state.isDirty) {
        e.preventDefault();
        e.returnValue = '';
      }
    });
  }

  private handleInput(): void {
    const currentContent = this.editor.value;
    
    // Save to undo stack
    if (this.lastContent !== currentContent) {
      this.undoStack.push(this.lastContent);
      this.redoStack = [];
      this.lastContent = currentContent;
    }

    this.state.content = currentContent;
    this.state.isDirty = true;
    this.updateLineNumbers();
    this.updateStats();
    this.updateFileName();
  }

  private handleKeydown(e: KeyboardEvent): void {
    // Tab key - insert spaces
    if (e.key === 'Tab') {
      e.preventDefault();
      const start = this.editor.selectionStart;
      const end = this.editor.selectionEnd;
      const value = this.editor.value;

      if (e.shiftKey) {
        // Outdent
        const beforeCursor = value.substring(0, start);
        const lineStart = beforeCursor.lastIndexOf('\n') + 1;
        const lineContent = value.substring(lineStart, start);
        
        if (lineContent.startsWith('  ')) {
          this.editor.value = value.substring(0, lineStart) + value.substring(lineStart + 2);
          this.editor.selectionStart = this.editor.selectionEnd = start - 2;
        }
      } else {
        // Indent
        this.editor.value = value.substring(0, start) + '  ' + value.substring(end);
        this.editor.selectionStart = this.editor.selectionEnd = start + 2;
      }
      this.handleInput();
    }

    // Auto-close brackets and quotes
    const pairs: Record<string, string> = {
      '(': ')',
      '[': ']',
      '{': '}',
      '"': '"',
      "'": "'",
      '`': '`',
    };

    if (pairs[e.key]) {
      const start = this.editor.selectionStart;
      const end = this.editor.selectionEnd;
      const value = this.editor.value;

      if (start !== end) {
        // Wrap selection
        e.preventDefault();
        const selected = value.substring(start, end);
        this.editor.value = value.substring(0, start) + e.key + selected + pairs[e.key] + value.substring(end);
        this.editor.selectionStart = start + 1;
        this.editor.selectionEnd = end + 1;
        this.handleInput();
      }
    }
  }

  private handleGlobalKeydown(e: KeyboardEvent): void {
    const isMac = navigator.platform.toUpperCase().indexOf('MAC') >= 0;
    const ctrl = isMac ? e.metaKey : e.ctrlKey;

    if (ctrl && e.key === 's') {
      e.preventDefault();
      this.saveFile();
    } else if (ctrl && e.key === 'n') {
      e.preventDefault();
      this.newFile();
    } else if (ctrl && e.key === 'o') {
      e.preventDefault();
      this.openFile();
    } else if (ctrl && e.key === 'z') {
      e.preventDefault();
      this.undo();
    } else if (ctrl && e.key === 'y') {
      e.preventDefault();
      this.redo();
    }
  }

  private syncScroll(): void {
    this.lineNumbers.scrollTop = this.editor.scrollTop;
  }

  private updateLineNumbers(): void {
    const lines = this.editor.value.split('\n').length;
    let html = '';
    for (let i = 1; i <= lines; i++) {
      html += `<span class="line-number">${i}</span>`;
    }
    this.lineNumbers.innerHTML = html;
  }

  private updateStats(): void {
    const content = this.editor.value;
    const lines = content.split('\n');
    const chars = content.length;

    this.charCount.textContent = `${chars} chars`;
    this.lineCount.textContent = `${lines.length} line${lines.length !== 1 ? 's' : ''}`;
  }

  private updateCursorPos(): void {
    const pos = this.editor.selectionStart;
    const value = this.editor.value.substring(0, pos);
    const lines = value.split('\n');
    const line = lines.length;
    const col = lines[lines.length - 1].length + 1;

    this.cursorPos.textContent = `Ln ${line}, Col ${col}`;
  }

  private updateFileName(): void {
    const displayName = this.state.isDirty ? `● ${this.state.fileName}` : this.state.fileName;
    this.fileNameDisplay.textContent = displayName;
  }

  private setFontSize(size: number): void {
    this.state.fontSize = size;
    this.editor.style.fontSize = `${size}px`;
    this.lineNumbers.style.fontSize = `${size}px`;
  }

  // File operations
  private newFile(): void {
    if (this.state.isDirty) {
      if (!confirm('You have unsaved changes. Create new file anyway?')) {
        return;
      }
    }

    this.editor.value = '';
    this.state.content = '';
    this.state.fileName = 'untitled.txt';
    this.state.isDirty = false;
    this.undoStack = [];
    this.redoStack = [];
    this.lastContent = '';
    
    this.updateLineNumbers();
    this.updateStats();
    this.updateFileName();
    this.editor.focus();
  }

  private openFile(): void {
    this.fileInput.click();
  }

  private handleFileSelect(e: Event): void {
    const file = (e.target as HTMLInputElement).files?.[0];
    if (!file) return;

    const reader = new FileReader();
    reader.onload = (event) => {
      const content = event.target?.result as string;
      this.editor.value = content;
      this.state.content = content;
      this.state.fileName = file.name;
      this.state.isDirty = false;
      this.undoStack = [];
      this.redoStack = [];
      this.lastContent = content;
      
      this.updateLineNumbers();
      this.updateStats();
      this.updateFileName();
      this.editor.focus();
    };
    reader.readAsText(file);

    // Reset file input
    (e.target as HTMLInputElement).value = '';
  }

  private saveFile(): void {
    const content = this.editor.value;
    const blob = new Blob([content], { type: 'text/plain' });
    const url = URL.createObjectURL(blob);
    
    const a = document.createElement('a');
    a.href = url;
    a.download = this.state.fileName;
    a.click();
    
    URL.revokeObjectURL(url);
    this.state.isDirty = false;
    this.updateFileName();
  }

  private undo(): void {
    if (this.undoStack.length === 0) return;

    const previous = this.undoStack.pop()!;
    this.redoStack.push(this.editor.value);
    this.editor.value = previous;
    this.lastContent = previous;
    this.state.content = previous;
    this.state.isDirty = true;
    
    this.updateLineNumbers();
    this.updateStats();
    this.updateFileName();
  }

  private redo(): void {
    if (this.redoStack.length === 0) return;

    const next = this.redoStack.pop()!;
    this.undoStack.push(this.editor.value);
    this.editor.value = next;
    this.lastContent = next;
    this.state.content = next;
    this.state.isDirty = true;
    
    this.updateLineNumbers();
    this.updateStats();
    this.updateFileName();
  }
}

// Initialize editor when DOM is ready
document.addEventListener('DOMContentLoaded', () => {
  new SimpleEditor();
});
