//! RustCodeGraph 库内只读查询适配器。

use super::super::traits::{
    Tool, ToolCallResult, ToolCallTelemetry, ToolRenderHint, ToolResult, ToolSpec,
};
use ::rustcodegraph::mcp::tools::{ToolHandler, ToolResult as LibraryToolResult};
use ::rustcodegraph::{CodeGraph, IndexOptions, WatchOptions};
use async_trait::async_trait;
use serde::Deserialize;
use serde_json::{Map, Value, json};
use std::path::{Component, Path, PathBuf};
use std::sync::{Mutex, mpsc};
use std::thread::{self, JoinHandle};
use vw_api_types::tools::ToolResultContentDto;

const TOOL_ID: &str = "rustcodegraph";
pub(super) const SUPPORTED_OPERATIONS: &[&str] =
    &["explore", "node", "search", "callers", "callees", "impact", "files", "status"];

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "lowercase")]
enum Operation {
    Explore,
    Node,
    Search,
    Callers,
    Callees,
    Impact,
    Files,
    Status,
}

impl Operation {
    fn as_str(self) -> &'static str {
        match self {
            Self::Explore => "explore",
            Self::Node => "node",
            Self::Search => "search",
            Self::Callers => "callers",
            Self::Callees => "callees",
            Self::Impact => "impact",
            Self::Files => "files",
            Self::Status => "status",
        }
    }

    fn library_tool(self) -> &'static str {
        match self {
            Self::Explore => "rustcodegraph_explore",
            Self::Node => "rustcodegraph_node",
            Self::Search => "rustcodegraph_search",
            Self::Callers => "rustcodegraph_callers",
            Self::Callees => "rustcodegraph_callees",
            Self::Impact => "rustcodegraph_impact",
            Self::Files => "rustcodegraph_files",
            Self::Status => "rustcodegraph_status",
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Args {
    operation: Operation,
    #[serde(default)]
    query: Option<String>,
    #[serde(default)]
    symbol: Option<String>,
    #[serde(default)]
    kind: Option<String>,
    #[serde(default)]
    limit: Option<u64>,
    #[serde(default)]
    file: Option<String>,
    #[serde(default)]
    include_code: Option<bool>,
    #[serde(default)]
    offset: Option<u64>,
    #[serde(default)]
    symbols_only: Option<bool>,
    #[serde(default)]
    line: Option<u64>,
    #[serde(default)]
    depth: Option<u64>,
    #[serde(default)]
    max_files: Option<u64>,
    #[serde(default)]
    path: Option<String>,
    #[serde(default)]
    pattern: Option<String>,
    #[serde(default)]
    format: Option<String>,
    #[serde(default)]
    include_metadata: Option<bool>,
    #[serde(default)]
    max_depth: Option<u64>,
}

impl Args {
    fn validate(&mut self) -> anyhow::Result<()> {
        match self.operation {
            Operation::Explore | Operation::Search => {
                self.query = Some(required_text(self.query.take(), "query")?);
            }
            Operation::Callers | Operation::Callees | Operation::Impact => {
                self.symbol = Some(required_text(self.symbol.take(), "symbol")?);
            }
            Operation::Node => {
                self.symbol = optional_text(self.symbol.take());
                self.file = optional_relative_path(self.file.take(), "file")?;
                if self.symbol.is_none() && self.file.is_none() {
                    anyhow::bail!("node requires a non-empty symbol or file");
                }
            }
            Operation::Files | Operation::Status => {}
        }

        if !matches!(self.operation, Operation::Node) {
            self.file = optional_relative_path(self.file.take(), "file")?;
        }
        self.path = optional_relative_path(self.path.take(), "path")?;
        self.pattern = optional_relative_path(self.pattern.take(), "pattern")?;
        Ok(())
    }

    fn library_args(&self) -> Map<String, Value> {
        let mut args = Map::new();
        match self.operation {
            Operation::Explore => {
                insert_string(&mut args, "query", self.query.as_ref());
                insert_number(&mut args, "maxFiles", self.max_files);
            }
            Operation::Node => {
                insert_string(&mut args, "symbol", self.symbol.as_ref());
                insert_bool(&mut args, "includeCode", self.include_code);
                insert_string(&mut args, "file", self.file.as_ref());
                insert_number(&mut args, "offset", self.offset);
                insert_number(&mut args, "limit", self.limit);
                insert_bool(&mut args, "symbolsOnly", self.symbols_only);
                insert_number(&mut args, "line", self.line);
            }
            Operation::Search => {
                insert_string(&mut args, "query", self.query.as_ref());
                insert_string(&mut args, "kind", self.kind.as_ref());
                insert_number(&mut args, "limit", self.limit);
            }
            Operation::Callers | Operation::Callees => {
                insert_string(&mut args, "symbol", self.symbol.as_ref());
                insert_string(&mut args, "file", self.file.as_ref());
                insert_number(&mut args, "limit", self.limit);
            }
            Operation::Impact => {
                insert_string(&mut args, "symbol", self.symbol.as_ref());
                insert_string(&mut args, "file", self.file.as_ref());
                insert_number(&mut args, "depth", self.depth);
            }
            Operation::Files => {
                insert_string(&mut args, "path", self.path.as_ref());
                insert_string(&mut args, "pattern", self.pattern.as_ref());
                insert_string(&mut args, "format", self.format.as_ref());
                insert_bool(&mut args, "includeMetadata", self.include_metadata);
                insert_number(&mut args, "maxDepth", self.max_depth);
            }
            Operation::Status => {}
        }
        args
    }
}

enum LifecycleCommand {
    SyncOnceAndWatch(mpsc::SyncSender<()>),
    Shutdown,
}

struct IndexLifecycleWorker {
    commands: mpsc::Sender<LifecycleCommand>,
    thread: Mutex<Option<JoinHandle<()>>>,
}

pub struct RustCodeGraphTool {
    project_root: PathBuf,
    lifecycle: IndexLifecycleWorker,
}

impl RustCodeGraphTool {
    pub fn new(project_root: PathBuf) -> anyhow::Result<Self> {
        Self::new_with_watch_options(project_root, WatchOptions::default())
    }

    pub(super) fn new_with_watch_options(
        project_root: PathBuf,
        watch_options: WatchOptions,
    ) -> anyhow::Result<Self> {
        let project_root = project_root.canonicalize().map_err(|error| {
            anyhow::anyhow!(
                "RustCodeGraph workspace is unavailable at {}: {error}",
                project_root.display()
            )
        })?;
        if !project_root.is_dir() {
            anyhow::bail!(
                "RustCodeGraph workspace is not a directory: {}",
                project_root.display()
            );
        }
        if !CodeGraph::is_initialized(&project_root) {
            anyhow::bail!(
                "RustCodeGraph is not initialized in the exact workspace: {}",
                project_root.display()
            );
        }

        let lifecycle = Self::start_lifecycle_worker(&project_root, watch_options)?;
        Ok(Self {
            project_root,
            lifecycle,
        })
    }

    fn start_lifecycle_worker(
        project_root: &Path,
        watch_options: WatchOptions,
    ) -> anyhow::Result<IndexLifecycleWorker> {
        let worker_root = project_root.to_path_buf();
        let (commands, receiver) = mpsc::channel();
        let (initialized_tx, initialized_rx) = mpsc::sync_channel(1);
        let thread = thread::Builder::new()
            .name("vw-rustcodegraph-lifecycle".to_string())
            .spawn(move || {
                let mut graph = match CodeGraph::open_sync(&worker_root) {
                    Ok(graph) => graph,
                    Err(error) => {
                        let _ = initialized_tx.send(Err(error.to_string()));
                        return;
                    }
                };
                if initialized_tx.send(Ok(())).is_err() {
                    graph.close();
                    return;
                }

                let mut first_sync_pending = true;
                while let Ok(command) = receiver.recv() {
                    match command {
                        LifecycleCommand::SyncOnceAndWatch(done) => {
                            if first_sync_pending {
                                graph.sync(IndexOptions::default());
                                graph.watch(watch_options.clone());
                                first_sync_pending = false;
                            }
                            let _ = done.send(());
                        }
                        LifecycleCommand::Shutdown => break,
                    }
                }

                graph.unwatch();
                graph.close();
            })
            .map_err(|error| anyhow::anyhow!("failed to start RustCodeGraph lifecycle: {error}"))?;

        match initialized_rx.recv() {
            Ok(Ok(())) => Ok(IndexLifecycleWorker {
                commands,
                thread: Mutex::new(Some(thread)),
            }),
            Ok(Err(error)) => {
                let _ = thread.join();
                anyhow::bail!(
                    "RustCodeGraph index is unavailable in {}: {error}",
                    project_root.display()
                )
            }
            Err(error) => {
                let _ = thread.join();
                anyhow::bail!("RustCodeGraph lifecycle failed during startup: {error}")
            }
        }
    }

    fn schema() -> Value {
        json!({
            "type": "object",
            "properties": {
                "operation": {
                    "type": "string",
                    "enum": SUPPORTED_OPERATIONS,
                    "description": "RustCodeGraph operation; prefer explore for code understanding."
                },
                "query": { "type": "string" },
                "symbol": { "type": "string" },
                "kind": {
                    "type": "string",
                    "enum": ["function", "method", "class", "interface", "type", "variable", "route", "component"]
                },
                "limit": { "type": "integer", "minimum": 1 },
                "file": { "type": "string", "description": "Workspace-relative file path or suffix." },
                "includeCode": { "type": "boolean" },
                "offset": { "type": "integer", "minimum": 1 },
                "symbolsOnly": { "type": "boolean" },
                "line": { "type": "integer", "minimum": 1 },
                "depth": { "type": "integer", "minimum": 1 },
                "maxFiles": { "type": "integer", "minimum": 1 },
                "path": { "type": "string", "description": "Workspace-relative directory filter." },
                "pattern": { "type": "string", "description": "Workspace-relative glob filter." },
                "format": { "type": "string", "enum": ["tree", "flat", "grouped"] },
                "includeMetadata": { "type": "boolean" },
                "maxDepth": { "type": "integer", "minimum": 1 }
            },
            "required": ["operation"],
            "additionalProperties": false
        })
    }

    async fn execute_query(&self, args: &Args) -> anyhow::Result<LibraryToolResult> {
        let project_root = self.project_root.clone();
        let lifecycle_commands = self.lifecycle.commands.clone();
        let operation = args.operation;
        let library_args = args.library_args();
        tokio::task::spawn_blocking(move || -> anyhow::Result<LibraryToolResult> {
            if !matches!(operation, Operation::Status) {
                let (done, completed) = mpsc::sync_channel(1);
                lifecycle_commands
                    .send(LifecycleCommand::SyncOnceAndWatch(done))
                    .map_err(|_| anyhow::anyhow!("RustCodeGraph lifecycle worker stopped"))?;
                completed
                    .recv()
                    .map_err(|_| anyhow::anyhow!("RustCodeGraph lifecycle sync was interrupted"))?;
            }

            let mut handler = ToolHandler::new(true);
            handler.set_default_project_root(project_root.to_string_lossy().into_owned());
            Ok(handler.execute_for_cli(operation.library_tool(), &library_args))
        })
        .await
        .map_err(|error| anyhow::anyhow!("RustCodeGraph query task failed: {error}"))
        .and_then(|result| result)
    }

    fn map_result(operation: Operation, result: LibraryToolResult) -> ToolCallResult {
        let success = result.is_error != Some(true);
        let texts = result.content.into_iter().map(|content| content.text).collect::<Vec<_>>();
        let result_text = texts.join("\n");
        let data = json!({
            "success": success,
            "operation": operation.as_str(),
            "library_tool": operation.library_tool(),
            "result_text": result_text,
            "is_error": !success,
            "error": (!success).then_some(result_text.clone()),
        });
        let mut content_blocks =
            texts.into_iter().map(|text| ToolResultContentDto::Text { text }).collect::<Vec<_>>();
        content_blocks.push(ToolResultContentDto::Json { value: data.clone() });

        ToolCallResult {
            data,
            model_result: Value::String(result_text.clone()),
            content_blocks,
            render_hint: Some(ToolRenderHint {
                title: Some("RustCodeGraph".to_string()),
                kind: Some("rustcodegraph".to_string()),
                summary: Some(format!("{} query", operation.as_str())),
                metadata: json!({
                    "tool_id": TOOL_ID,
                    "canonical_tool_id": TOOL_ID,
                    "operation": operation.as_str(),
                    "library_tool": operation.library_tool(),
                    "success": success,
                }),
            }),
            telemetry: Some(ToolCallTelemetry { success, ..ToolCallTelemetry::default() }),
            ..ToolCallResult::default()
        }
    }
}

impl Drop for RustCodeGraphTool {
    fn drop(&mut self) {
        let _ = self.lifecycle.commands.send(LifecycleCommand::Shutdown);
        let mut thread = match self.lifecycle.thread.lock() {
            Ok(thread) => thread,
            Err(poisoned) => poisoned.into_inner(),
        };
        if let Some(thread) = thread.take() {
            let _ = thread.join();
        }
    }
}

#[async_trait]
impl Tool for RustCodeGraphTool {
    fn name(&self) -> &str {
        TOOL_ID
    }

    fn description(&self) -> &str {
        include_str!("rustcodegraph.txt")
    }

    fn parameters_schema(&self) -> Value {
        Self::schema()
    }

    fn spec(&self) -> ToolSpec {
        ToolSpec::new(TOOL_ID, self.description(), self.parameters_schema())
            .with_display_name(TOOL_ID)
            .with_read_only(true)
            .with_destructive(false)
            .with_concurrency_safe(true)
            .with_requires_user_interaction(false)
            .with_strict(true)
    }

    fn validate_input(&self, input: Value) -> anyhow::Result<Value> {
        parse_args(input.clone())?;
        Ok(input)
    }

    async fn execute(&self, input: Value) -> anyhow::Result<ToolResult> {
        let result = self.call(input).await?;
        Ok(ToolResult {
            success: result.is_success(),
            output: result.model_text(),
            error: result.error_text(),
        })
    }

    async fn call(&self, input: Value) -> anyhow::Result<ToolCallResult> {
        let args = parse_args(input)?;
        let result = self.execute_query(&args).await?;
        Ok(Self::map_result(args.operation, result))
    }
}

fn parse_args(input: Value) -> anyhow::Result<Args> {
    if !input.is_object() {
        anyhow::bail!("Missing or invalid parameters: expected object");
    }
    let mut args: Args = serde_json::from_value(input)
        .map_err(|error| anyhow::anyhow!("Missing or invalid parameters: {error}"))?;
    args.validate()?;
    Ok(args)
}

fn required_text(value: Option<String>, field: &str) -> anyhow::Result<String> {
    optional_text(value).ok_or_else(|| anyhow::anyhow!("Missing or empty {field}"))
}

fn optional_text(value: Option<String>) -> Option<String> {
    value.map(|text| text.trim().to_string()).filter(|text| !text.is_empty())
}

fn optional_relative_path(value: Option<String>, field: &str) -> anyhow::Result<Option<String>> {
    let Some(value) = optional_text(value) else {
        return Ok(None);
    };
    let path = Path::new(&value);
    if path.is_absolute()
        || path.components().any(|component| {
            matches!(component, Component::ParentDir | Component::RootDir | Component::Prefix(_))
        })
    {
        anyhow::bail!("{field} must be relative to the current workspace");
    }
    Ok(Some(value))
}

fn insert_string(args: &mut Map<String, Value>, key: &str, value: Option<&String>) {
    if let Some(value) = value {
        args.insert(key.to_string(), Value::String(value.clone()));
    }
}

fn insert_number(args: &mut Map<String, Value>, key: &str, value: Option<u64>) {
    if let Some(value) = value {
        args.insert(key.to_string(), Value::Number(value.into()));
    }
}

fn insert_bool(args: &mut Map<String, Value>, key: &str, value: Option<bool>) {
    if let Some(value) = value {
        args.insert(key.to_string(), Value::Bool(value));
    }
}
