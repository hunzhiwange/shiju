//! 待办事项管理工具
//!
//! 管理编码会话中的任务列表。支持读取和写入待办事项，包括状态跟踪
//! （pending/in_progress/completed/cancelled）和优先级设置。

mod normalize;
mod schema;
mod store;

#[cfg(test)]
#[path = "normalize_tests.rs"]
mod normalize_tests;
#[cfg(test)]
#[path = "schema_tests.rs"]
mod schema_tests;
#[cfg(test)]
#[path = "store_tests.rs"]
mod store_tests;
#[cfg(test)]
mod tests;

use crate::app::agent::tools::traits::{
    Tool, ToolCallResult, ToolCallTelemetry, ToolRenderHint, ToolResult, ToolSpec,
};
use async_trait::async_trait;
use serde_json::{Value, json};
use std::sync::Arc;
use vw_api_types::tools::ToolResultContentDto;

pub use vw_shared::todo::{LegacyTodo, Todo};

use self::store::{read_for_tool, read_for_ui, write_todos};

pub(crate) fn read_session_todos(session: &str) -> Vec<Todo> {
    read_for_tool(session).unwrap_or_default()
}

pub(crate) fn write_session_todos(session: &str, todos: Vec<Todo>) -> anyhow::Result<String> {
    write_todos(session, json!({ "todos": todos, "merge": false }))
}

#[cfg(test)]
pub(crate) fn write_session_todos_for_test(
    session: &str,
    todos: Vec<Todo>,
) -> anyhow::Result<String> {
    write_session_todos(session, todos)
}

/// 读取当前会话的待办事项列表
///
/// # 参数
/// - `ctx`: 工具执行上下文，包含会话标识等信息
///
/// # 返回值
/// - `Ok(String)`: JSON 格式的待办事项列表（美化输出）
/// - `Err(ToolCallError)`: 序列化失败时的错误
pub fn read(
    ctx: &crate::app::agent::tools::ToolRuntimeContext,
) -> Result<String, crate::app::agent::tools::ToolCallError> {
    let todos = read_for_ui(&ctx.session);
    serde_json::to_string_pretty(&todos)
        .map_err(|e| crate::app::agent::tools::ToolCallError::Failed(e.to_string()))
}

/// 写入/更新当前会话的待办事项列表
///
/// # 参数
/// - `input`: JSON 格式的输入字符串，包含待办事项数据和可选的合并标志
/// - `ctx`: 工具执行上下文，包含会话标识等信息
///
/// # 返回值
/// - `Ok(String)`: 更新后的待办事项列表（JSON 格式）
/// - `Err(ToolCallError)`: 参数解析失败或写入失败时的错误
pub fn write(
    input: &str,
    ctx: &crate::app::agent::tools::ToolRuntimeContext,
) -> Result<String, crate::app::agent::tools::ToolCallError> {
    let raw = input.trim();
    if raw.is_empty() {
        return Err(crate::app::agent::tools::ToolCallError::Failed("缺少参数".to_string()));
    }
    let args: serde_json::Value = serde_json::from_str(raw)
        .map_err(|e| crate::app::agent::tools::ToolCallError::Failed(e.to_string()))?;
    write_todos(&ctx.session, args)
        .map_err(|e| crate::app::agent::tools::ToolCallError::Failed(e.to_string()))
}

/// 待办事项读取工具
///
/// 实现Tool trait，提供读取当前会话待办事项列表的能力。
/// 无需参数，返回所有待办事项的 JSON 格式数据。
#[derive(Clone)]
pub struct TodoReadTool {
    /// 关联的会话标识符
    session: String,
}

impl TodoReadTool {
    /// 创建新的待办事项读取工具实例
    ///
    /// # 参数
    /// - `session`: 会话标识符，用于隔离不同会话的待办事项
    ///
    /// # 示例
    /// ```ignore
    /// let tool = TodoReadTool::new("session-123".to_string());
    /// ```
    pub fn new(session: String) -> Self {
        Self { session }
    }
}

/// 待办事项读取工具的 Tool trait 实现
///
/// 提供异步读取能力，自动从磁盘加载并规范化数据。
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl Tool for TodoReadTool {
    /// 返回工具名称
    fn name(&self) -> &str {
        "todoread"
    }

    /// 返回工具描述（从外部文件加载）
    fn description(&self) -> &str {
        include_str!("todoread.txt")
    }

    /// 返回参数 schema（读取工具无需参数）
    fn parameters_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {}
        })
    }

    fn spec(&self) -> ToolSpec {
        ToolSpec::new(
            crate::app::agent::tools::TODO_READ_TOOL_ID,
            self.description(),
            self.parameters_schema(),
        )
        .with_display_name(crate::app::agent::tools::TODO_READ_TOOL_ID)
        .with_aliases(vec![crate::app::agent::tools::TODO_READ_TOOL_ALIAS.to_string()])
        .with_read_only(true)
        .with_destructive(false)
        .with_concurrency_safe(true)
        .with_requires_user_interaction(false)
        .with_strict(true)
    }

    async fn call(&self, _input: Value) -> anyhow::Result<ToolCallResult> {
        let todos = read_for_tool(&self.session)?;
        let todo_count = todos.len();
        let output = serde_json::to_string_pretty(&todos)?;
        let data = json!({ "todos": todos });

        Ok(ToolCallResult {
            data: data.clone(),
            model_result: Value::String(output),
            content_blocks: vec![ToolResultContentDto::Json { value: data }],
            render_hint: Some(ToolRenderHint {
                title: Some(crate::app::agent::tools::TODO_READ_TOOL_ID.to_string()),
                kind: Some("todo_read".to_string()),
                summary: Some(format!("Read {} todo(s)", todo_count)),
                metadata: json!({ "count": todo_count }),
            }),
            telemetry: Some(ToolCallTelemetry { success: true, ..ToolCallTelemetry::default() }),
            ..ToolCallResult::default()
        })
    }

    /// 执行读取操作
    ///
    /// 从内存缓存或磁盘加载待办事项，执行规范化处理后返回。
    ///
    /// # 参数
    /// - `_args`: 未使用（读取操作无需参数）
    ///
    /// # 返回值
    /// - `Ok(ToolResult)`: 成功时包含美化格式的 JSON 待办事项列表
    /// - `Err(anyhow::Error)`: 锁获取失败或序列化失败
    async fn execute(&self, _args: serde_json::Value) -> anyhow::Result<ToolResult> {
        let normalized = read_for_tool(&self.session)?;
        let output = serde_json::to_string_pretty(&normalized)?;
        Ok(ToolResult { success: true, output, error: None })
    }
}

/// 待办事项写入工具
///
/// 实现Tool trait，提供写入/更新待办事项列表的能力。
/// 支持两种模式：完全替换和增量合并。
/// 包含速率限制保护，防止滥用。
#[derive(Clone)]
pub struct TodoWriteTool {
    /// 关联的会话标识符
    session: String,
    /// 安全策略引用，用于速率限制检查
    security: Arc<crate::app::agent::security::SecurityPolicy>,
}

impl TodoWriteTool {
    /// 创建新的待办事项写入工具实例
    ///
    /// # 参数
    /// - `session`: 会话标识符，用于隔离不同会话的待办事项
    /// - `security`: 安全策略引用，用于执行速率限制
    ///
    /// # 示例
    /// ```ignore
    /// let security = Arc::new(SecurityPolicy::default());
    /// let tool = TodoWriteTool::new("session-123".to_string(), security);
    /// ```
    pub fn new(
        session: String,
        security: Arc<crate::app::agent::security::SecurityPolicy>,
    ) -> Self {
        Self { session, security }
    }
}

/// 待办事项写入工具的 Tool trait 实现
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl Tool for TodoWriteTool {
    /// 返回工具名称
    fn name(&self) -> &str {
        "todowrite"
    }

    /// 返回工具描述（从外部文件加载）
    fn description(&self) -> &str {
        include_str!("todowrite.txt")
    }

    /// 返回参数 JSON Schema
    ///
    /// 定义了 todos 数组和 merge 布尔值两个参数：
    /// - todos: 待办事项数组，每项包含 id、content、status、priority 字段
    /// - merge: 是否使用合并模式（可选，默认 false）
    fn parameters_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "additionalProperties": false,
            "properties": {
                "todos": {
                    "type": "array",
                    "items": {
                        "type": "object",
                        "additionalProperties": false,
                        "properties": {
                            "id": {
                                "type": ["string", "number"]
                            },
                            "content": {
                                "type": "string"
                            },
                            "status": {
                                "type": "string"
                            },
                            "priority": {
                                "type": "string"
                            }
                        }
                    }
                },
                "merge": {
                    "type": "boolean"
                }
            },
            "required": ["todos"]
        })
    }

    fn spec(&self) -> ToolSpec {
        ToolSpec::new(
            crate::app::agent::tools::TODO_WRITE_TOOL_ID,
            self.description(),
            self.parameters_schema(),
        )
        .with_display_name(crate::app::agent::tools::TODO_WRITE_TOOL_ID)
        .with_aliases(vec![crate::app::agent::tools::TODO_WRITE_TOOL_ALIAS.to_string()])
        .with_read_only(false)
        .with_destructive(false)
        .with_concurrency_safe(false)
        .with_requires_user_interaction(false)
        .with_strict(true)
    }

    async fn call(&self, input: Value) -> anyhow::Result<ToolCallResult> {
        if self.security.is_rate_limited() {
            let mut result = ToolCallResult::from_legacy_result(ToolResult {
                success: false,
                output: String::new(),
                error: Some("Rate limit exceeded".into()),
            });
            result.render_hint = Some(ToolRenderHint {
                title: Some(crate::app::agent::tools::TODO_WRITE_TOOL_ID.to_string()),
                kind: Some("todo_write".to_string()),
                summary: Some("Todo update blocked by rate limit".to_string()),
                metadata: Value::Object(Default::default()),
            });
            return Ok(result);
        }
        if !self.security.record_action() {
            let mut result = ToolCallResult::from_legacy_result(ToolResult {
                success: false,
                output: String::new(),
                error: Some("Rate limit exceeded".into()),
            });
            result.render_hint = Some(ToolRenderHint {
                title: Some(crate::app::agent::tools::TODO_WRITE_TOOL_ID.to_string()),
                kind: Some("todo_write".to_string()),
                summary: Some("Todo update blocked by rate limit".to_string()),
                metadata: Value::Object(Default::default()),
            });
            return Ok(result);
        }

        let args: schema::WriteArgs = serde_json::from_value(input.clone())?;
        let previous = read_for_tool(&self.session)?;
        let output = write_todos(&self.session, input)?;
        let next = read_for_tool(&self.session)?;
        let next_count = next.len();
        let data = json!({
            "oldTodos": previous,
            "newTodos": next,
            "merge": args.merge,
        });

        Ok(ToolCallResult {
            data: data.clone(),
            model_result: Value::String(output),
            content_blocks: vec![ToolResultContentDto::Json { value: data }],
            render_hint: Some(ToolRenderHint {
                title: Some(crate::app::agent::tools::TODO_WRITE_TOOL_ID.to_string()),
                kind: Some("todo_write".to_string()),
                summary: Some(format!("Updated {} todo(s)", next_count)),
                metadata: json!({
                    "count": next_count,
                    "merge": args.merge,
                }),
            }),
            telemetry: Some(ToolCallTelemetry { success: true, ..ToolCallTelemetry::default() }),
            ..ToolCallResult::default()
        })
    }

    /// 执行写入操作
    ///
    /// 首先检查速率限制，然后根据 merge 标志执行替换或合并操作。
    ///
    /// # 参数
    /// - `args`: JSON 格式的参数，包含 todos 数组和可选的 merge 标志
    ///
    /// # 返回值
    /// - `Ok(ToolResult)`: 成功时包含更新后的待办事项列表
    /// - 超过速率限制时返回错误
    async fn execute(&self, args: serde_json::Value) -> anyhow::Result<ToolResult> {
        // 检查是否超过速率限制
        if self.security.is_rate_limited() {
            return Ok(ToolResult {
                success: false,
                output: String::new(),
                error: Some("Rate limit exceeded".into()),
            });
        }
        // 记录本次操作（用于速率限制计数）
        if !self.security.record_action() {
            return Ok(ToolResult {
                success: false,
                output: String::new(),
                error: Some("Rate limit exceeded".into()),
            });
        }

        let output = write_todos(&self.session, args)?;

        Ok(ToolResult { success: true, output, error: None })
    }
}
