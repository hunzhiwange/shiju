//! ACP 子命令的协议边界测试。
//! 覆盖命令行路由、基于标准输入输出的初始化交互，以及 EOF 触发的取消与退出。
//! 同时约束诊断信息不得污染 ACP 协议的标准输出通道。

use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use clap::Parser;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use acp_server::server::{
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
        std::sync::Mutex<Option<tokio::sync::mpsc::UnboundedSender<acp_server::server::BackendEvent>>>,
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
    assert!(!output.contains("ShiJu"));
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

#[test]
fn employee_acp_cli_parses_scope_target_and_state_directory() {
    use crate::cli::{Commands, EmployeeAcpKind};

    let args = [
        "shiju",
        "employee-acp",
        "--employee-id",
        "fixture-employee",
        "--employee-kind",
        "shared",
        "--state-dir",
        "/fixture/state",
        "--account-scope",
        "fixture-account",
    ];
    let cli = crate::cli::Cli::try_parse_from(args).expect("employee ACP command");
    let Commands::EmployeeAcp { employee_id, employee_kind, state_dir, account_scope } =
        cli.command
    else {
        panic!("wrong command");
    };
    assert_eq!(employee_id, "fixture-employee");
    assert!(matches!(employee_kind, EmployeeAcpKind::Shared));
    assert_eq!(state_dir, PathBuf::from("/fixture/state"));
    assert_eq!(account_scope, "fixture-account");
    assert!(crate::cli::Cli::try_parse_from(&args[..args.len() - 2]).is_err());
}

#[tokio::test]
async fn employee_acp_cli_missing_credential_leaves_stdout_empty() {
    let root = tempfile::tempdir().unwrap();
    let (reader, writer) = tokio::io::duplex(1024);
    let error = super::run_employee_with_io(
        "fixture-employee".into(),
        crate::cli::EmployeeAcpKind::Private,
        root.path().to_owned(),
        "fixture-account".into(),
        None,
        tokio::io::empty(),
        writer,
    )
    .await
    .unwrap_err();
    assert!(error.to_string().contains("SHIJU_EMPLOYEE_ACP_CREDENTIAL"));
    let mut output = String::new();
    BufReader::new(reader).read_line(&mut output).await.unwrap();
    assert_eq!(output, "");
}

#[tokio::test]
async fn employee_acp_cli_stdout_contains_only_protocol_frames() {
    let root = tempfile::tempdir().unwrap();
    let (mut client, server) = tokio::io::duplex(4096);
    let (reader, writer) = tokio::io::split(server);
    let server = super::run_employee_with_io(
        "fixture-employee".into(),
        crate::cli::EmployeeAcpKind::Private,
        root.path().to_owned(),
        "fixture-account".into(),
        Some("fixture-credential"),
        reader,
        writer,
    );
    let client = async move {
        use tokio::io::AsyncReadExt;

        client.write_all(b"{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"initialize\",\"params\":{\"protocolVersion\":1}}\n")
            .await.unwrap();
        let mut client = BufReader::new(client);
        let mut output = String::new();
        client.read_line(&mut output).await.unwrap();
        client.get_mut().shutdown().await.unwrap();
        client.read_to_string(&mut output).await.unwrap();
        output
    };
    let (server, output) = tokio::time::timeout(std::time::Duration::from_secs(5), async {
        tokio::join!(server, client)
    })
    .await
    .unwrap();
    server.unwrap();
    let frame: serde_json::Value = serde_json::from_str(output.trim()).unwrap();
    assert_eq!(frame["id"], 1);
    assert_eq!(frame["result"]["agentCapabilities"]["loadSession"], true);
    assert!(!output.contains("fixture-credential"));
}
