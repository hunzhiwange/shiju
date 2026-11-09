use super::*;
use std::future::ready;
use vw_gateway_client::{GatewayClient, GatewayEndpoint};

const ORIGIN: &str = "https://links.example.test:8443";
const URL: &str = "https://links.example.test:8443/report?q=demo#section";

fn client(port: u16) -> GatewayClient {
    GatewayClient::new(GatewayEndpoint::new("127.0.0.1", port)).unwrap()
}

fn policy() -> McpAppNetworkPolicy {
    McpAppNetworkPolicy::parse(Some(&json!({"redirectDomains":[ORIGIN]}))).unwrap()
}

fn initialize() -> Value {
    json!({"jsonrpc":"2.0","id":"init","method":"ui/initialize"})
}

fn initialized() -> Value {
    json!({"jsonrpc":"2.0","method":"ui/notifications/initialized"})
}

fn link(id: Value, url: &str) -> Value {
    json!({"jsonrpc":"2.0","id":id,"method":"ui/open-link","params":{"url":url}})
}

#[cfg(unix)]
struct Fixture {
    process: McpAppProcess,
    send: std::sync::mpsc::Sender<String>,
    capture: tempfile::TempDir,
}

#[cfg(unix)]
impl Fixture {
    fn new(policy: McpAppNetworkPolicy) -> Self {
        use std::process::{Command, Stdio};
        let capture = tempfile::tempdir().unwrap();
        let mut child = Command::new("sh")
            .args(["-c", "exec cat > \"$1\"", "sh"])
            .arg(capture.path().join("replies"))
            .stdin(Stdio::piped())
            .spawn()
            .unwrap();
        let input = child.stdin.take().unwrap();
        let (send, events) = std::sync::mpsc::channel();
        let mut bridge =
            McpAppBridge::new("session".into(), "server".into(), "null".into(), BTreeSet::new())
                .unwrap();
        bridge.redirect_domains = policy.redirect_domains;
        Self {
            process: McpAppProcess {
                child,
                input,
                events,
                bridge,
                document_active: Arc::new(AtomicBool::new(true)),
                session: "session".into(),
                server: "server".into(),
            },
            send,
            capture,
        }
    }

    fn send(&self, origin: &str, message: Value) {
        self.send.send(json!({"origin":origin,"message":message}).to_string()).unwrap();
    }

    fn handshake(&self) {
        self.send("null", initialize());
        self.send("null", initialized());
    }

    fn replies(mut self) -> Vec<Value> {
        let path = self.capture.path().join("replies");
        let deadline = std::time::Instant::now() + Duration::from_secs(3);
        while !std::fs::read_to_string(&path).is_ok_and(|text| text.contains("\n")) {
            assert!(std::time::Instant::now() < deadline, "missing RPC replies");
            std::thread::sleep(Duration::from_millis(10));
        }
        // close() writes its teardown marker to the same pipe before waiting.
        self.process.close().unwrap();
        std::fs::read_to_string(path)
            .unwrap()
            .lines()
            .map(|line| serde_json::from_str(line).unwrap())
            .filter(|message: &Value| message["id"] != "host-teardown")
            .collect()
    }
}

#[cfg(unix)]
#[tokio::test]
async fn mcp_app_open_link_dispatches_after_confirmation() {
    let mut f = Fixture::new(policy());
    f.handshake();
    f.send("null", link(json!("approve"), URL));
    f.send("null", link(json!("approve"), URL));
    f.send("null", link(json!(27), URL));
    let confirmed = std::cell::Cell::new(0);
    let mut opened = vec![];
    f.process
        .poll_gateway(
            &client(1),
            "binding",
            |origin| {
                assert_eq!(origin, ORIGIN);
                confirmed.set(confirmed.get() + 1);
                ready(confirmed.get() == 1)
            },
            |url| {
                assert_eq!(confirmed.get(), 1, "browser opens only after approval");
                opened.push(url.to_owned());
                true
            },
            || false,
        )
        .await
        .unwrap();
    assert_eq!(confirmed.get(), 2, "a replay must not create a new confirmation");
    assert_eq!(opened, [URL]);
    let replies = f.replies();
    assert_eq!(replies[0]["result"]["hostCapabilities"]["openLinks"], json!({}));
    assert_eq!(replies[1], json!({"jsonrpc":"2.0","id":"approve","result":{}}));
    assert_eq!(replies[2]["error"]["message"], "Duplicate external link request");
    assert_eq!(replies[3]["id"], 27);
    assert_eq!(replies[3]["error"]["message"], "External link opening denied by user");
}

#[cfg(unix)]
#[tokio::test]
async fn mcp_app_open_link_rejects_forged_and_unapproved_requests() {
    let mut f = Fixture::new(policy());
    f.send("null", link(json!(1), URL));
    f.handshake();
    f.send("https://untrusted.test", link(json!(2), URL));
    f.send("null", json!({"jsonrpc":"2.0","id":3,"method":"native/confirm","params":{"url":URL}}));
    let mut without_id = link(Value::Null, URL);
    without_id.as_object_mut().unwrap().remove("id");
    f.send("null", without_id);
    for (index, url) in [
        "javascript:alert(1)",
        "file:///tmp/private",
        "http://links.example.test:8443/report",
        "https://links.example.test/report",
        "https://links.example.test:8443.evil.test/report",
        "https://user@links.example.test:8443/report",
        "https:///links.example.test:8443/report",
        "https://links.example.test:8443/%0a",
    ]
    .into_iter()
    .enumerate()
    {
        f.send("null", link(json!(index + 4), url));
    }
    f.process
        .poll_gateway(
            &client(1),
            "binding",
            |_| async { panic!("no confirmation") },
            |_| panic!("no open"),
            || false,
        )
        .await
        .unwrap();
    let replies = f.replies();
    assert_eq!(replies.len(), 13);
    assert_eq!(replies[0]["error"]["message"], "MCP app is not ready");
    assert_eq!(replies[1]["id"], "init");
    assert!(replies[2..].iter().all(|reply| reply.get("error").is_some()));
}

#[cfg(unix)]
#[tokio::test]
async fn mcp_app_open_link_revokes_approval_on_close() {
    for closes_panel in [false, true] {
        let mut f = Fixture::new(policy());
        f.handshake();
        f.send("null", link(json!("revoked"), URL));
        let document = f.process.document_active.clone();
        let cancelled = std::cell::Cell::new(false);
        f.process
            .poll_gateway(
                &client(1),
                "binding",
                |_| async {
                    if closes_panel {
                        cancelled.set(true);
                    } else {
                        document.store(false, Ordering::Release);
                    }
                    true
                },
                |_| panic!("a revoked request must not open"),
                || cancelled.get(),
            )
            .await
            .unwrap();
        let replies = f.replies();
        assert_eq!(replies.len(), 1, "no reply is delivered to a revoked document");
    }
}

#[cfg(unix)]
#[tokio::test]
async fn mcp_app_open_link_capability_requires_a_bound_policy() {
    let mut f = Fixture::new(McpAppNetworkPolicy::default());
    f.handshake();
    f.send("null", link(json!(1), URL));
    f.process
        .poll_gateway(
            &client(1),
            "binding",
            |_| async { panic!("no confirmation") },
            |_| panic!("no open"),
            || false,
        )
        .await
        .unwrap();
    let replies = f.replies();
    assert!(replies[0]["result"]["hostCapabilities"].get("openLinks").is_none());
    assert_eq!(replies[1]["error"]["message"], "External link origin is not allowed");
}

#[test]
fn mcp_app_open_link_rejects_expired_confirmation_and_reports_open_failure() {
    let now = tokio::time::Instant::now();
    let expired = resolve_link(json!("expired"), URL, now, Some(true), |_| {
        panic!("late approval must not open")
    });
    assert_eq!(expired["id"], "expired");
    assert_eq!(expired["error"]["message"], "External link confirmation expired");
    let unavailable =
        resolve_link(json!(1), URL, now + Duration::from_secs(60), Some(true), |_| false);
    assert_eq!(unavailable["id"], 1);
    assert_eq!(unavailable["error"]["message"], "System browser could not open external link");
}

#[test]
fn mcp_app_open_link_denies_other_sessions_and_closed_bridge() {
    let mut bridge =
        McpAppBridge::new("session".into(), "server".into(), "null".into(), BTreeSet::new())
            .unwrap();
    bridge.redirect_domains = policy().redirect_domains;
    for request in [initialize(), initialized()] {
        assert!(
            bridge
                .admit(
                    "session",
                    "server",
                    "null",
                    &request.to_string(),
                    true,
                    |_, _, _, _| unreachable!()
                )
                .is_ok()
        );
    }
    let request = link(json!(1), URL).to_string();
    assert!(
        bridge
            .admit("other", "server", "null", &request, true, |_, _, _, _| unreachable!())
            .is_err()
    );
    bridge.close();
    assert!(
        bridge
            .admit("session", "server", "null", &request, true, |_, _, _, _| unreachable!())
            .is_err()
    );
}

#[tokio::test]
#[ignore = "requires a built WebView helper and a graphical desktop"]
async fn mcp_app_open_link_real_helper_roundtrip() {
    use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
    let helper = std::env::var_os("VW_MCP_LINK_HELPER").expect("VW_MCP_LINK_HELPER is required");
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let client = client(listener.local_addr().unwrap().port());
    let gateway = tokio::spawn(async move {
        let (socket, _) = listener.accept().await.unwrap();
        let mut reader = BufReader::new(socket);
        let mut length = 0;
        loop {
            let mut line = String::new();
            assert_ne!(reader.read_line(&mut line).await.unwrap(), 0);
            if line == "\r\n" {
                break;
            }
            if let Some((name, value)) = line.split_once(':')
                && name.eq_ignore_ascii_case("content-length")
            {
                length = value.trim().parse().unwrap();
            }
        }
        let mut body = vec![0; length];
        reader.read_exact(&mut body).await.unwrap();
        let request: Value = serde_json::from_slice(&body).unwrap();
        reader.get_mut().write_all(b"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: 2\r\nConnection: close\r\n\r\n{}").await.unwrap();
        request
    });

    let html = r#"<meta charset="utf-8"><p>MCP 外部链接测试</p>
        <script>
        const send = (method, id, params) => parent.postMessage({jsonrpc: '2.0', id, method, params}, '*');
        addEventListener('message', e => {
            if (e.source !== parent) return;
            const msg = e.data;
            if (msg.id === 'init' && msg.result?.hostCapabilities?.openLinks) {
                send('ui/notifications/initialized');
                send('ui/open-link', 'approved', {url: 'https://links.example.test:8443/report?q=demo#section'});
            }
            if (msg.id === 'approved' && msg.result && Object.keys(msg.result).length === 0) {
                send('ui/open-link', 2, {url: 'https://links.example.test:8443/cancel'});
            }
            if (msg.id === 2 && msg.error?.code === -32000) {
                window.open('https://links.example.test:8443/from-popup', '_blank');
                send('tools/call', 3, {name: 'proof', arguments: {confirmed: true, cancelled: true}});
            }
        });
        send('ui/initialize', 'init');
        </script>"#;
    let mut process = McpAppProcess::launch_with_policy(
        std::path::Path::new(&helper),
        "session".into(),
        "server".into(),
        html,
        BTreeSet::from(["proof".into()]),
        policy(),
    )
    .unwrap();
    let mut decisions = 0;
    let mut opened = vec![];
    tokio::time::timeout(Duration::from_secs(20), async {
        while !gateway.is_finished() {
            process
                .poll_gateway(
                    &client,
                    "binding",
                    |origin| {
                        assert_eq!(origin, ORIGIN);
                        decisions += 1;
                        ready(decisions != 2)
                    },
                    |url| {
                        opened.push(url.to_owned());
                        true
                    },
                    || false,
                )
                .await
                .unwrap();
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    })
    .await
    .expect("real WebView must receive both replies and call the existing tool bridge");
    process.close().unwrap();
    assert_eq!(opened, [URL, "https://links.example.test:8443/from-popup"]);
    assert_eq!(
        gateway.await.unwrap(),
        json!({
            "session":"session", "server":"server", "binding":"binding", "name":"proof",
            "arguments":{"confirmed":true,"cancelled":true}
        })
    );
}
