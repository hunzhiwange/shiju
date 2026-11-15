//! JSON-RPC transport evidence for mount request parsing; not native skill acceptance.
use serde_json::{Value, json};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

use super::agent_tests::FakeBackend;

async fn exchange_with(backend: impl super::SessionBackend, requests: Vec<Value>) -> Vec<Value> {
    let (client, server) = tokio::io::duplex(64 * 1024);
    let (reader, writer) = tokio::io::split(server);
    let service = super::serve_with_io(backend, reader, writer);
    let client = async move {
        let (reader, mut writer) = tokio::io::split(client);
        let mut reader = BufReader::new(reader);
        let mut results = Vec::new();
        for request in requests {
            writer.write_all(format!("{request}\n").as_bytes()).await.unwrap();
            let mut line = String::new();
            reader.read_line(&mut line).await.unwrap();
            results.push(serde_json::from_str(&line).unwrap());
        }
        writer.shutdown().await.unwrap();
        results
    };
    let (result, frames) = tokio::join!(service, client);
    result.unwrap();
    frames
}

fn initialize() -> Value {
    json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":1,"clientCapabilities":{}}})
}

fn new_session(cwd: &std::path::Path, selection: Value) -> Value {
    json!({"jsonrpc":"2.0","id":2,"method":"session/new","params":{"cwd":cwd,"mcpServers":[],"_meta":{"shiju.skillMounts":selection}}})
}

#[tokio::test(flavor = "current_thread")]
async fn acp_server_mount_request_invalid_version_is_rejected() {
    let root = tempfile::tempdir().unwrap();
    let frames =
        exchange(vec![initialize(), new_session(root.path(), json!({"version":2,"skills":[]}))])
            .await;
    assert_eq!(frames[1]["error"]["code"], -32602);
}

#[tokio::test(flavor = "current_thread")]
async fn acp_server_mount_request_nonempty_unsupported_is_rejected() {
    let root = tempfile::tempdir().unwrap();
    let frames = exchange(vec![
        initialize(),
        new_session(
            root.path(),
            json!({"version":1,"skills":[{"name":"example","path":"/unread/skill"}]}),
        ),
    ])
    .await;
    assert_eq!(frames[1]["error"]["code"], -32602);
}

#[tokio::test(flavor = "current_thread")]
async fn acp_server_mount_request_explicit_empty_returns_actual_names() {
    let root = tempfile::tempdir().unwrap();
    let frames =
        exchange(vec![initialize(), new_session(root.path(), json!({"version":1,"skills":[]}))])
            .await;
    assert_eq!(
        frames[1]["result"]["_meta"]["shiju.mounts"],
        json!({"revision":0,"skills":[],"mcpServers":[]})
    );
}

#[tokio::test(flavor = "current_thread")]
async fn acp_server_mount_request_uninitialized_is_rejected() {
    let root = tempfile::tempdir().unwrap();
    let frames = exchange(vec![new_session(root.path(), json!({"version":1,"skills":[]}))]).await;
    assert_eq!(frames[0]["error"]["code"], -32600);
}

async fn exchange(requests: Vec<Value>) -> Vec<Value> {
    exchange_with(FakeBackend::default(), requests).await
}

#[derive(Clone, Default)]
struct MountBackend {
    active: std::sync::Arc<
        std::sync::Mutex<Option<tokio::sync::mpsc::UnboundedSender<super::BackendEvent>>>,
    >,
    requests: std::sync::Arc<std::sync::Mutex<Vec<super::BackendMountRequest>>>,
    saved: std::sync::Arc<std::sync::Mutex<Vec<crate::session_mounts::SessionSkillMount>>>,
    cwd: std::sync::Arc<std::sync::Mutex<Option<std::path::PathBuf>>>,
}

#[async_trait::async_trait]
impl super::SessionBackend for MountBackend {
    async fn start_prompt(
        &self,
        _: &str,
        _: Vec<super::PromptContent>,
    ) -> Result<super::BackendPrompt, super::BackendError> {
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
        *self.active.lock().unwrap() = Some(tx);
        Ok(super::BackendPrompt::new(rx, std::sync::Arc::new(NoCancel)))
    }

    async fn create_session(
        &self,
        _: std::path::PathBuf,
    ) -> Result<super::BackendSession, super::BackendError> {
        panic!("mount input must reach the typed backend entry")
    }
    async fn load_session(
        &self,
        _: &str,
        _: std::path::PathBuf,
    ) -> Result<super::BackendSession, super::BackendError> {
        panic!("mount input must reach the typed backend entry")
    }
    async fn create_session_with_mounts(
        &self,
        cwd: std::path::PathBuf,
        request: super::BackendMountRequest,
    ) -> Result<super::BackendMountedSession, super::BackendError> {
        *self.cwd.lock().unwrap() = Some(cwd.clone());
        self.apply(cwd, request)
    }
    async fn load_session_with_mounts(
        &self,
        id: &str,
        cwd: std::path::PathBuf,
        request: super::BackendMountRequest,
    ) -> Result<super::BackendMountedSession, super::BackendError> {
        if id != "owned" || self.cwd.lock().unwrap().as_ref() != Some(&cwd) {
            return Err(super::BackendError::OwnershipMismatch(id.into()));
        }
        self.apply(cwd, request)
    }
}

impl MountBackend {
    fn apply(
        &self,
        cwd: std::path::PathBuf,
        request: super::BackendMountRequest,
    ) -> Result<super::BackendMountedSession, super::BackendError> {
        let mut saved = self.saved.lock().unwrap();
        *saved = crate::session_mounts::SessionMountSelection::resolve(
            request.skills.as_ref(),
            Some(&saved),
        );
        let mut mounts = request.legacy_response()?;
        mounts.skills = saved.iter().map(|skill| skill.name.clone()).collect();
        mounts.revision = crate::session_mounts::SessionMountRevision(7);
        self.requests.lock().unwrap().push(request);
        Ok(super::BackendMountedSession {
            session: super::BackendSession { id: "owned".into(), cwd, history: Vec::new() },
            mounts,
        })
    }
}

fn load(cwd: &std::path::Path, id: &str) -> Value {
    json!({"jsonrpc":"2.0","id":3,"method":"session/load","params":{"cwd":cwd,"sessionId":id,"mcpServers":[]}})
}

#[tokio::test(flavor = "current_thread")]
async fn acp_server_mount_request_roundtrip_preserves_selection_mcp_and_restore_vs_empty() {
    let root = tempfile::tempdir().unwrap();
    let backend = MountBackend::default();
    let mut new = new_session(
        root.path(),
        json!({"version":1,"skills":[{"name":"example","path":"/unread/skill"}]}),
    );
    new["params"]["mcpServers"] =
        json!([{"name":"fixture","command":"/unexecuted/mcp","args":["--stdio"],"env":[]}]);
    let mut clear = load(root.path(), "owned");
    clear["params"]["_meta"] = json!({"shiju.skillMounts":{"version":1,"skills":[]}});
    let frames =
        exchange_with(backend.clone(), vec![initialize(), new, load(root.path(), "owned"), clear])
            .await;
    assert_eq!(frames[0]["result"]["_meta"]["shiju.mounts"]["skills"], false);
    assert_eq!(frames[0]["result"]["_meta"]["shiju.mounts"]["setMounts"], false);
    assert_eq!(
        frames[1]["result"]["_meta"]["shiju.mounts"],
        json!({"revision":7,"skills":["example"],"mcpServers":["fixture"]})
    );
    assert_eq!(frames[2]["result"]["_meta"]["shiju.mounts"]["skills"], json!(["example"]));
    assert_eq!(frames[3]["result"]["_meta"]["shiju.mounts"]["skills"], json!([]));
    let requests = backend.requests.lock().unwrap();
    assert_eq!(requests.len(), 3);
    assert_eq!(requests[0].skills.as_ref().unwrap().skills[0].path, "/unread/skill");
    assert_eq!(
        serde_json::to_value(&requests[0].mcp_servers).unwrap()[0]["args"],
        json!(["--stdio"])
    );
    assert!(requests[1].skills.is_none());
    assert!(requests[2].skills.as_ref().unwrap().skills.is_empty());
}

#[tokio::test(flavor = "current_thread")]
async fn acp_server_mount_request_invalid_meta_has_no_backend_side_effects() {
    let root = tempfile::tempdir().unwrap();
    let backend = MountBackend::default();
    let frames = exchange_with(
        backend.clone(),
        vec![
            initialize(),
            new_session(root.path(), Value::Null),
            new_session(root.path(), json!({"version":1,"skills":[],"unknown":true})),
            new_session(
                root.path(),
                json!({"version":1,"skills":[{"name":"example","path":"relative"}]}),
            ),
            new_session(root.path(), json!({"version":2,"skills":[]})),
        ],
    )
    .await;
    for frame in &frames[1..] {
        assert_eq!(frame["error"]["code"], -32602);
    }
    assert!(backend.requests.lock().unwrap().is_empty());
}

#[tokio::test(flavor = "current_thread")]
async fn acp_server_mount_request_foreign_session_and_directory_have_no_side_effects() {
    let root = tempfile::tempdir().unwrap();
    let other = tempfile::tempdir().unwrap();
    let backend = MountBackend::default();
    let frames = exchange_with(
        backend.clone(),
        vec![
            initialize(),
            new_session(root.path(), json!({"version":1,"skills":[]})),
            load(other.path(), "owned"),
            load(root.path(), "foreign"),
        ],
    )
    .await;
    assert_eq!(frames[2]["error"]["code"], -32602);
    assert_eq!(frames[3]["error"]["code"], -32602);
    assert_eq!(backend.requests.lock().unwrap().len(), 1);
}

struct NoCancel;
impl super::BackendCancelHandle for NoCancel {
    fn cancel(&self) -> bool {
        panic!("busy load must not cancel the prompt")
    }
}

#[tokio::test(flavor = "current_thread")]
async fn acp_server_mount_request_busy_load_does_not_prepare_mounts_or_interrupt_prompt() {
    let root = tempfile::tempdir().unwrap();
    let backend = MountBackend::default();
    let observed = backend.clone();
    let (client, server) = tokio::io::duplex(64 * 1024);
    let (reader, writer) = tokio::io::split(server);
    let service = super::serve_with_io(backend, reader, writer);
    let client = async {
        let (reader, mut writer) = tokio::io::split(client);
        let mut reader = BufReader::new(reader);
        for request in [initialize(), new_session(root.path(), json!({"version":1,"skills":[]}))] {
            writer.write_all(format!("{request}\n").as_bytes()).await.unwrap();
            let mut line = String::new();
            reader.read_line(&mut line).await.unwrap();
            let frame: Value = serde_json::from_str(&line).unwrap();
            assert!(frame.get("result").is_some());
        }
        let prompt = json!({"jsonrpc":"2.0","id":4,"method":"session/prompt","params":{"sessionId":"owned","prompt":[{"type":"text","text":"wait"}]}});
        writer.write_all(format!("{prompt}\n").as_bytes()).await.unwrap();
        while observed.active.lock().unwrap().is_none() {
            tokio::task::yield_now().await;
        }
        let mut request = load(root.path(), "owned");
        request["params"]["_meta"] = json!({"shiju.skillMounts":{"version":1,"skills":[{"name":"never-read","path":"/unread/skill"}]}});
        writer.write_all(format!("{request}\n").as_bytes()).await.unwrap();
        let mut line = String::new();
        reader.read_line(&mut line).await.unwrap();
        let frame: Value = serde_json::from_str(&line).unwrap();
        assert_eq!(frame["id"], 3);
        assert_eq!(frame["error"]["code"], -32600);
        assert_eq!(observed.requests.lock().unwrap().len(), 1);
        observed
            .active
            .lock()
            .unwrap()
            .take()
            .unwrap()
            .send(super::BackendEvent::Terminal(super::BackendTermination::EndTurn))
            .unwrap();
        line.clear();
        reader.read_line(&mut line).await.unwrap();
        let frame: Value = serde_json::from_str(&line).unwrap();
        assert_eq!(frame["id"], 4);
        assert_eq!(frame["result"]["stopReason"], "end_turn");
        writer.shutdown().await.unwrap();
    };
    tokio::time::timeout(std::time::Duration::from_secs(5), async {
        let (result, ()) = tokio::join!(service, client);
        result.unwrap();
    })
    .await
    .unwrap();
}
