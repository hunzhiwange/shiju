const frame = document.querySelector("iframe");
let port;
let connected = false;
let requestId = 0;
let pendingBytes = 0;
const pending = new Map();
const reported = new Set();
const encoder = new TextEncoder();
const linkStatus = document.querySelector("#link-status");
let noticeTimer;
function showNotice(text) {
    linkStatus.textContent = text;
    linkStatus.hidden = false;
    clearTimeout(noticeTimer);
    noticeTimer = setTimeout(() => { linkStatus.hidden = true; }, 4500);
}
window.__mcpPopupBlocked = () => showNotice("MCP 应用无法打开此弹窗，请使用外部链接。");

window.addEventListener("message", event => {
    if (event.source !== frame.contentWindow || event.origin !== "null") return;

    if (!connected && event.data === documentNonce && event.ports.length === 1) {
        connected = true;
        port = event.ports[0];
        port.onmessage = ({data}) => {
            if (data.closed === true) {
                port.close();
                port = undefined;
                pending.clear();
                pendingBytes = 0;
                window.ipc.postMessage(JSON.stringify({nonce: hostNonce, closed: true}));
                return;
            }
            if (data.notice === "popupBlocked") {
                window.__mcpPopupBlocked();
                return;
            }
            if (data.notice === "linkDenied") {
                showNotice("未打开外部链接。");
                return;
            }
            if (data.blocked) {
                if (!reported.has(data.blocked)) {
                    reported.add(data.blocked);
                    window.ipc.postMessage(JSON.stringify({nonce: hostNonce, blocked: data.blocked}));
                }
                return;
            }
            const request = pending.get(data.ack);
            if (request !== undefined) {
                pending.delete(data.ack);
                pendingBytes -= request.bytes;
                window.ipc.postMessage(request.body);
            }
        };
        return;
    }

    if (!port || event.data?.jsonrpc !== "2.0" || pending.size >= 32) return;
    let body;
    try {
        body = JSON.stringify({nonce: hostNonce, message: event.data});
    } catch {
        return;
    }
    if (body.length + pendingBytes > maxMessageBytes) return;
    const bytes = encoder.encode(body).length;
    if (bytes + pendingBytes > maxMessageBytes) return;

    // A WindowProxy survives navigation. Require an acknowledgement from the
    // original document's private port before forwarding its window message.
    const id = ++requestId;
    pending.set(id, {body, bytes});
    pendingBytes += bytes;
    port.postMessage({ack: id});
});

window.__mcpDeliver = message => port?.postMessage({message, origin: location.origin});
frame.src = "/app.html";
