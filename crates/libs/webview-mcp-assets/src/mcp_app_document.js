const host = window.parent;
const channel = new MessageChannel();
const port = channel.port1;
const send = port.postMessage.bind(port);
const dispatch = window.dispatchEvent.bind(window);
const HostMessageEvent = MessageEvent;
const sendRpc = host.postMessage.bind(host);
let linkId = 0;
let pendingLink;
let linkTimer;

// The SDK still uses window.postMessage and checks event.source === parent.
// Receiving through this port prevents host data reaching replacement documents.
port.onmessage = ({data}) => {
    if (typeof data.ack === "number") {
        send({ack: data.ack});
    } else if (pendingLink && data.message?.id === pendingLink) {
        pendingLink = undefined;
        clearTimeout(linkTimer);
        if (data.message.error) send({notice: "linkDenied"});
    } else {
        dispatch(new HostMessageEvent("message", {
            source: host,
            origin: data.origin,
            data: data.message,
        }));
    }
};

window.addEventListener("pagehide", () => {
    send({closed: true});
    port.close();
}, {once: true});
window.addEventListener("securitypolicyviolation", event => {
    if (event.isTrusted && event.disposition === "enforce") {
        send({blocked: event.effectiveDirective});
    }
});

host.postMessage(documentNonce, "*", [channel.port2]);

function openLink(url) {
    if (pendingLink) return;
    pendingLink = `host-link:${documentNonce}:${++linkId}`;
    sendRpc({jsonrpc: "2.0", id: pendingLink, method: "ui/open-link", params: {url}}, "*");
    linkTimer = setTimeout(() => {
        pendingLink = undefined;
        send({notice: "linkDenied"});
    }, 65000);
}

function isExternalWebLink(target) {
    return (target?.protocol === "https:" || target?.protocol === "http:") &&
        (target.protocol !== location.protocol || target.host !== location.host);
}

function followExternalLink(event) {
    if (!event.isTrusted || event.defaultPrevented ||
        (event.type === "auxclick" && event.button !== 1)) return;
    const link = event.composedPath().find(node => node instanceof HTMLAnchorElement);
    if (!link || !link.hasAttribute("href") || link.hasAttribute("download")) return;
    if (isExternalWebLink(link)) {
        event.preventDefault();
        openLink(link.href);
    } else if (link.target.toLowerCase() === "_blank") {
        event.preventDefault();
        send({notice: "popupBlocked"});
    }
}
window.addEventListener("click", followExternalLink);
window.addEventListener("auxclick", followExternalLink);

// Sandboxed popups have no WindowProxy. A URL-only call can still request
// browser navigation; a blank window or custom scheme cannot be replayed.
window.open = (url) => {
    try {
        const target = typeof url === "string" && url.trim() ? new URL(url, document.baseURI) : null;
        if (isExternalWebLink(target)) {
            openLink(target.href);
            return null;
        }
    } catch {}
    send({notice: "popupBlocked"});
    return null;
};
