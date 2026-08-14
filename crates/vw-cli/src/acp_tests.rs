use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use clap::Parser;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use vw_acp::server::{
    BackendCancelHandle, BackendError, BackendPrompt, BackendSession, PromptContent, SessionBackend,
};

use super::run_with_io;

#[test]
fn cli_accepts_acp_subcommand_without_extra_flags() {
    let cli = crate::cli::Cli::try_parse_from(["vibewindow", "acp"]).expect("ACP command");
    assert!(matches!(cli.command, crate::cli::Commands::Acp));
}

#[derive(Clone)]
struct RecordingBackend {
    cwd: PathBuf,
    creates: Arc<AtomicUsize>,
    prompt_started: Arc<AtomicBool>,
    cancel_count: Arc<AtomicUsize>,
    active: Arc<
        std::sync::Mutex<Option<tokio::sync::mpsc::UnboundedSender<vw_acp::server::BackendEvent>>>,
    >,
}

impl RecordingBackend {
    fn new(cwd: PathBuf) -> Self {
        Self {
            cwd,
            creates: Arc::new(AtomicUsize::new(0)),
            prompt_started: Arc::new(AtomicBool::new(false)),
            cancel_count: Arc::new(AtomicUsize::new(0)),
            active: Arc::new(std::sync::Mutex::new(None)),
        }
    }
}

struct RecordingCancel {
    count: Arc<AtomicUsize>,
}

impl BackendCancelHandle for RecordingCancel {
    fn cancel(&self) -> bool {
        self.count.fetch_add(1, Ordering::SeqCst) == 0
    }
}

#[async_trait::async_trait]
impl SessionBackend for RecordingBackend {
    async fn create_session(&self, cwd: PathBuf) -> Result<BackendSession, BackendError> {
        self.creates.fetch_add(1, Ordering::SeqCst);
        Ok(BackendSession { id: "session-1".to_string(), cwd, history: Vec::new() })
    }

    async fn load_session(
        &self,
        session_id: &str,
        cwd: PathBuf,
    ) -> Result<BackendSession, BackendError> {
        Ok(BackendSession { id: session_id.to_string(), cwd, history: Vec::new() })
    }

    async fn start_prompt(
        &self,
        _session_id: &str,
        _prompt: Vec<PromptContent>,
    ) -> Result<BackendPrompt, BackendError> {
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
        *self.active.lock().expect("active prompt lock") = Some(tx);
        self.prompt_started.store(true, Ordering::SeqCst);
        Ok(BackendPrompt::new(
            rx,
            Arc::new(RecordingCancel { count: Arc::clone(&self.cancel_count) }),
        ))
    }
}

async fn initialize_line(backend: RecordingBackend) -> String {
    let (mut client, server) = tokio::io::duplex(16 * 1024);
    let (reader, writer) = tokio::io::split(server);
    let server_task = run_with_io(backend, reader, writer);
    let client_task = async move {
        client
            .write_all(
                br#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":1,"clientCapabilities":{}}}
"#,
            )
            .await
            .expect("initialize request");
        let mut line = String::new();
        BufReader::new(&mut client).read_line(&mut line).await.expect("initialize response");
        client.shutdown().await.expect("client shutdown");
        line
    };
    let (result, line) = tokio::join!(server_task, client_task);
    result.expect("stdio server");
    line
}

#[tokio::test(flavor = "current_thread")]
async fn acp_command_routes_to_agent_side_stdio() {
    let root = tempfile::tempdir().expect("temporary cwd");
    let response = initialize_line(RecordingBackend::new(root.path().to_path_buf())).await;
    let frame: serde_json::Value = serde_json::from_str(response.trim()).expect("JSON-RPC");
    assert_eq!(frame["jsonrpc"], "2.0");
    assert_eq!(frame["id"], 1);
    assert_eq!(frame["result"]["protocolVersion"], 1);
}

#[tokio::test(flavor = "current_thread")]
async fn acp_stdio_keeps_diagnostics_off_stdout() {
    let root = tempfile::tempdir().expect("temporary cwd");
    let output = initialize_line(RecordingBackend::new(root.path().to_path_buf())).await;
    assert!(!output.contains("VibeWindow"));
    assert!(!output.contains("INFO"));
    for line in output.lines() {
        let frame: serde_json::Value = serde_json::from_str(line).expect("protocol frame");
        assert_eq!(frame["jsonrpc"], "2.0");
    }
}

#[tokio::test(flavor = "current_thread")]
async fn acp_stdio_eof_cancels_and_exits() {
    let root = tempfile::tempdir().expect("temporary cwd");
    let backend = RecordingBackend::new(root.path().to_path_buf());
    let observed = backend.clone();
    let (mut client, server) = tokio::io::duplex(32 * 1024);
    let (reader, writer) = tokio::io::split(server);
    let server_task = run_with_io(backend, reader, writer);
    let cwd = root.path().to_string_lossy().into_owned();
    let client_task = async move {
        let requests = format!(
            "{{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"initialize\",\"params\":{{\"protocolVersion\":1,\"clientCapabilities\":{{}}}}}}\n\
             {{\"jsonrpc\":\"2.0\",\"id\":2,\"method\":\"session/new\",\"params\":{{\"cwd\":{},\"mcpServers\":[]}}}}\n\
             {{\"jsonrpc\":\"2.0\",\"id\":3,\"method\":\"session/prompt\",\"params\":{{\"sessionId\":\"session-1\",\"prompt\":[{{\"type\":\"text\",\"text\":\"wait\"}}]}}}}\n",
            serde_json::to_string(&cwd).expect("cwd JSON"),
        );
        client.write_all(requests.as_bytes()).await.expect("protocol requests");
        for _ in 0..100 {
            if observed.prompt_started.load(Ordering::SeqCst) {
                break;
            }
            tokio::task::yield_now().await;
        }
        assert!(observed.prompt_started.load(Ordering::SeqCst));
        client.shutdown().await.expect("stdin EOF");
    };

    let (result, ()) = tokio::join!(server_task, client_task);
    result.expect("stdio exits after EOF");
    assert_eq!(observed.creates.load(Ordering::SeqCst), 1);
    assert_eq!(observed.cancel_count.load(Ordering::SeqCst), 1);
}
