//! 管理桌面端语言服务协议能力。
//! 本模块把编辑器可见的诊断、补全和后台 LSP 生命周期分离。

use crate::app::lsp::config::{
    LspCommand, LspServerConfig, ensure_rust_analyzer_config, lsp_language_for_path,
    lsp_server_config, resolve_lsp_command,
};
use iced_code_editor::{LspClient, LspDocument, LspEvent, LspPosition, LspRange, LspTextChange};
use serde_json::json;
use std::collections::{HashMap, HashSet};
use std::io::{BufRead, BufReader, Read, Write};
#[cfg(windows)]
use std::os::windows::process::CommandExt;
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, mpsc};
use std::thread::{self, JoinHandle};
use walkdir::WalkDir;

const METHOD_PROGRESS: &str = "$/progress";
const METHOD_WORK_DONE_PROGRESS_CREATE: &str = "window/workDoneProgress/create";
const PROGRESS_KIND_END: &str = "end";
const PROJECT_SCAN_FILE_LIMIT: usize = 2048;

/// 模块内可见结构体，承载 LspServiceManager 对应的状态数据。
/// 字段保持与相邻业务流程和序列化格式一致。
pub(crate) struct LspServiceManager {
    event_sender: mpsc::Sender<LspEvent>,
    services: HashMap<LspServiceKey, LspServiceEntry>,
    prestarted_projects: HashSet<String>,
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
struct LspServiceKey {
    server_key: String,
    root_uri: String,
}

impl LspServiceKey {
    fn new(server_key: &str, root_uri: &str) -> Self {
        Self { server_key: server_key.to_string(), root_uri: root_uri.to_string() }
    }
}

struct LspServiceEntry {
    service: Arc<SharedLspService>,
    child: Child,
    writer_thread: JoinHandle<()>,
    reader_thread: JoinHandle<()>,
    stderr_thread: JoinHandle<()>,
}

struct SharedLspService {
    #[allow(dead_code)]
    server_key: String,
    writer: mpsc::Sender<Vec<u8>>,
    documents: Arc<Mutex<HashMap<String, DocumentState>>>,
    request_id: AtomicU64,
    pending_requests: Arc<Mutex<HashMap<u64, LspRequestKind>>>,
}

/// 模块内可见结构体，承载 SharedLspClient 对应的状态数据。
/// 字段保持与相邻业务流程和序列化格式一致。
pub(crate) struct SharedLspClient {
    service: Arc<SharedLspService>,
}

struct DocumentState {
    text: TextModel,
}

struct TextModel {
    lines: Vec<String>,
}

enum LspRequestKind {
    Hover,
    Completion,
    Definition,
}

impl LspServiceManager {
    /// 模块内可见函数，执行 new 对应的应用流程。
    /// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
    pub(crate) fn new(event_sender: mpsc::Sender<LspEvent>) -> Self {
        Self { event_sender, services: HashMap::new(), prestarted_projects: HashSet::new() }
    }

    /// 模块内可见函数，执行 get_or_create 对应的应用流程。
    /// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
    pub(crate) fn get_or_create(
        &mut self,
        server_key: &str,
        root_uri: &str,
    ) -> Result<Box<dyn LspClient>, String> {
        let service_key = LspServiceKey::new(server_key, root_uri);
        if !self.services.contains_key(&service_key) {
            let entry = LspServiceEntry::spawn(server_key, root_uri, self.event_sender.clone())?;
            self.services.insert(service_key.clone(), entry);
        }

        let service = self
            .services
            .get(&service_key)
            .map(|entry| entry.service.clone())
            .ok_or_else(|| format!("LSP service unavailable: {}", server_key))?;

        Ok(Box::new(SharedLspClient { service }))
    }

    /// 模块内可见函数，执行 prestart_for_project 对应的应用流程。
    /// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
    pub(crate) fn prestart_for_project(&mut self, project_path: &str) {
        if !self.prestarted_projects.insert(project_path.to_string()) {
            return;
        }

        let root_uri = crate::app::path_to_file_uri(Path::new(project_path));
        for server_key in detect_project_servers(project_path) {
            let service_key = LspServiceKey::new(&server_key, &root_uri);
            if self.services.contains_key(&service_key) {
                continue;
            }
            match LspServiceEntry::spawn(&server_key, &root_uri, self.event_sender.clone()) {
                Ok(entry) => {
                    self.services.insert(service_key, entry);
                }
                Err(err) => {
                    let _ = self.event_sender.send(LspEvent::Log {
                        server_key,
                        message: format!("预启动失败: {}", err),
                    });
                }
            }
        }
    }

    /// 模块内可见函数，执行 shutdown_all 对应的应用流程。
    /// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
    pub(crate) fn shutdown_all(&mut self) {
        self.services.clear();
    }
}

impl LspServiceEntry {
    fn spawn(
        server_key: &str,
        root_uri: &str,
        events: mpsc::Sender<LspEvent>,
    ) -> Result<Self, String> {
        let config = lsp_server_config(server_key)
            .ok_or_else(|| format!("Unsupported LSP server: {}", server_key))?;

        if server_key == "rust-analyzer" {
            ensure_rust_analyzer_config();
        }

        let command = resolve_lsp_launch_command(config)?;
        let mut process = Command::new(&command.program);
        #[cfg(windows)]
        process.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
        vw_shared::shell::apply_augmented_path_std_command(&mut process);
        let mut child = process
            .args(&command.args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|error| {
                if error.kind() == std::io::ErrorKind::NotFound {
                    if command.program == "rust-analyzer" {
                        "LSP server program rust-analyzer not found. Please install rust-analyzer or set RUST_ANALYZER/RUST_ANALYZER_PATH environment variable".to_string()
                    } else {
                        format!("LSP server program {} not found", command.program)
                    }
                } else {
                    error.to_string()
                }
            })?;

        let stdin = child.stdin.take().ok_or("stdin unavailable")?;
        let stdout = child.stdout.take().ok_or("stdout unavailable")?;
        let stderr = child.stderr.take().ok_or("stderr unavailable")?;
        let (writer, rx) = mpsc::channel::<Vec<u8>>();
        let documents = Arc::new(Mutex::new(HashMap::new()));
        let pending_requests = Arc::new(Mutex::new(HashMap::new()));
        let tx_reader = writer.clone();
        let pending_reader = pending_requests.clone();
        let events_reader = events.clone();
        let events_log = events;
        let server_key_owned = server_key.to_string();
        let server_key_reader = server_key_owned.clone();
        let server_key_log = server_key_owned.clone();

        let writer_thread = thread::spawn(move || {
            let mut input = stdin;
            for bytes in rx {
                if input.write_all(&bytes).is_err() {
                    break;
                }
                let _ = input.flush();
            }
        });

        let reader_thread = thread::spawn(move || {
            let mut reader = BufReader::new(stdout);
            loop {
                let mut content_length: Option<usize> = None;
                let mut line = String::new();
                loop {
                    line.clear();
                    if reader.read_line(&mut line).ok().filter(|count| *count > 0).is_none() {
                        return;
                    }
                    let trimmed = line.trim();
                    if trimmed.is_empty() {
                        break;
                    }
                    if let Some(value) = trimmed.strip_prefix("Content-Length:")
                        && let Ok(length) = value.trim().parse::<usize>()
                    {
                        content_length = Some(length);
                    }
                }

                let Some(length) = content_length else { continue };
                let mut body = vec![0u8; length];
                if reader.read_exact(&mut body).is_err() {
                    return;
                }

                if let Ok(value) = serde_json::from_slice::<serde_json::Value>(&body) {
                    if let Some(id) = value.get("id").and_then(|raw| raw.as_u64()) {
                        if let Some(method) = value.get("method").and_then(|raw| raw.as_str()) {
                            handle_server_request(id, method, &tx_reader);
                        } else {
                            handle_client_response(id, &value, &pending_reader, &events_reader);
                        }
                    } else if let Some(method) = value.get("method").and_then(|raw| raw.as_str())
                        && let Some(params) = value.get("params")
                    {
                        handle_server_notification(
                            method,
                            params,
                            &events_reader,
                            &server_key_reader,
                        );
                    }
                }
            }
        });

        let stderr_thread = thread::spawn(move || {
            let reader = BufReader::new(stderr);
            for line in reader.lines() {
                let Ok(line) = line else { break };
                let line = line.trim();
                if line.is_empty() {
                    continue;
                }
                let _ = events_log.send(LspEvent::Log {
                    server_key: server_key_log.clone(),
                    message: line.to_string(),
                });
            }
        });

        let service = Arc::new(SharedLspService {
            server_key: server_key_owned,
            writer,
            documents,
            request_id: AtomicU64::new(1),
            pending_requests,
        });

        service.send_message(&json!({
            "jsonrpc": "2.0",
            "id": service.next_id(),
            "method": "initialize",
            "params": {
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
            }
        }));
        service.send_message(&json!({
            "jsonrpc": "2.0",
            "method": "initialized",
            "params": {}
        }));

        Ok(Self { service, child, writer_thread, reader_thread, stderr_thread })
    }
}

fn resolve_lsp_launch_command(config: &LspServerConfig) -> Result<LspCommand, String> {
    let mut command = match resolve_lsp_command(config) {
        Ok(command) => command,
        Err(original_error) => {
            let configured_program = config
                .env_vars
                .iter()
                .find_map(|name| std::env::var(name).ok().filter(|value| !value.trim().is_empty()));
            let program = configured_program
                .and_then(|program| vw_shared::shell::resolve_executable(&program))
                .or_else(|| {
                    config
                        .default_command
                        .first()
                        .and_then(|program| vw_shared::shell::resolve_executable(program))
                })
                .ok_or(original_error)?;
            LspCommand {
                program: program.to_string_lossy().into_owned(),
                args: config.default_command.iter().skip(1).map(|arg| (*arg).to_string()).collect(),
            }
        }
    };

    let resolved = vw_shared::shell::resolve_executable(&command.program)
        .ok_or_else(|| format!("LSP server program {} not found", command.program))?;
    command.program = resolved.to_string_lossy().into_owned();

    if config.key == "rust-analyzer" {
        let mut probe = Command::new(&command.program);
        #[cfg(windows)]
        probe.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
        vw_shared::shell::apply_augmented_path_std_command(&mut probe);
        let output = probe
            .arg("--version")
            .output()
            .map_err(|error| format!("无法验证 rust-analyzer: {error}"))?;
        if !output.status.success() {
            let detail = String::from_utf8_lossy(&output.stderr).trim().to_string();
            return Err(if detail.is_empty() {
                "rust-analyzer 无法启动，请安装 rust-analyzer 组件".to_string()
            } else {
                format!("rust-analyzer 无法启动: {detail}")
            });
        }
    }

    Ok(command)
}

impl Drop for LspServiceEntry {
    fn drop(&mut self) {
        self.service.send_message(&json!({
            "jsonrpc": "2.0",
            "id": self.service.next_id(),
            "method": "shutdown",
            "params": null
        }));
        self.service.send_message(&json!({
            "jsonrpc": "2.0",
            "method": "exit",
            "params": {}
        }));

        if self.child.try_wait().ok().flatten().is_none() {
            let _ = self.child.kill();
        }

        let _ = self.writer_thread.thread().id();
        let _ = self.reader_thread.thread().id();
        let _ = self.stderr_thread.thread().id();
    }
}

impl SharedLspService {
    fn next_id(&self) -> u64 {
        self.request_id.fetch_add(1, Ordering::Relaxed)
    }

    fn send_message(&self, value: &serde_json::Value) {
        if let Ok(data) = serde_json::to_vec(value) {
            let mut header = format!("Content-Length: {}\r\n\r\n", data.len()).into_bytes();
            header.extend_from_slice(&data);
            let _ = self.writer.send(header);
        }
    }

    fn apply_change_and_convert(
        &self,
        uri: &str,
        changes: &[LspTextChange],
    ) -> Vec<serde_json::Value> {
        let mut output = Vec::new();
        let mut docs = self.documents.lock().unwrap_or_else(|error| error.into_inner());
        let Some(state) = docs.get_mut(uri) else { return output };

        for change in changes {
            let start = state.text.to_utf16_position(change.range.start);
            let end = state.text.to_utf16_position(change.range.end);
            output.push(json!({
                "range": {
                    "start": { "line": start.line, "character": start.character },
                    "end": { "line": end.line, "character": end.character }
                },
                "text": change.text
            }));
            state.text.apply_change(change);
        }

        output
    }
}

impl LspClient for SharedLspClient {
    fn did_open(&mut self, document: &LspDocument, text: &str) {
        let mut docs = self.service.documents.lock().unwrap_or_else(|error| error.into_inner());
        docs.insert(document.uri.clone(), DocumentState { text: TextModel::from_text(text) });
        drop(docs);

        self.service.send_message(&json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didOpen",
            "params": {
                "textDocument": {
                    "uri": document.uri,
                    "languageId": document.language_id,
                    "version": document.version,
                    "text": text
                }
            }
        }));
    }

    fn did_change(&mut self, document: &LspDocument, changes: &[LspTextChange]) {
        let content_changes = self.service.apply_change_and_convert(&document.uri, changes);
        if content_changes.is_empty() {
            return;
        }

        self.service.send_message(&json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didChange",
            "params": {
                "textDocument": {
                    "uri": document.uri,
                    "version": document.version
                },
                "contentChanges": content_changes
            }
        }));
    }

    fn did_save(&mut self, document: &LspDocument, text: &str) {
        self.service.send_message(&json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didSave",
            "params": {
                "textDocument": { "uri": document.uri },
                "text": text
            }
        }));
    }

    fn did_close(&mut self, document: &LspDocument) {
        let mut docs = self.service.documents.lock().unwrap_or_else(|error| error.into_inner());
        docs.remove(&document.uri);
        drop(docs);

        self.service.send_message(&json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didClose",
            "params": {
                "textDocument": { "uri": document.uri }
            }
        }));
    }

    fn request_hover(&mut self, document: &LspDocument, position: LspPosition) {
        let docs = self.service.documents.lock().unwrap_or_else(|error| error.into_inner());
        let Some(state) = docs.get(&document.uri) else { return };
        let position = state.text.to_utf16_position(position);
        drop(docs);

        let id = self.service.next_id();
        let mut pending =
            self.service.pending_requests.lock().unwrap_or_else(|error| error.into_inner());
        pending.insert(id, LspRequestKind::Hover);
        drop(pending);

        self.service.send_message(&json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": "textDocument/hover",
            "params": {
                "textDocument": { "uri": document.uri },
                "position": { "line": position.line, "character": position.character }
            }
        }));
    }

    fn request_completion(&mut self, document: &LspDocument, position: LspPosition) {
        let docs = self.service.documents.lock().unwrap_or_else(|error| error.into_inner());
        let Some(state) = docs.get(&document.uri) else { return };
        let position = state.text.to_utf16_position(position);
        drop(docs);

        let id = self.service.next_id();
        let mut pending =
            self.service.pending_requests.lock().unwrap_or_else(|error| error.into_inner());
        pending.insert(id, LspRequestKind::Completion);
        drop(pending);

        self.service.send_message(&json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": "textDocument/completion",
            "params": {
                "textDocument": { "uri": document.uri },
                "position": { "line": position.line, "character": position.character },
                "context": { "triggerKind": 1 }
            }
        }));
    }

    fn request_definition(&mut self, document: &LspDocument, position: LspPosition) {
        let docs = self.service.documents.lock().unwrap_or_else(|error| error.into_inner());
        let Some(state) = docs.get(&document.uri) else { return };
        let position = state.text.to_utf16_position(position);
        drop(docs);

        let id = self.service.next_id();
        let mut pending =
            self.service.pending_requests.lock().unwrap_or_else(|error| error.into_inner());
        pending.insert(id, LspRequestKind::Definition);
        drop(pending);

        self.service.send_message(&json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": "textDocument/definition",
            "params": {
                "textDocument": { "uri": document.uri },
                "position": { "line": position.line, "character": position.character }
            }
        }));
    }
}

impl TextModel {
    fn from_text(text: &str) -> Self {
        let lines = if text.is_empty() {
            vec![String::new()]
        } else {
            text.lines().map(String::from).collect()
        };
        Self { lines }
    }

    fn apply_change(&mut self, change: &LspTextChange) {
        let start_line = change.range.start.line as usize;
        let end_line = change.range.end.line as usize;
        if start_line >= self.lines.len() || end_line >= self.lines.len() {
            return;
        }

        let start_col = change.range.start.character as usize;
        let end_col = change.range.end.character as usize;
        let start_byte = char_to_byte_index(&self.lines[start_line], start_col);
        let end_byte = char_to_byte_index(&self.lines[end_line], end_col);
        let prefix = self.lines[start_line][..start_byte].to_string();
        let suffix = self.lines[end_line][end_byte..].to_string();
        let inserted: Vec<&str> = change.text.split('\n').collect();
        let mut replacement = Vec::new();

        if inserted.len() == 1 {
            replacement.push(format!("{}{}{}", prefix, inserted[0], suffix));
        } else {
            replacement.push(format!("{}{}", prefix, inserted[0]));
            for middle in inserted.iter().take(inserted.len() - 1).skip(1) {
                replacement.push((*middle).to_string());
            }
            replacement.push(format!("{}{}", inserted[inserted.len() - 1], suffix));
        }

        self.lines.splice(start_line..=end_line, replacement);
    }

    fn to_utf16_position(&self, position: LspPosition) -> LspPosition {
        let Some(line) = self.lines.get(position.line as usize).map(String::as_str) else {
            return position;
        };
        let utf16_col =
            line.chars().take(position.character as usize).map(|ch| ch.len_utf16() as u32).sum();
        LspPosition { line: position.line, character: utf16_col }
    }
}

fn char_to_byte_index(text: &str, char_index: usize) -> usize {
    text.char_indices().nth(char_index).map_or(text.len(), |(index, _)| index)
}

fn handle_server_request(id: u64, method: &str, tx: &mpsc::Sender<Vec<u8>>) {
    if method != METHOD_WORK_DONE_PROGRESS_CREATE {
        return;
    }

    let response = json!({
        "jsonrpc": "2.0",
        "id": id,
        "result": null
    });
    if let Ok(data) = serde_json::to_vec(&response) {
        let mut header = format!("Content-Length: {}\r\n\r\n", data.len()).into_bytes();
        header.extend_from_slice(&data);
        let _ = tx.send(header);
    }
}

fn handle_client_response(
    id: u64,
    value: &serde_json::Value,
    pending: &Arc<Mutex<HashMap<u64, LspRequestKind>>>,
    events: &mpsc::Sender<LspEvent>,
) {
    let kind = {
        let mut pending = pending.lock().unwrap_or_else(|error| error.into_inner());
        pending.remove(&id)
    };
    let Some(kind) = kind else { return };
    let result = value.get("result").unwrap_or(&serde_json::Value::Null);

    match kind {
        LspRequestKind::Hover => {
            let text = parse_hover_text(result).unwrap_or_default();
            let _ = events.send(LspEvent::Hover { text });
        }
        LspRequestKind::Completion => {
            let items = parse_completion_items(result);
            if !items.is_empty() {
                let _ = events.send(LspEvent::Completion { items });
            }
        }
        LspRequestKind::Definition => {
            if let Some((uri, range)) = parse_definition_location(result) {
                let _ = events.send(LspEvent::Definition { uri, range });
            }
        }
    }
}

fn handle_server_notification(
    method: &str,
    params: &serde_json::Value,
    events: &mpsc::Sender<LspEvent>,
    server_key: &str,
) {
    if method != METHOD_PROGRESS {
        return;
    }

    let Some(token) = params.get("token").and_then(|raw| {
        raw.as_str().map(String::from).or_else(|| raw.as_i64().map(|value| value.to_string()))
    }) else {
        return;
    };
    let Some(value) = params.get("value") else { return };
    let title =
        value.get("title").and_then(|raw| raw.as_str()).map(String::from).unwrap_or_default();
    let message = value.get("message").and_then(|raw| raw.as_str()).map(String::from);
    let percentage = value.get("percentage").and_then(|raw| raw.as_u64()).map(|raw| raw as u32);
    let done = value.get("kind").and_then(|raw| raw.as_str()) == Some(PROGRESS_KIND_END);

    let _ = events.send(LspEvent::Progress {
        token,
        server_key: server_key.to_string(),
        title,
        message,
        percentage,
        done,
    });
}

fn parse_hover_text(result: &serde_json::Value) -> Option<String> {
    hover_text_from_contents(result.get("contents")?)
}

fn hover_text_from_contents(value: &serde_json::Value) -> Option<String> {
    match value {
        serde_json::Value::String(text) => Some(text.clone()),
        serde_json::Value::Array(items) => {
            let parts: Vec<String> = items.iter().filter_map(hover_text_from_contents).collect();
            if parts.is_empty() { None } else { Some(parts.join("\n")) }
        }
        serde_json::Value::Object(map) => {
            map.get("value").and_then(|raw| raw.as_str()).map(String::from)
        }
        _ => None,
    }
}

fn parse_completion_items(result: &serde_json::Value) -> Vec<String> {
    let mut items = Vec::new();
    if let Some(array) = result.as_array() {
        items.extend(array.iter());
    } else if let Some(array) = result.get("items").and_then(|raw| raw.as_array()) {
        items.extend(array.iter());
    }

    items
        .into_iter()
        .filter_map(|item| item.get("label").and_then(|raw| raw.as_str()))
        .map(String::from)
        .collect()
}

fn parse_definition_location(result: &serde_json::Value) -> Option<(String, LspRange)> {
    fn extract_location(value: &serde_json::Value) -> Option<(String, LspRange)> {
        let uri = value.get("uri")?.as_str()?.to_string();
        let range = value.get("range")?;
        Some((uri, parse_range(range)?))
    }

    fn extract_link(value: &serde_json::Value) -> Option<(String, LspRange)> {
        let uri = value.get("targetUri")?.as_str()?.to_string();
        let range = value.get("targetSelectionRange").or_else(|| value.get("targetRange"))?;
        Some((uri, parse_range(range)?))
    }

    if let Some(array) = result.as_array() {
        let first = array.first()?;
        if first.get("targetUri").is_some() { extract_link(first) } else { extract_location(first) }
    } else if result.is_object() {
        extract_location(result)
    } else {
        None
    }
}

fn parse_range(value: &serde_json::Value) -> Option<LspRange> {
    let start = value.get("start")?;
    let end = value.get("end")?;
    Some(LspRange {
        start: LspPosition {
            line: start.get("line")?.as_u64()? as u32,
            character: start.get("character")?.as_u64()? as u32,
        },
        end: LspPosition {
            line: end.get("line")?.as_u64()? as u32,
            character: end.get("character")?.as_u64()? as u32,
        },
    })
}

fn detect_project_servers(project_path: &str) -> Vec<String> {
    let mut servers = HashSet::new();
    for entry in WalkDir::new(project_path)
        .follow_links(false)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_file())
        .take(PROJECT_SCAN_FILE_LIMIT)
    {
        if let Some(language) = lsp_language_for_path(entry.path()) {
            servers.insert(language.server_key.to_string());
        }
    }

    let mut servers = servers.into_iter().collect::<Vec<_>>();
    servers.sort();
    servers
}
#[cfg(test)]
#[path = "manager_tests.rs"]
mod manager_tests;
