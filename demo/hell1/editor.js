/**
 * Simple TypeScript Editor
 * A lightweight code editor with syntax highlighting and basic IDE features
 */
var SimpleEditor = /** @class */ (function () {
    function SimpleEditor() {
        var _a, _b, _c;
        this.editor = document.getElementById('editor');
        this.lineNumbers = document.getElementById('line-numbers');
        this.cursorPos = document.getElementById('cursor-pos');
        this.charCount = document.getElementById('char-count');
        this.lineCount = document.getElementById('line-count');
        this.fileNameDisplay = document.getElementById('file-name');
        this.fileInput = document.getElementById('file-input');
        this.fontSizeSelect = document.getElementById('font-size');
        this.state = {
            fileName: 'untitled.txt',
            content: '',
            isDirty: false,
            fontSize: 14,
        };
        this.undoStack = [];
        this.redoStack = [];
        this.lastContent = '';
        this.init();
    }
    SimpleEditor.prototype.init = function () {
        this.bindEvents();
        this.updateLineNumbers();
        this.updateStats();
        this.editor.focus();
    };
    SimpleEditor.prototype.bindEvents = function () {
        var _this = this;
        // Editor events
        this.editor.addEventListener('input', function () { return _this.handleInput(); });
        this.editor.addEventListener('scroll', function () { return _this.syncScroll(); });
        this.editor.addEventListener('keydown', function (e) { return _this.handleKeydown(e); });
        this.editor.addEventListener('click', function () { return _this.updateCursorPos(); });
        this.editor.addEventListener('keyup', function () { return _this.updateCursorPos(); });
        // Toolbar events
        (_a = document.getElementById('btn-new')) === null || _a === void 0 ? void 0 : _a.addEventListener('click', function () { return _this.newFile(); });
        (_b = document.getElementById('btn-save')) === null || _b === void 0 ? void 0 : _b.addEventListener('click', function () { return _this.saveFile(); });
        (_c = document.getElementById('btn-open')) === null || _c === void 0 ? void 0 : _c.addEventListener('click', function () { return _this.openFile(); });
        document.getElementById('btn-undo') === null || document.getElementById('btn-undo') === void 0 ? void 0 : document.getElementById('btn-undo').addEventListener('click', function () { return _this.undo(); });
        document.getElementById('btn-redo') === null || document.getElementById('btn-redo') === void 0 ? void 0 : document.getElementById('btn-redo').addEventListener('click', function () { return _this.redo(); });
        this.fontSizeSelect.addEventListener('change', function (e) {
            var size = parseInt(e.target.value);
            _this.setFontSize(size);
        });
        // File input
        this.fileInput.addEventListener('change', function (e) { return _this.handleFileSelect(e); });
        // Keyboard shortcuts
        document.addEventListener('keydown', function (e) { return _this.handleGlobalKeydown(e); });
        // Before unload warning
        window.addEventListener('beforeunload', function (e) {
            if (_this.state.isDirty) {
                e.preventDefault();
                e.returnValue = '';
            }
        });
    };
    SimpleEditor.prototype.handleInput = function () {
        var currentContent = this.editor.value;
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
    };
    SimpleEditor.prototype.handleKeydown = function (e) {
        // Tab key - insert spaces
        if (e.key === 'Tab') {
            e.preventDefault();
            var start = this.editor.selectionStart;
            var end = this.editor.selectionEnd;
            var value = this.editor.value;
            if (e.shiftKey) {
                // Outdent
                var beforeCursor = value.substring(0, start);
                var lineStart = beforeCursor.lastIndexOf('\n') + 1;
                var lineContent = value.substring(lineStart, start);
                if (lineContent.startsWith('  ')) {
                    this.editor.value = value.substring(0, lineStart) + value.substring(lineStart + 2);
                    this.editor.selectionStart = this.editor.selectionEnd = start - 2;
                }
            }
            else {
                // Indent
                this.editor.value = value.substring(0, start) + '  ' + value.substring(end);
                this.editor.selectionStart = this.editor.selectionEnd = start + 2;
            }
            this.handleInput();
        }
        // Auto-close brackets and quotes
        var pairs = {
            '(': ')',
            '[': ']',
            '{': '}',
            '"': '"',
            "'": "'",
            '`': '`',
        };
        if (pairs[e.key]) {
            var start = this.editor.selectionStart;
            var end = this.editor.selectionEnd;
            var value = this.editor.value;
            if (start !== end) {
                // Wrap selection
                e.preventDefault();
                var selected = value.substring(start, end);
                this.editor.value = value.substring(0, start) + e.key + selected + pairs[e.key] + value.substring(end);
                this.editor.selectionStart = start + 1;
                this.editor.selectionEnd = end + 1;
                this.handleInput();
            }
        }
    };
    SimpleEditor.prototype.handleGlobalKeydown = function (e) {
        var isMac = navigator.platform.toUpperCase().indexOf('MAC') >= 0;
        var ctrl = isMac ? e.metaKey : e.ctrlKey;
        if (ctrl && e.key === 's') {
            e.preventDefault();
            this.saveFile();
        }
        else if (ctrl && e.key === 'n') {
            e.preventDefault();
            this.newFile();
        }
        else if (ctrl && e.key === 'o') {
            e.preventDefault();
            this.openFile();
        }
        else if (ctrl && e.key === 'z') {
            e.preventDefault();
            this.undo();
        }
        else if (ctrl && e.key === 'y') {
            e.preventDefault();
            this.redo();
        }
    };
    SimpleEditor.prototype.syncScroll = function () {
        this.lineNumbers.scrollTop = this.editor.scrollTop;
    };
    SimpleEditor.prototype.updateLineNumbers = function () {
        var lines = this.editor.value.split('\n').length;
        var html = '';
        for (var i = 1; i <= lines; i++) {
            html += "<span class=\"line-number\">".concat(i, "</span>");
        }
        this.lineNumbers.innerHTML = html;
    };
    SimpleEditor.prototype.updateStats = function () {
        var content = this.editor.value;
        var lines = content.split('\n');
        var chars = content.length;
        this.charCount.textContent = "".concat(chars, " chars");
        this.lineCount.textContent = "".concat(lines.length, " line").concat(lines.length !== 1 ? 's' : '');
    };
    SimpleEditor.prototype.updateCursorPos = function () {
        var pos = this.editor.selectionStart;
        var value = this.editor.value.substring(0, pos);
        var lines = value.split('\n');
        var line = lines.length;
        var col = lines[lines.length - 1].length + 1;
        this.cursorPos.textContent = "Ln ".concat(line, ", Col ").concat(col);
    };
    SimpleEditor.prototype.updateFileName = function () {
        var displayName = this.state.isDirty ? "\u25CF ".concat(this.state.fileName) : this.state.fileName;
        this.fileNameDisplay.textContent = displayName;
    };
    SimpleEditor.prototype.setFontSize = function (size) {
        this.state.fontSize = size;
        this.editor.style.fontSize = "".concat(size, "px");
        this.lineNumbers.style.fontSize = "".concat(size, "px");
    };
    // File operations
    SimpleEditor.prototype.newFile = function () {
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
    };
    SimpleEditor.prototype.openFile = function () {
        var _a;
        (_a = this.fileInput) === null || _a === void 0 ? void 0 : _a.click();
    };
    SimpleEditor.prototype.handleFileSelect = function (e) {
        var _this = this;
        var _a;
        var file = (_a = e.target.files) === null || _a === void 0 ? void 0 : _a[0];
        if (!file)
            return;
        var reader = new FileReader();
        reader.onload = function (event) {
            var _a;
            var content = (_a = event.target) === null || _a === void 0 ? void 0 : _a.result;
            _this.editor.value = content;
            _this.state.content = content;
            _this.state.fileName = file.name;
            _this.state.isDirty = false;
            _this.undoStack = [];
            _this.redoStack = [];
            _this.lastContent = content;
            _this.updateLineNumbers();
            _this.updateStats();
            _this.updateFileName();
            _this.editor.focus();
        };
        reader.readAsText(file);
        // Reset file input
        e.target.value = '';
    };
    SimpleEditor.prototype.saveFile = function () {
        var content = this.editor.value;
        var blob = new Blob([content], { type: 'text/plain' });
        var url = URL.createObjectURL(blob);
        var a = document.createElement('a');
        a.href = url;
        a.download = this.state.fileName;
        a.click();
        URL.revokeObjectURL(url);
        this.state.isDirty = false;
        this.updateFileName();
    };
    SimpleEditor.prototype.undo = function () {
        if (this.undoStack.length === 0)
            return;
        var previous = this.undoStack.pop();
        this.redoStack.push(this.editor.value);
        this.editor.value = previous;
        this.lastContent = previous;
        this.state.content = previous;
        this.state.isDirty = true;
        this.updateLineNumbers();
        this.updateStats();
        this.updateFileName();
    };
    SimpleEditor.prototype.redo = function () {
        if (this.redoStack.length === 0)
            return;
        var next = this.redoStack.pop();
        this.undoStack.push(this.editor.value);
        this.editor.value = next;
        this.lastContent = next;
        this.state.content = next;
        this.state.isDirty = true;
        this.updateLineNumbers();
        this.updateStats();
        this.updateFileName();
    };
    return SimpleEditor;
}());
// Initialize editor when DOM is ready
document.addEventListener('DOMContentLoaded', function () {
    new SimpleEditor();
});
