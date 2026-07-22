//! LSP 子进程后端和会话复用管理。
//!
//! 本模块负责按语言启动语言服务器、维护 JSON-RPC 请求/响应映射，并把当前文件
//! 内容同步给 LSP。它只暴露会话级接口给工具层，具体 server 发现逻辑放在
//! `config` 模块中。

use super::config::{ResolvedServerCommand, ServerConfig, resolve_command, server_config_for_path};
use once_cell::sync::Lazy;
use parking_lot::Mutex;
use serde_json::{Value, json};
use std::collections::HashMap;
use std::io::{BufRead, BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc;
use tokio::sync::oneshot;
use tokio::time::{Duration, timeout};

const REQUEST_TIMEOUT: Duration = Duration::from_secs(8);
const INITIALIZE_TIMEOUT: Duration = Duration::from_secs(12);

static GLOBAL_MANAGER: Lazy<Mutex<BackendManager>> =
    Lazy::new(|| Mutex::new(BackendManager::default()));

#[derive(Default)]
struct BackendManager {
    services: HashMap<String, Arc<SharedLspService>>,
}

#[derive(Debug, Clone)]
struct DocumentState {
    version: i32,
    text: String,
}

/// 单个文件和某个 LSP 服务之间的会话句柄。
///
/// 句柄内部共享后端服务，记录本次打开文件的 server、language id 和 URI。
/// 请求错误包括 server 不存在、子进程启动失败、协议响应错误或超时。
pub(crate) struct LspBackendSession {
    service: Arc<SharedLspService>,
    server_key: String,
    language_id: String,
    uri: String,
}

struct SharedLspService {
    #[cfg(not(test))]
    #[allow(dead_code)]
    child: Mutex<Child>,
    #[cfg(test)]
    #[allow(dead_code)]
    child: Mutex<Option<Child>>,
    writer: mpsc::Sender<Vec<u8>>,
    request_id: AtomicU64,
    pending: Mutex<HashMap<u64, oneshot::Sender<anyhow::Result<Value>>>>,
    documents: Mutex<HashMap<String, DocumentState>>,
}

impl LspBackendSession {
    /// 为文件打开或复用一个 LSP 后端会话。
    ///
    /// `workspace_root` 用于确定 server root；`absolute_path` 必须是待分析文件的绝对路径；
    /// `file_content` 会通过 `didOpen`/`didChange` 同步给 server。若文件类型没有配置
    /// 或本机找不到对应语言服务器，返回 `Ok(None)`。
    pub(crate) async fn open(
        workspace_root: &Path,
        absolute_path: &Path,
        file_content: &str,
    ) -> anyhow::Result<Option<Self>> {
        let Some(config) = server_config_for_path(absolute_path) else {
            return Ok(None);
        };
        let Some(command) = resolve_command(config) else {
            return Ok(None);
        };

        let root_dir = resolve_root_dir(workspace_root, absolute_path);
        let service_key = format!("{}::{}", config.server_key, root_dir.to_string_lossy());
        let existing = { GLOBAL_MANAGER.lock().services.get(&service_key).cloned() };
        let service = if let Some(service) = existing {
            service
        } else {
            let spawned = spawn_service(config, &command, &root_dir).await?;
            let mut manager = GLOBAL_MANAGER.lock();
            // 启动和插入分开做，避免持有全局锁等待子进程初始化。
            manager.services.entry(service_key).or_insert_with(|| spawned.clone()).clone()
        };

        let uri = path_to_file_uri(absolute_path);
        service.sync_document(&uri, config.language_id, file_content).await?;
        Ok(Some(Self {
            service,
            server_key: config.server_key.to_string(),
            language_id: config.language_id.to_string(),
            uri,
        }))
    }

    /// 向当前 LSP server 发送请求并等待响应。
    ///
    /// `method` 和 `params` 必须符合 LSP 协议；返回值为原始 JSON `result`。协议错误、
    /// transport 关闭或超时会作为 `anyhow::Error` 返回。
    pub(crate) async fn request(&self, method: &str, params: Value) -> anyhow::Result<Value> {
        self.service.send_request(method, params, REQUEST_TIMEOUT).await
    }

    /// 返回当前会话使用的语言服务器标识。
    pub(crate) fn server_key(&self) -> &str {
        &self.server_key
    }

    /// 返回当前文件同步到 LSP 时使用的 language id。
    pub(crate) fn language_id(&self) -> &str {
        &self.language_id
    }

    /// 返回当前文件的 `file://` URI。
    pub(crate) fn uri(&self) -> &str {
        &self.uri
    }
}

async fn spawn_service(
    config: &ServerConfig,
    command: &ResolvedServerCommand,
    root_dir: &Path,
) -> anyhow::Result<Arc<SharedLspService>> {
    let mut child = Command::new(&command.program)
        .args(&command.args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| anyhow::anyhow!("Failed to start {}: {error}", config.server_key))?;

    let stdin = child
        .stdin
        .take()
        .ok_or_else(|| anyhow::anyhow!("stdin unavailable for {}", config.server_key))?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| anyhow::anyhow!("stdout unavailable for {}", config.server_key))?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| anyhow::anyhow!("stderr unavailable for {}", config.server_key))?;
    let (writer, rx) = mpsc::channel::<Vec<u8>>();

    #[cfg(test)]
    let child = Some(child);

    let service = Arc::new(SharedLspService {
        child: Mutex::new(child),
        writer,
        request_id: AtomicU64::new(1),
        pending: Mutex::new(HashMap::new()),
        documents: Mutex::new(HashMap::new()),
    });

    std::thread::spawn(move || {
        let mut stdin = stdin;
        for bytes in rx {
            if stdin.write_all(&bytes).is_err() {
                break;
            }
            let _ = stdin.flush();
        }
    });

    let reader_service = service.clone();
    // stdout 读取在独立线程中持续运行，把响应分发给 pending 请求表。
    std::thread::spawn(move || {
        let mut reader = BufReader::new(stdout);
        while let Some(message) = read_lsp_message(&mut reader) {
            reader_service.handle_message(message);
        }
    });

    let server_key = config.server_key.to_string();
    std::thread::spawn(move || {
        let reader = BufReader::new(stderr);
        for line in reader.lines() {
            let Ok(line) = line else {
                break;
            };
            let line = line.trim();
            if !line.is_empty() {
                tracing::debug!(target: "vw_agent::tools::lsp", server = %server_key, "{line}");
            }
        }
    });

    let root_uri = path_to_file_uri(root_dir);
    let initialize = json!({
        "processId": std::process::id(),
        "rootUri": root_uri,
        "capabilities": {
            "textDocument": {
                "synchronization": {
                    "dynamicRegistration": false,
                    "willSave": false,
                    "didSave": true
                }
            },
            "window": {
                "workDoneProgress": true
            }
        },
        "workspaceFolders": null
    });
    let _ = service
        .send_request("initialize", initialize, INITIALIZE_TIMEOUT)
        .await
        .map_err(|error| anyhow::anyhow!("Failed to initialize {}: {error}", config.server_key))?;
    service.send_notification("initialized", json!({}))?;
    Ok(service)
}

impl SharedLspService {
    async fn sync_document(&self, uri: &str, language_id: &str, text: &str) -> anyhow::Result<()> {
        let mut documents = self.documents.lock();
        if let Some(state) = documents.get_mut(uri) {
            if state.text == text {
                return Ok(());
            }
            state.version += 1;
            state.text = text.to_string();
            let version = state.version;
            drop(documents);
            self.send_notification(
                "textDocument/didChange",
                json!({
                    "textDocument": { "uri": uri, "version": version },
                    "contentChanges": [{ "text": text }]
                }),
            )?;
            return Ok(());
        }

        documents.insert(uri.to_string(), DocumentState { version: 1, text: text.to_string() });
        drop(documents);
        self.send_notification(
            "textDocument/didOpen",
            json!({
                "textDocument": {
                    "uri": uri,
                    "languageId": language_id,
                    "version": 1,
                    "text": text
                }
            }),
        )?;
        Ok(())
    }

    async fn send_request(
        &self,
        method: &str,
        params: Value,
        wait_for: Duration,
    ) -> anyhow::Result<Value> {
        let id = self.request_id.fetch_add(1, Ordering::Relaxed);
        let (tx, rx) = oneshot::channel();
        self.pending.lock().insert(id, tx);
        let request = json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method,
            "params": params,
        });

        if let Err(error) = self.send_raw(&request) {
            self.pending.lock().remove(&id);
            return Err(error);
        }

        match timeout(wait_for, rx).await {
            Ok(Ok(result)) => result,
            Ok(Err(_)) => Err(anyhow::anyhow!("LSP response channel closed for {method}")),
            Err(_) => {
                // 超时后清理 pending，防止迟到响应永久占用请求表。
                self.pending.lock().remove(&id);
                Err(anyhow::anyhow!("LSP request timed out for {method}"))
            }
        }
    }

    fn send_notification(&self, method: &str, params: Value) -> anyhow::Result<()> {
        self.send_raw(&json!({
            "jsonrpc": "2.0",
            "method": method,
            "params": params,
        }))
    }

    fn send_raw(&self, value: &Value) -> anyhow::Result<()> {
        let mut payload = serde_json::to_vec(value)?;
        let mut header = format!("Content-Length: {}\r\n\r\n", payload.len()).into_bytes();
        header.append(&mut payload);
        self.writer.send(header).map_err(|_| anyhow::anyhow!("LSP transport is closed"))
    }

    fn handle_message(&self, value: Value) {
        if value.get("method").is_some() && value.get("id").is_some() {
            self.handle_server_request(&value);
            return;
        }

        let Some(id) = value.get("id").and_then(Value::as_u64) else {
            return;
        };
        let Some(sender) = self.pending.lock().remove(&id) else {
            return;
        };

        if let Some(error) = value.get("error") {
            let _ = sender.send(Err(anyhow::anyhow!(format_lsp_error(error))));
            return;
        }

        let result = value.get("result").cloned().unwrap_or(Value::Null);
        let _ = sender.send(Ok(result));
    }

    fn handle_server_request(&self, value: &Value) {
        let Some(id) = value.get("id").and_then(Value::as_u64) else {
            return;
        };
        // 当前工具不支持 server 发起的交互请求，返回 null 让语言服务器继续工作。
        let result = Value::Null;
        let _ = self.send_raw(&json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": result,
        }));
    }
}

fn read_lsp_message(reader: &mut BufReader<impl Read>) -> Option<Value> {
    let mut content_length = None;
    let mut line = String::new();

    loop {
        line.clear();
        let read = reader.read_line(&mut line).ok()?;
        if read == 0 {
            return None;
        }
        let trimmed = line.trim();
        if trimmed.is_empty() {
            break;
        }
        if let Some(value) = trimmed.strip_prefix("Content-Length:") {
            content_length = value.trim().parse::<usize>().ok();
        }
    }

    let length = content_length?;
    let mut body = vec![0_u8; length];
    reader.read_exact(&mut body).ok()?;
    serde_json::from_slice(&body).ok()
}

fn format_lsp_error(error: &Value) -> String {
    let code = error.get("code").and_then(Value::as_i64);
    let message = error.get("message").and_then(Value::as_str).unwrap_or("unknown LSP error");
    match code {
        Some(code) => format!("{message} (code {code})"),
        None => message.to_string(),
    }
}

fn resolve_root_dir(workspace_root: &Path, file_path: &Path) -> PathBuf {
    let workspace_root =
        workspace_root.canonicalize().unwrap_or_else(|_| workspace_root.to_path_buf());
    let candidate = file_path
        .parent()
        .map(|path| path.canonicalize().unwrap_or_else(|_| path.to_path_buf()))
        .unwrap_or_else(|| workspace_root.clone());

    if candidate.starts_with(&workspace_root) { workspace_root } else { candidate }
}

fn path_to_file_uri(path: &Path) -> String {
    format!("file://{}", path.to_string_lossy().replace(' ', "%20"))
}
#[cfg(test)]
#[path = "backend_tests.rs"]
mod backend_tests;
