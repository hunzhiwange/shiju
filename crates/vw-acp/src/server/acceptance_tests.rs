//! 验证 ACP v1 标准输入输出流程、历史回放和能力拒绝契约。
//!
//! 内存传输和伪后端 fixture 用于从协议边界观察服务端行为。

use std::path::PathBuf;
use std::sync::Arc;

use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

use super::{
    BackendCancelHandle, BackendError, BackendEvent, BackendHistoryEntry, BackendPermissionOption,
    BackendPermissionOptionKind, BackendPermissionRequest, BackendPrompt, BackendRole,
    BackendSession, PromptContent, SessionBackend,
};

#[derive(Clone)]
struct AcceptanceBackend;

struct CancelToTerminal {
    events: tokio::sync::mpsc::UnboundedSender<BackendEvent>,
    _permission: tokio::sync::oneshot::Receiver<Option<String>>,
}

impl BackendCancelHandle for CancelToTerminal {
    fn cancel(&self) -> bool {
        self.events.send(BackendEvent::Terminal(super::BackendTermination::Cancelled)).is_ok()
    }
}

#[async_trait::async_trait]
impl SessionBackend for AcceptanceBackend {
    async fn create_session(&self, cwd: PathBuf) -> Result<BackendSession, BackendError> {
        Ok(BackendSession { id: "session-1".to_string(), cwd, history: Vec::new() })
    }

    async fn load_session(
        &self,
        session_id: &str,
        cwd: PathBuf,
    ) -> Result<BackendSession, BackendError> {
        Ok(BackendSession {
            id: session_id.to_string(),
            cwd,
            history: vec![
                BackendHistoryEntry { role: BackendRole::User, text: "previous question".into() },
                BackendHistoryEntry {
                    role: BackendRole::Assistant,
                    text: "previous answer".into(),
                },
            ],
        })
    }

    async fn start_prompt(
        &self,
        _session_id: &str,
        _prompt: Vec<PromptContent>,
    ) -> Result<BackendPrompt, BackendError> {
        let (events_tx, events_rx) = tokio::sync::mpsc::unbounded_channel();
        events_tx.send(BackendEvent::Text("working".into())).expect("text event");
        events_tx
            .send(BackendEvent::ToolStart {
                call_id: "call-1".into(),
                title: "fixture".into(),
                raw_input: serde_json::json!({"safe": true}),
            })
            .expect("tool event");
        let (permissions_tx, permissions_rx) = tokio::sync::mpsc::unbounded_channel();
        let (response, selected) = tokio::sync::oneshot::channel();
        permissions_tx
            .send(BackendPermissionRequest {
                request_id: "approval-1".into(),
                call_id: "call-1".into(),
                title: "fixture".into(),
                raw_input: serde_json::json!({"safe": true}),
                options: vec![
                    BackendPermissionOption {
                        id: "allow_once".into(),
                        name: "Allow once".into(),
                        kind: BackendPermissionOptionKind::AllowOnce,
                    },
                    BackendPermissionOption {
                        id: "reject_once".into(),
                        name: "Reject".into(),
                        kind: BackendPermissionOptionKind::RejectOnce,
                    },
                ],
                response,
            })
            .expect("permission event");
        drop(permissions_tx);
        Ok(BackendPrompt::new(
            events_rx,
            Arc::new(CancelToTerminal { events: events_tx, _permission: selected }),
        )
        .with_permissions(permissions_rx))
    }
}

pub(super) async fn write_frame(stream: &mut tokio::io::DuplexStream, frame: serde_json::Value) {
    let mut bytes = serde_json::to_vec(&frame).expect("serialize frame");
    bytes.push(b'\n');
    stream.write_all(&bytes).await.expect("write frame");
}

pub(super) async fn read_frame(
    reader: &mut BufReader<&mut tokio::io::DuplexStream>,
) -> serde_json::Value {
    let mut line = String::new();
    reader.read_line(&mut line).await.expect("read frame");
    assert!(!line.is_empty(), "protocol stream closed before expected frame");
    serde_json::from_str(&line).expect("JSON-RPC frame")
}

// 下列行为用例覆盖能力声明、会话隔离、并发拒绝和传输输出。
#[tokio::test(flavor = "current_thread")]
async fn acp_v1_stdio_acceptance_flow_matches_advertised_capabilities() {
    let root = tempfile::tempdir().expect("temporary cwd");
    let backend = AcceptanceBackend;
    let (mut client, server) = tokio::io::duplex(64 * 1024);
    let (server_read, server_write) = tokio::io::split(server);
    let server_task = super::serve_with_io(backend, server_read, server_write);
    let cwd = root.path().to_string_lossy().into_owned();
    let client_task = async move {
        write_frame(
            &mut client,
            serde_json::json!({
                "jsonrpc":"2.0","id":1,"method":"initialize",
                "params":{"protocolVersion":1,"clientCapabilities":{}}
            }),
        )
        .await;
        let mut reader = BufReader::new(&mut client);
        let initialized = read_frame(&mut reader).await;
        assert_eq!(initialized["result"]["protocolVersion"], 1);
        assert_eq!(initialized["result"]["agentCapabilities"]["loadSession"], true);
        assert_eq!(initialized["result"]["agentCapabilities"]["mcpCapabilities"]["http"], true);
        assert_eq!(initialized["result"]["agentCapabilities"]["mcpCapabilities"]["sse"], true);

        write_frame(
            reader.get_mut(),
            serde_json::json!({
                "jsonrpc":"2.0","id":2,"method":"session/new",
                "params":{"cwd":cwd,"mcpServers":[]}
            }),
        )
        .await;
        let created = read_frame(&mut reader).await;
        assert_eq!(created["result"]["sessionId"], "session-1");
        write_frame(
            reader.get_mut(),
            serde_json::json!({
                "jsonrpc":"2.0","id":3,"method":"session/prompt",
                "params":{"sessionId":"session-1","prompt":[{"type":"text","text":"work"}]}
            }),
        )
        .await;

        let mut saw_update = false;
        let mut saw_permission = false;
        let mut cancel_sent = false;
        loop {
            let frame = read_frame(&mut reader).await;
            if frame["method"] == "session/update" {
                saw_update = true;
            } else if frame["method"] == "session/request_permission" {
                saw_permission = true;
                write_frame(
                    reader.get_mut(),
                    serde_json::json!({
                        "jsonrpc":"2.0",
                        "id":frame["id"].clone(),
                        "result":{"outcome":"selected","optionId":"allow_once"}
                    }),
                )
                .await;
            }
            if saw_update && saw_permission && !cancel_sent {
                write_frame(
                    reader.get_mut(),
                    serde_json::json!({
                        "jsonrpc":"2.0","method":"session/cancel",
                        "params":{"sessionId":"session-1"}
                    }),
                )
                .await;
                cancel_sent = true;
            }
            if frame["id"] == 3 {
                assert_eq!(frame["result"]["stopReason"], "cancelled");
                break;
            }
        }
        reader.get_mut().shutdown().await.expect("client EOF");
        (saw_update, saw_permission)
    };
    let (result, observed) = tokio::join!(server_task, client_task);
    result.expect("acceptance server");
    assert_eq!(observed, (true, true));
}

#[tokio::test(flavor = "current_thread")]
async fn load_session_replays_history_over_protocol() {
    let root = tempfile::tempdir().expect("temporary cwd");
    let backend = AcceptanceBackend;
    let (mut client, server) = tokio::io::duplex(32 * 1024);
    let (server_read, server_write) = tokio::io::split(server);
    let server_task = super::serve_with_io(backend, server_read, server_write);
    let cwd = root.path().to_string_lossy().into_owned();
    let client_task = async move {
        write_frame(
            &mut client,
            serde_json::json!({
                "jsonrpc":"2.0","id":1,"method":"initialize",
                "params":{"protocolVersion":1,"clientCapabilities":{}}
            }),
        )
        .await;
        let mut reader = BufReader::new(&mut client);
        let _ = read_frame(&mut reader).await;
        write_frame(
            reader.get_mut(),
            serde_json::json!({
                "jsonrpc":"2.0","id":2,"method":"session/load",
                "params":{"sessionId":"session-1","cwd":cwd,"mcpServers":[]}
            }),
        )
        .await;
        let mut updates = Vec::new();
        loop {
            let frame = read_frame(&mut reader).await;
            if frame["method"] == "session/update" {
                updates.push(frame);
            } else if frame["id"] == 2 {
                break;
            }
        }
        reader.get_mut().shutdown().await.expect("client EOF");
        updates
    };
    let (result, updates) = tokio::join!(server_task, client_task);
    result.expect("load server");
    assert_eq!(updates.len(), 2);
    let serialized = serde_json::to_string(&updates).expect("updates JSON");
    assert!(serialized.contains("previous question"));
    assert!(serialized.contains("previous answer"));
}

#[tokio::test(flavor = "current_thread")]
async fn unsupported_capabilities_return_method_not_found() {
    let backend = AcceptanceBackend;
    let (mut client, server) = tokio::io::duplex(16 * 1024);
    let (server_read, server_write) = tokio::io::split(server);
    let server_task = super::serve_with_io(backend, server_read, server_write);
    let client_task = async move {
        write_frame(
            &mut client,
            serde_json::json!({
                "jsonrpc":"2.0","id":1,"method":"initialize",
                "params":{"protocolVersion":1,"clientCapabilities":{}}
            }),
        )
        .await;
        let mut reader = BufReader::new(&mut client);
        let _ = read_frame(&mut reader).await;
        write_frame(
            reader.get_mut(),
            serde_json::json!({
                "jsonrpc":"2.0","id":2,"method":"authenticate",
                "params":{"methodId":"unsupported"}
            }),
        )
        .await;
        let unsupported = read_frame(&mut reader).await;
        reader.get_mut().shutdown().await.expect("client EOF");
        unsupported
    };
    let (result, unsupported) = tokio::join!(server_task, client_task);
    result.expect("unsupported server");
    assert_eq!(unsupported["error"]["code"], -32601);
}
