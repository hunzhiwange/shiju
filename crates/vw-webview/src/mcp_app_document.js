const host = window.parent;
const channel = new MessageChannel();
const port = channel.port1;
const send = port.postMessage.bind(port);
const dispatch = window.dispatchEvent.bind(window);
const HostMessageEvent = MessageEvent;

// The SDK still uses window.postMessage and checks event.source === parent.
// Receiving through this port prevents host data reaching replacement documents.
port.onmessage = ({data}) => {
    if (typeof data.ack === "number") {
        send({ack: data.ack});
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
