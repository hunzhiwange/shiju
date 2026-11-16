// Browser text input for all widgets using Iced's InputMethod protocol.
// Owns its DOM nodes/listeners; no application messages or page coordinates.
export function createImeBridge(canvas, emit) {
    const document = canvas.ownerDocument;
    const listeners = [];
    const textarea = document.createElement('textarea');
    const password = document.createElement('input');
    password.type = 'password';
    let target = textarea;
    let enabled = false;
    let composing = false;
    let lastCommit = null;
    let disposed = false;
    let transferring = false;
    let geometry = null;
    let forwardedEdit = null;

    function on(node, name, handler, options) {
        node.addEventListener(name, handler, options);
        listeners.push(() => node.removeEventListener(name, handler, options));
    }
    function clear() {
        textarea.value = '';
        password.value = '';
        composing = false;
        lastCommit = null;
        forwardedEdit = null;
    }
    function position() {
        if (!enabled || !geometry) return;
        const { x, y, height } = geometry;
        const rect = canvas.getBoundingClientRect();
        const scaleX = rect.width / (canvas.width || rect.width || 1);
        const scaleY = rect.height / (canvas.height || rect.height || 1);
        target.style.left = `${rect.left + x * scaleX}px`;
        target.style.top = `${rect.top + y * scaleY}px`;
        target.style.height = `${Math.max(1, height * scaleY)}px`;
    }
    on(document, 'scroll', position, true);
    on(document.defaultView, 'resize', position);
    const viewport = document.defaultView.visualViewport;
    if (viewport) {
        on(viewport, 'resize', position);
        on(viewport, 'scroll', position);
    }
    function focusTarget() {
        if (!enabled || disposed || document.visibilityState === 'hidden') return;
        transferring = true;
        target.focus({ preventScroll: true });
        transferring = false;
    }
    // Run synchronously in a user gesture. A later animation-frame update cannot
    // obtain keyboard permission on mobile Safari. Never cancel Iced's pointer event.
    on(canvas, 'pointerup', event => {
        if (event.pointerType === 'touch' || event.pointerType === 'pen') focusTarget();
    });
    on(canvas, 'click', focusTarget);
    function editKey(key) {
        canvas.dispatchEvent(new KeyboardEvent('keydown', { key }));
        canvas.dispatchEvent(new KeyboardEvent('keyup', { key }));
    }
    function commit(text) {
        emit('preedit', '');
        if (text) emit('commit', text);
    }
    for (const node of [textarea, password]) {
        node.dataset.icedIme = '';
        node.tabIndex = -1;
        node.setAttribute('aria-hidden', 'true');
        node.setAttribute('autocomplete', 'off');
        node.setAttribute('autocorrect', 'off');
        node.setAttribute('autocapitalize', 'off');
        node.spellcheck = false;
        node.style.cssText = 'position:fixed;width:1px;height:1px;padding:0;border:0;opacity:0;pointer-events:none;resize:none;font-size:16px;';
        document.body.appendChild(node);
        on(node, 'compositionstart', () => {
            if (!enabled || node !== target) return;
            composing = true;
            lastCommit = null;
            emit('opened', '');
        });
        on(node, 'compositionupdate', event => {
            if (enabled && node === target) emit('preedit', event.data || '');
        });
        on(node, 'compositionend', event => {
            if (!enabled || node !== target) { node.value = ''; return; }
            composing = false;
            const text = event.data || '';
            node.value = '';
            lastCommit = text;
            commit(text);
        });
        on(node, 'beforeinput', event => {
            if (!enabled || node !== target) return;
            const type = event.inputType || '';
            // The keyboard can confirm composition with a paragraph event.
            // Consume it without forwarding Enter (which may submit a message).
            if (event.isComposing || composing || type.includes('Composition')) {
                if ((type === 'insertParagraph' || type === 'insertLineBreak') && event.cancelable) event.preventDefault();
                return;
            }
            const key = {
                deleteContentBackward: 'Backspace',
                deleteContentForward: 'Delete',
                insertParagraph: 'Enter',
                insertLineBreak: 'Enter',
            }[type];
            if (key) {
                // These edits must be cancelled: the empty DOM proxy is not the
                // widget's document, and an input event would apply the edit twice.
                if (!event.cancelable) return;
                event.preventDefault();
                node.value = '';
                const confirming = key === 'Enter' && lastCommit !== null;
                lastCommit = null;
                if (!confirming && forwardedEdit !== key) editKey(key);
                forwardedEdit = null;
                return;
            }
            // A fresh edit must not be mistaken for trailing composition input.
            lastCommit = null;
            forwardedEdit = null;
        });
        on(node, 'input', event => {
            if (!enabled || node !== target) { node.value = ''; return; }
            if (event.isComposing || composing) return;
            // Some IME edits are non-cancelable. Apply their input notification
            // through the same keyboard protocol, never as proxy text.
            const key = {
                deleteContentBackward: 'Backspace',
                deleteContentForward: 'Delete',
                insertParagraph: 'Enter',
                insertLineBreak: 'Enter',
            }[event.inputType];
            if (key) {
                node.value = '';
                const confirming = key === 'Enter' && lastCommit !== null;
                lastCommit = null;
                if (!confirming && forwardedEdit !== key) editKey(key);
                forwardedEdit = null;
                return;
            }
            const text = node.value;
            node.value = '';
            if (lastCommit !== null && text === lastCommit) { lastCommit = null; return; }
            lastCommit = null;
            commit(text);
        });
        on(node, 'keydown', event => {
            if (!enabled || node !== target) return;
            // keyCode 229 covers older engines reporting the IME acceptance key.
            if (event.isComposing || composing || event.keyCode === 229) return;
            lastCommit = null;
            const shortcut = event.ctrlKey || event.metaKey;
            const key = event.key.toLowerCase();
            // Native paste produces an input event, including multiline text.
            if (shortcut && key === 'v') return;
            // Printable text comes from input, not a guessed keyboard character.
            if (!shortcut && Array.from(event.key).length === 1) return;
            // Iced expects the unmodified character for shortcut matching.
            const forwarded = new KeyboardEvent('keydown', {
                key: shortcut && Array.from(event.key).length === 1 ? key : event.key,
                code: event.code, location: event.location, repeat: event.repeat,
                ctrlKey: event.ctrlKey, metaKey: event.metaKey,
                altKey: event.altKey, shiftKey: event.shiftKey,
            });
            forwardedEdit = ['Backspace', 'Delete', 'Enter'].includes(event.key) ? event.key : null;
            canvas.dispatchEvent(forwarded);
            // Keep browser clipboard copy/cut handling out of the empty proxy.
            event.preventDefault();
        });
        on(node, 'keyup', event => {
            forwardedEdit = null;
            if (enabled && node === target && !event.isComposing && !composing) canvas.dispatchEvent(new KeyboardEvent('keyup', event));
        });
        on(node, 'focus', () => { if (!transferring && enabled) emit('focus', ''); });
        on(node, 'blur', event => {
            if (transferring) return;
            clear();
            emit('closed', '');
            if (event.relatedTarget !== canvas) emit('blur', '');
        });
    }
    return {
        update(allow, x, y, width, height, secure) {
            if (disposed) return;
            if (!allow) {
                if (enabled) emit('closed', '');
                enabled = false;
                if (this.hasFocus()) {
                    transferring = true;
                    canvas.focus({ preventScroll: true });
                    transferring = false;
                }
                clear();
                return;
            }
            enabled = true;
            const next = secure ? password : textarea;
            if (target !== next) { clear(); target = next; }
            geometry = { x, y, height };
            position();
            // Never steal focus from another page control or an inactive tab.
            if (document.activeElement === canvas || document.activeElement === textarea || document.activeElement === password) {
                focusTarget();
            }
        },
        hasFocus() { return document.activeElement === textarea || document.activeElement === password || transferring; },
        dispose() {
            if (disposed) return;
            disposed = true;
            enabled = false;
            for (const remove of listeners) remove();
            clear();
            textarea.remove();
            password.remove();
        },
    };
}
