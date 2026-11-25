use std::future::Future;

use serde_json::{Value, json};
use tokio::{
    io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader, DuplexStream},
    net::TcpListener,
    sync::oneshot,
    task::JoinHandle,
};

use super::*;

const CLOUD: &str = "0b56ed16-f194-4c8c-8972-790948760266";
const EMPLOYEE: &str = "fixture-employee";
const ACCOUNT: &str = "fixture-account";
const KEY: &str = "fixture-credential";
const AT: &str = "2026-11-21T00:00:00Z";
const BASE: &str = "/sj/agent/api/v1";

fn uuid(n: usize) -> String {
    format!("00000000-0000-4000-8000-{n:012}")
}

fn conversation(kind: EmployeeKind) -> Value {
    json!({
        "conversation_id": CLOUD,
        "assistant_id": EMPLOYEE,
        "active_version_id": "version",
        "title": "Fixture",
        "kind": match kind { EmployeeKind::Private => "PRIVATE_DIRECT", EmployeeKind::Shared => "SHARED_DIRECT" },
        "status": "ACTIVE",
        "created_at": AT,
        "updated_at": AT
    })
}

fn message(n: usize, employee: bool) -> Value {
    json!({
        "message_id": uuid(n * 10 + if employee { 2 } else { 1 }),
        "sender": if employee { "employee" } else { "user" },
        "body": if employee { format!("answer {n}") } else { format!("question {n}") },
        "reply_to_message_id": employee.then(|| uuid(n * 10 + 1)),
        "created_at": AT
    })
}

fn turn(n: usize, status: &str) -> Value {
    json!({
        "turn_id": uuid(n * 10 + 3),
        "user_message_id": uuid(n * 10 + 1),
        "employee_version_id": "version",
        "status": status,
        "created_at": AT,
        "updated_at": AT
    })
}

fn submission(kind: EmployeeKind, n: usize, status: &str, idempotent: bool) -> Value {
    json!({
        "conversation": conversation(kind),
        "message": message(n, false),
        "turn": turn(n, status),
        "idempotent": idempotent
    })
}

fn history(turns: &[&str]) -> Value {
    let mut messages = Vec::new();
    let mut results = Vec::new();
    for (index, status) in turns.iter().enumerate() {
        let n = index + 1;
        messages.push(message(n, false));
        if *status == "COMPLETED" {
            messages.push(message(n, true));
        }
        results.push(turn(n, status));
    }
    json!({ "messages": messages, "turns": results })
}

fn collection(kind: EmployeeKind) -> String {
    let group = match kind {
        EmployeeKind::Private => "assistants",
        EmployeeKind::Shared => "employees",
    };
    format!("{BASE}/{group}/{EMPLOYEE}/conversations")
}

fn messages() -> String {
    format!("{BASE}/conversations/{CLOUD}/messages")
}

fn cancel() -> String {
    format!("{BASE}/conversations/{CLOUD}/turns/{}/cancel", uuid(13))
}

struct Exchange {
    request: String,
    response: Option<(u16, Value)>,
    received: Option<oneshot::Sender<()>>,
    release: Option<oneshot::Receiver<()>>,
}

impl Exchange {
    fn new(method: &str, path: String, status: u16, body: Value) -> Self {
        Self {
            request: format!("{method} {path} HTTP/1.1"),
            response: Some((status, body)),
            received: None,
            release: None,
        }
    }

    fn gated(mut self) -> (Self, oneshot::Receiver<()>, oneshot::Sender<()>) {
        let (received, request) = oneshot::channel();
        let (release, gate) = oneshot::channel();
        self.received = Some(received);
        self.release = Some(gate);
        (self, request, release)
    }
}

#[derive(Debug)]
struct Request {
    key: Option<String>,
    body: Vec<u8>,
}

async fn fixture(
    exchanges: Vec<Exchange>,
) -> (EmployeeConversationClient, JoinHandle<Vec<Request>>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let credential = ShijuCredential::new(KEY).unwrap();
    let client =
        EmployeeConversationClient::with_loopback(listener.local_addr().unwrap(), &credential)
            .unwrap();
    let server = tokio::spawn(async move {
        let mut requests = Vec::new();
        for exchange in exchanges {
            let (stream, _) = listener.accept().await.unwrap();
            let mut stream = BufReader::new(stream);
            let mut line = String::new();
            stream.read_line(&mut line).await.unwrap();
            assert_eq!(line.trim_end(), exchange.request);
            let mut key = None;
            let mut auth = None;
            let mut length = 0;
            loop {
                line.clear();
                assert_ne!(stream.read_line(&mut line).await.unwrap(), 0);
                if line == "\r\n" {
                    break;
                }
                let (header, value) = line.trim_end().split_once(':').unwrap();
                match header.to_ascii_lowercase().as_str() {
                    "x-api-key" => auth = Some(value.trim().to_owned()),
                    "idempotency-key" => key = Some(value.trim().to_owned()),
                    "content-length" => length = value.trim().parse::<usize>().unwrap(),
                    _ => {}
                }
            }
            assert_eq!(auth.as_deref(), Some(KEY));
            let mut body = vec![0; length];
            stream.read_exact(&mut body).await.unwrap();
            requests.push(Request { key, body });
            if let Some(received) = exchange.received {
                received.send(()).unwrap();
            }
            if let Some(release) = exchange.release {
                release.await.unwrap();
            }
            if let Some((status, body)) = exchange.response {
                let body = body.to_string();
                let header = format!(
                    "HTTP/1.1 {status} Fixture\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    body.len()
                );
                if stream.get_mut().write_all(header.as_bytes()).await.is_ok() {
                    let _ = stream.get_mut().write_all(body.as_bytes()).await;
                }
            }
        }
        requests
    });
    (client, server)
}

fn agent(
    client: &EmployeeConversationClient,
    root: &Path,
    kind: EmployeeKind,
    account: &str,
) -> EmployeeAgent {
    EmployeeAgent::new(
        client.clone(),
        EmployeeTarget { employee_id: EMPLOYEE.into(), kind },
        root,
        account,
    )
    .unwrap()
}

struct Rpc(BufReader<DuplexStream>);

impl Rpc {
    async fn send(&mut self, method: &str, id: Option<u32>, params: Value) {
        let mut message = json!({ "jsonrpc": "2.0", "method": method, "params": params });
        if let Some(id) = id {
            message["id"] = json!(id);
        }
        self.0.get_mut().write_all(format!("{message}\n").as_bytes()).await.unwrap();
    }

    async fn read(&mut self) -> Value {
        let mut line = String::new();
        assert_ne!(self.0.read_line(&mut line).await.unwrap(), 0, "unexpected ACP EOF");
        serde_json::from_str(&line).unwrap()
    }

    async fn initialize(&mut self) {
        self.send("initialize", Some(1), json!({ "protocolVersion": 1 })).await;
        let initialized = self.read().await;
        assert_eq!(initialized["result"]["agentCapabilities"]["loadSession"], true);
        assert_eq!(
            initialized["result"]["agentCapabilities"]["sessionCapabilities"]["list"],
            json!({})
        );
    }

    async fn new_session(&mut self, cwd: &Path) -> String {
        self.send("session/new", Some(2), json!({ "cwd": cwd, "mcpServers": [] })).await;
        self.read().await["result"]["sessionId"].as_str().expect("local session").to_owned()
    }

    async fn prompt(&mut self, id: &str, n: usize) {
        self.send(
            "session/prompt",
            Some(3),
            json!({
                "sessionId": id, "prompt": [{ "type": "text", "text": format!("question {n}") }]
            }),
        )
        .await;
    }

    async fn reply(&mut self, id: &str, n: usize) {
        let update = self.read().await;
        assert_eq!(update["method"], "session/update");
        assert_eq!(update["params"]["sessionId"], id);
        assert_eq!(update["params"]["update"]["sessionUpdate"], "agent_message_chunk");
        assert_eq!(update["params"]["update"]["content"]["text"], format!("answer {n}"));
        let result = self.read().await;
        assert_eq!(result["id"], 3);
        assert_eq!(result["result"]["stopReason"], "end_turn");
    }
}

async fn session<T, F: Future<Output = T>>(
    agent: EmployeeAgent,
    client: impl FnOnce(Rpc) -> F,
) -> T {
    let (client_io, server_io) = tokio::io::duplex(32 * 1024);
    let (reader, writer) = tokio::io::split(server_io);
    let (server, result) = time::timeout(Duration::from_secs(8), async {
        tokio::join!(agent.serve_with_io(reader, writer), client(Rpc(BufReader::new(client_io))))
    })
    .await
    .expect("fixture timed out");
    server.expect("stdio transport");
    result
}

#[tokio::test]
async fn employee_probe_handshakes_without_allowing_sessions() {
    let (client_io, server_io) = tokio::io::duplex(4096);
    let (reader, writer) = tokio::io::split(server_io);
    let (result, ()) = time::timeout(Duration::from_secs(5), async {
        tokio::join!(serve_initialize_probe(reader, writer), async {
            let mut client = Rpc(BufReader::new(client_io));
            client.initialize().await;
            client.send("session/new", Some(2), json!({ "cwd": "/", "mcpServers": [] })).await;
            assert_eq!(client.read().await["error"]["code"], -32601);
        })
    })
    .await
    .expect("probe timed out");
    result.expect("stdio transport");
}

async fn roundtrip(kind: EmployeeKind) {
    let (client, http) = fixture(vec![
        Exchange::new("POST", collection(kind), 201, submission(kind, 1, "DELIVERED", false)),
        Exchange::new("GET", messages(), 200, history(&["COMPLETED"])),
        Exchange::new("POST", messages(), 201, submission(kind, 2, "DELIVERED", false)),
        Exchange::new("GET", messages(), 200, history(&["COMPLETED", "COMPLETED"])),
        Exchange::new("GET", collection(kind), 200, json!({ "items": [conversation(kind)] })),
        Exchange::new("GET", messages(), 200, history(&["COMPLETED", "COMPLETED"])),
        Exchange::new("GET", collection(kind), 200, json!({ "items": [conversation(kind)] })),
    ])
    .await;
    let root_dir = tempfile::tempdir().unwrap();
    let root = root_dir.path();
    let cwd_dir = tempfile::tempdir().unwrap();
    let cwd = cwd_dir.path();
    let id = session(agent(&client, root, kind, ACCOUNT), |mut rpc| async move {
        rpc.initialize().await;
        let id = rpc.new_session(cwd).await;
        rpc.prompt(&id, 1).await;
        rpc.reply(&id, 1).await;
        rpc.prompt(&id, 2).await;
        rpc.reply(&id, 2).await;
        id
    })
    .await;

    let reopen_id = id.clone();
    session(agent(&client, root, kind, ACCOUNT), |mut rpc| async move {
        let id = reopen_id;
        rpc.initialize().await;
        rpc.send("session/load", Some(4), json!({ "sessionId": id, "cwd": cwd, "mcpServers": [] }))
            .await;
        for (i, text) in
            ["question 1", "answer 1", "question 2", "answer 2"].into_iter().enumerate()
        {
            let frame = rpc.read().await;
            assert_eq!(
                frame["params"]["update"]["sessionUpdate"],
                if i % 2 == 0 { "user_message_chunk" } else { "agent_message_chunk" }
            );
            assert_eq!(frame["params"]["update"]["content"]["text"], text);
        }
        assert_eq!(rpc.read().await["result"], json!({}));
        rpc.send("session/list", Some(5), json!({ "cwd": cwd })).await;
        assert_eq!(rpc.read().await["result"]["sessions"][0]["sessionId"], id);
    })
    .await;

    session(agent(&client, root, kind, "other-account"), |mut rpc| async move {
        rpc.initialize().await;
        rpc.send("session/load", Some(6), json!({ "sessionId": id, "cwd": cwd, "mcpServers": [] }))
            .await;
        assert_eq!(rpc.read().await["error"]["code"], -32602);
    })
    .await;

    let requests = http.await.unwrap();
    assert!(requests[0].key.is_some());
    assert_ne!(requests[0].key, requests[2].key);
}

#[tokio::test]
async fn employee_acp_private_two_turns_and_reopen() {
    roundtrip(EmployeeKind::Private).await;
}

#[tokio::test]
async fn employee_acp_shared_two_turns_and_reopen() {
    roundtrip(EmployeeKind::Shared).await;
}

#[tokio::test]
async fn employee_acp_streams_partial_reply_before_completion() {
    let kind = EmployeeKind::Shared;
    let mut first = history(&["RESPONDING"]);
    first["turns"][0]["partial_reply"] = json!("ans");
    let mut second = first.clone();
    second["turns"][0]["partial_reply"] = json!("answer");
    let (complete, requested, release) =
        Exchange::new("GET", messages(), 200, history(&["COMPLETED"])).gated();
    let (client, http) = fixture(vec![
        Exchange::new("POST", collection(kind), 201, submission(kind, 1, "DELIVERED", false)),
        Exchange::new("GET", messages(), 200, first.clone()),
        Exchange::new("GET", messages(), 200, first),
        Exchange::new("GET", messages(), 200, second),
        complete,
    ])
    .await;
    let root = tempfile::tempdir().unwrap();
    let cwd = tempfile::tempdir().unwrap();
    session(agent(&client, root.path(), kind, ACCOUNT), |mut rpc| async move {
        rpc.initialize().await;
        let id = rpc.new_session(cwd.path()).await;
        rpc.prompt(&id, 1).await;
        let partial = rpc.read().await;
        assert_eq!(partial["params"]["sessionId"], id);
        assert_eq!(partial["params"]["update"]["sessionUpdate"], "agent_message_chunk");
        assert_eq!(partial["params"]["update"]["content"]["text"], "ans");
        assert_eq!(rpc.read().await["params"]["update"]["content"]["text"], "wer");
        requested.await.unwrap();
        release.send(()).unwrap();
        let suffix = rpc.read().await;
        assert_eq!(suffix["params"]["update"]["content"]["text"], " 1");
        assert_eq!(rpc.read().await["result"]["stopReason"], "end_turn");
    })
    .await;
    assert_eq!(http.await.unwrap().len(), 5);
}

#[tokio::test]
async fn employee_acp_lost_post_reuses_key_after_restart() {
    let kind = EmployeeKind::Private;
    let mut lost = Exchange::new("POST", collection(kind), 201, Value::Null);
    lost.response = None;
    let (client, http) = fixture(vec![
        lost,
        Exchange::new("GET", collection(kind), 200, json!({ "items": [conversation(kind)] })),
        Exchange::new("POST", collection(kind), 201, submission(kind, 1, "COMPLETED", true)),
        Exchange::new("GET", messages(), 200, history(&["COMPLETED"])),
    ])
    .await;
    let root_dir = tempfile::tempdir().unwrap();
    let root = root_dir.path();
    let id = session(agent(&client, root, kind, ACCOUNT), |mut rpc| async move {
        rpc.initialize().await;
        let id = rpc.new_session(root).await;
        rpc.prompt(&id, 1).await;
        assert!(rpc.read().await["error"]["message"].as_str().unwrap().contains("unconfirmed"));
        id
    })
    .await;
    session(agent(&client, root, kind, ACCOUNT), |mut rpc| async move {
        rpc.initialize().await;
        rpc.send(
            "session/load",
            Some(4),
            json!({ "sessionId": id, "cwd": root, "mcpServers": [] }),
        )
        .await;
        assert_eq!(rpc.read().await["result"], json!({}));
        rpc.prompt(&id, 2).await;
        assert!(rpc.read().await["error"]["message"].as_str().unwrap().contains("unconfirmed"));
        rpc.prompt(&id, 1).await;
        rpc.reply(&id, 1).await;
    })
    .await;
    let requests = http.await.unwrap();
    assert!(requests[0].key.is_some());
    assert_eq!(requests[0].key, requests[2].key);
    assert_eq!(requests[0].body, requests[2].body);
}

async fn cancellation(completion_wins: bool) {
    let kind = EmployeeKind::Shared;
    let (post, posted, release) =
        Exchange::new("POST", collection(kind), 201, submission(kind, 1, "DELIVERED", false))
            .gated();
    let status = if completion_wins { "COMPLETED" } else { "CANCELLED" };
    let mut exchanges = vec![post, Exchange::new("POST", cancel(), 200, turn(1, status))];
    if completion_wins {
        exchanges.push(Exchange::new("GET", messages(), 200, history(&[status])));
    }
    exchanges.extend([
        Exchange::new("POST", messages(), 201, submission(kind, 2, "DELIVERED", false)),
        Exchange::new("GET", messages(), 200, history(&[status, "COMPLETED"])),
    ]);
    let (client, http) = fixture(exchanges).await;
    let root_dir = tempfile::tempdir().unwrap();
    let root = root_dir.path();
    let employee = agent(&client, root, kind, ACCOUNT);
    let store = StateStore::open(root, ACCOUNT, &employee.target).unwrap();
    session(employee, |mut rpc| async move {
        rpc.initialize().await;
        let id = rpc.new_session(root).await;
        rpc.prompt(&id, 1).await;
        posted.await.unwrap();
        assert_eq!(i32::from(store.lease(&id).unwrap_err().code), -32600);
        rpc.send("session/cancel", None, json!({ "sessionId": id })).await;
        while !store.read(&id).unwrap().unwrap().pending.unwrap().cancel_requested {
            time::sleep(Duration::from_millis(5)).await;
        }
        release.send(()).unwrap();
        if completion_wins {
            rpc.reply(&id, 1).await;
        } else {
            let result = rpc.read().await;
            assert_eq!(result["id"], 3);
            assert_eq!(result["result"]["stopReason"], "cancelled");
        }
        rpc.prompt(&id, 2).await;
        rpc.reply(&id, 2).await;
    })
    .await;
    http.await.unwrap();
}

#[tokio::test]
async fn employee_acp_cancel_during_submit_then_next_turn() {
    cancellation(false).await;
}

#[tokio::test]
async fn employee_acp_confirmed_completion_wins_cancel_race() {
    cancellation(true).await;
}

#[tokio::test]
async fn employee_acp_eof_preserves_confirmed_turn_without_cancelling() {
    let kind = EmployeeKind::Private;
    let (poll, polled, release) =
        Exchange::new("GET", messages(), 200, history(&["DELIVERED"])).gated();
    let (client, http) = fixture(vec![
        Exchange::new("POST", collection(kind), 201, submission(kind, 1, "DELIVERED", false)),
        poll,
        Exchange::new("GET", messages(), 200, history(&["COMPLETED"])),
    ])
    .await;
    let root_dir = tempfile::tempdir().unwrap();
    let root = root_dir.path();
    let id = session(agent(&client, root, kind, ACCOUNT), |mut rpc| async move {
        rpc.initialize().await;
        let id = rpc.new_session(root).await;
        rpc.prompt(&id, 1).await;
        polled.await.unwrap();
        id
    })
    .await;
    release.send(()).unwrap();
    session(agent(&client, root, kind, ACCOUNT), |mut rpc| async move {
        rpc.initialize().await;
        rpc.prompt(&id, 1).await;
        rpc.reply(&id, 1).await;
    })
    .await;
    assert_eq!(http.await.unwrap().iter().filter(|request| request.key.is_some()).count(), 1);
}

#[tokio::test]
async fn employee_acp_list_cursor_is_bound_to_account_target_and_cwd() {
    let kind = EmployeeKind::Private;
    let mut items = Vec::new();
    for n in 0..17 {
        let mut item = conversation(kind);
        item["conversation_id"] = json!(uuid(n + 100));
        items.push(item);
    }
    let visible = json!({ "items": items });
    let (client, http) = fixture(vec![
        Exchange::new("GET", collection(kind), 200, visible.clone()),
        Exchange::new("GET", collection(kind), 200, visible),
    ])
    .await;
    let root = tempfile::tempdir().unwrap();
    let directory = tempfile::tempdir().unwrap();
    let directory = directory.path();
    let cwd = std::env::current_dir().unwrap();
    let cwd = cwd.as_path();
    let cursor = session(agent(&client, root.path(), kind, ACCOUNT), |mut rpc| async move {
        rpc.initialize().await;
        rpc.send("session/list", Some(5), json!({ "cwd": cwd })).await;
        let result = rpc.read().await;
        assert_eq!(result["result"]["sessions"].as_array().unwrap().len(), 16);
        let cursor = result["result"]["nextCursor"].as_str().unwrap().to_owned();
        rpc.send("session/list", Some(5), json!({ "cwd": directory, "cursor": cursor })).await;
        assert_eq!(rpc.read().await["error"]["code"], -32602);
        rpc.send("session/list", Some(5), json!({ "cwd": cwd, "cursor": cursor })).await;
        let page = rpc.read().await;
        assert_eq!(page["result"]["sessions"].as_array().unwrap().len(), 1);
        assert_eq!(page["result"]["sessions"][0]["sessionId"], uuid(116));
        assert!(page["result"]["nextCursor"].is_null());
        cursor
    })
    .await;
    let cursor = cursor.as_str();
    for (kind, account) in [(kind, "other-account"), (EmployeeKind::Shared, ACCOUNT)] {
        session(agent(&client, root.path(), kind, account), |mut rpc| async move {
            rpc.initialize().await;
            rpc.send("session/list", Some(5), json!({ "cwd": cwd, "cursor": cursor })).await;
            assert_eq!(rpc.read().await["error"]["code"], -32602);
        })
        .await;
    }
    http.await.unwrap();
}

#[tokio::test]
async fn employee_acp_rejects_mcp_non_text_and_oversize_without_http() {
    let (client, http) = fixture(vec![]).await;
    let root_dir = tempfile::tempdir().unwrap();
    let root = root_dir.path();
    session(agent(&client, root, EmployeeKind::Private, ACCOUNT), |mut rpc| async move {
        rpc.initialize().await;
        rpc.send(
            "session/new",
            Some(2),
            json!({ "cwd": root, "mcpServers": [
            { "name": "fixture", "command": "fixture", "args": [], "env": [] }
        ] }),
        )
        .await;
        assert_eq!(rpc.read().await["error"]["code"], -32602);
        let id = rpc.new_session(root).await;
        for block in [
            json!({ "type": "image", "mimeType": "image/png", "data": "AA==" }),
            json!({ "type": "text", "text": "字".repeat(10_923) }),
        ] {
            rpc.send("session/prompt", Some(3), json!({ "sessionId": id, "prompt": [block] }))
                .await;
            assert_eq!(rpc.read().await["error"]["code"], -32602);
        }
    })
    .await;
    assert!(http.await.unwrap().is_empty());
}
