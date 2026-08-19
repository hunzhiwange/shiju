//! # 测试辅助模块
//!
//! 本模块为 Agent 单元测试提供一组可复用的模拟实现和辅助函数。
//!
//! ## 主要功能
//!
//! - **模拟 Provider 实现**：用于测试 Agent 与 LLM 提供者的交互逻辑
//! - **模拟 Tool 实现**：用于测试工具执行和错误处理路径
//! - **辅助构建函数**：快速创建 Memory、Observer、Agent 等测试组件
//! - **响应构造器**：简化 ChatResponse 的创建过程
//!
//! ## 设计原则
//!
//! - 所有模拟实现均为 `pub(super)`，仅限测试模块内部使用
//! - 使用 `Mutex` 保证线程安全的可变状态
//! - 支持 WASM 目标平台的异步 trait 实现

use crate::app::agent::agent::agent::Agent;
use crate::app::agent::agent::dispatcher::{ToolDispatcher, ToolExecutionResult};
use crate::app::agent::config::MemoryConfig;
use crate::app::agent::memory::{self, Memory};
use crate::app::agent::observability::{NoopObserver, Observer};
use crate::app::agent::providers::{ChatMessage, ChatRequest, ChatResponse, Provider, ToolCall};
use crate::app::agent::tools::{Tool, ToolResult};
use anyhow::Result;
use async_trait::async_trait;
use std::sync::{Arc, Mutex};
use vw_config_types::agent::AgentDefinitionConfig;

/// 脚本化 LLM Provider
///
/// 按预设顺序返回响应的模拟提供者。当响应队列耗尽时，返回简单的 "done" 文本响应。
/// 该实现记录所有接收到的请求，可用于断言测试。
///
/// # 使用场景
///
/// - 测试多轮对话逻辑
/// - 验证请求序列和参数传递
/// - 模拟 LLM 的确定性响应行为
///
/// # 示例
///
/// ```ignore
/// let responses = vec![text_response("Hello"), text_response("World")];
/// let provider = ScriptedProvider::new(responses);
/// // 第一次调用返回 "Hello"，第二次返回 "World"，之后返回 "done"
/// ```
pub(super) struct ScriptedProvider {
    /// 预设的响应队列，使用互斥锁保证线程安全
    responses: Mutex<Vec<ChatResponse>>,
    /// 已记录的所有请求历史，用于断言验证
    requests: Mutex<Vec<Vec<ChatMessage>>>,
}

impl ScriptedProvider {
    /// 创建新的脚本化 Provider
    ///
    /// # 参数
    ///
    /// - `responses`: 预设的响应队列，将按顺序依次返回
    ///
    /// # 返回值
    ///
    /// 返回初始化后的 `ScriptedProvider` 实例，请求记录为空
    pub(super) fn new(responses: Vec<ChatResponse>) -> Self {
        Self { responses: Mutex::new(responses), requests: Mutex::new(Vec::new()) }
    }

    /// 获取已处理的请求数量
    ///
    /// # 返回值
    ///
    /// 返回到目前为止已经接收并记录的请求数量
    pub(super) fn request_count(&self) -> usize {
        self.requests.lock().unwrap().len()
    }
}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl Provider for ScriptedProvider {
    /// 简化的聊天接口（带系统提示）
    ///
    /// 本实现为简化版本，始终返回固定的 "fallback" 字符串。
    /// 主要用于 Provider trait 的完整性，实际测试应使用 `chat` 方法。
    async fn chat_with_system(
        &self,
        _system_prompt: Option<&str>,
        _message: &str,
        _model: &str,
        _temperature: f64,
    ) -> Result<String> {
        Ok("fallback".into())
    }

    /// 完整的聊天接口
    ///
    /// 记录请求并从预设队列中弹出响应。队列为空时返回 "done" 文本响应。
    ///
    /// # 行为说明
    ///
    /// 1. 将当前请求的消息列表追加到 `requests` 历史记录
    /// 2. 从 `responses` 队列头部移除并返回一个响应
    /// 3. 如果队列为空，构造并返回默认的 "done" 响应
    async fn chat(
        &self,
        request: ChatRequest<'_>,
        _model: &str,
        _temperature: f64,
    ) -> Result<ChatResponse> {
        // 记录本次请求的所有消息
        self.requests.lock().unwrap().push(request.messages.to_vec());

        let mut guard = self.responses.lock().unwrap();
        if guard.is_empty() {
            // 队列已空，返回默认的完成响应
            return Ok(ChatResponse {
                text: Some("done".into()),
                tool_calls: vec![],
                usage: None,
                reasoning_content: None,
            });
        }
        // 从队列头部移除并返回响应
        Ok(guard.remove(0))
    }
}

/// 总是失败的 Provider
///
/// 所有方法调用都会返回错误的模拟提供者。
/// 适用于测试错误处理和降级逻辑。
///
/// # 使用场景
///
/// - 测试 Provider 错误时的 Agent 行为
/// - 验证错误传播和恢复机制
/// - 测试超时和重试逻辑
pub(super) struct FailingProvider;

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl Provider for FailingProvider {
    /// 总是返回错误的聊天接口（带系统提示）
    async fn chat_with_system(
        &self,
        _system_prompt: Option<&str>,
        _message: &str,
        _model: &str,
        _temperature: f64,
    ) -> Result<String> {
        anyhow::bail!("provider error")
    }

    /// 总是返回错误的完整聊天接口
    async fn chat(
        &self,
        _request: ChatRequest<'_>,
        _model: &str,
        _temperature: f64,
    ) -> Result<ChatResponse> {
        anyhow::bail!("provider error")
    }
}

/// 回显工具
///
/// 简单的测试工具，将输入参数原样返回作为输出。
/// 用于验证工具调用流程的端到端连通性。
///
/// # 使用场景
///
/// - 验证工具调用参数传递正确性
/// - 测试工具结果回传机制
/// - 作为最小可运行工具的示例
///
/// # 参数模式
///
/// 接受一个 JSON 对象，包含 `message` 字符串字段：
/// ```json
/// {"message": "要回显的内容"}
/// ```
pub(super) struct EchoTool;

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl Tool for EchoTool {
    /// 工具名称标识符
    fn name(&self) -> &str {
        "echo"
    }

    /// 工具功能描述
    fn description(&self) -> &str {
        "Echoes the input"
    }

    /// 参数 JSON Schema 定义
    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "message": {"type": "string"}
            }
        })
    }

    /// 执行回显操作
    ///
    /// # 参数
    ///
    /// - `args`: 包含 `message` 字段的 JSON 对象
    ///
    /// # 返回值
    ///
    /// 返回成功的 ToolResult，输出为 message 字段内容，缺失时为 "(empty)"
    async fn execute(&self, args: serde_json::Value) -> Result<ToolResult> {
        let msg = args.get("message").and_then(|v| v.as_str()).unwrap_or("(empty)").to_string();
        Ok(ToolResult { success: true, output: msg, error: None })
    }
}

/// 总是失败的工具
///
/// 执行时始终返回失败结果的模拟工具。
/// 用于测试工具执行失败的错误处理路径。
///
/// # 使用场景
///
/// - 测试 Agent 对工具失败的处理
/// - 验证错误信息传播
/// - 测试重试或降级逻辑
pub(super) struct FailingTool;

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl Tool for FailingTool {
    /// 工具名称标识符
    fn name(&self) -> &str {
        "fail"
    }

    /// 工具功能描述
    fn description(&self) -> &str {
        "Always fails"
    }

    /// 参数 JSON Schema 定义（无参数）
    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({"type": "object"})
    }

    /// 执行失败操作
    ///
    /// # 返回值
    ///
    /// 始终返回失败的 ToolResult，包含 "intentional failure" 错误信息
    async fn execute(&self, _args: serde_json::Value) -> Result<ToolResult> {
        Ok(ToolResult {
            success: false,
            output: String::new(),
            error: Some("intentional failure".into()),
        })
    }
}

/// 会恐慌的工具
///
/// 执行时会抛出错误的模拟工具，用于测试异常传播机制。
/// 与 FailingTool 不同，此工具使用 `bail!` 宏产生 Error 而非返回失败结果。
///
/// # 使用场景
///
/// - 测试 Agent 对工具异常的容错能力
/// - 验证错误边界和隔离机制
/// - 测试日志和监控对异常的捕获
pub(super) struct PanickingTool;

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl Tool for PanickingTool {
    /// 工具名称标识符
    fn name(&self) -> &str {
        "panicker"
    }

    /// 工具功能描述
    fn description(&self) -> &str {
        "Panics on execution"
    }

    /// 参数 JSON Schema 定义（无参数）
    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({"type": "object"})
    }

    /// 执行并抛出严重错误
    ///
    /// # 返回值
    ///
    /// 返回包含 "catastrophic tool failure" 消息的 anyhow::Error
    async fn execute(&self, _args: serde_json::Value) -> Result<ToolResult> {
        anyhow::bail!("catastrophic tool failure")
    }
}

/// 计数工具
///
/// 记录并报告调用次数的工具实现。
/// 使用共享的原子计数器，可在多个位置查询调用次数。
///
/// # 使用场景
///
/// - 验证工具被调用的次数
/// - 测试 Agent 循环中的工具调度逻辑
/// - 验证去重或限流机制
///
/// # 示例
///
/// ```ignore
/// let (tool, counter) = CountingTool::new();
/// // 执行 tool...
/// assert_eq!(*counter.lock().unwrap(), 1);
/// ```
pub(super) struct CountingTool {
    /// 共享的调用计数器，使用 Arc 可在工具外部访问
    count: Arc<Mutex<usize>>,
}

impl CountingTool {
    /// 创建新的计数工具
    ///
    /// # 返回值
    ///
    /// 返回元组 (工具实例, 共享计数器引用)：
    /// - 第一个元素：可直接注册到 Agent 的工具实例
    /// - 第二个元素：用于断言调用次数的 Arc 句柄
    ///
    /// # 示例
    ///
    /// ```ignore
    /// let (tool, counter) = CountingTool::new();
    /// // 将 tool 添加到 Agent，通过 counter 检查调用次数
    /// ```
    pub(super) fn new() -> (Self, Arc<Mutex<usize>>) {
        let count = Arc::new(Mutex::new(0));
        (Self { count: count.clone() }, count)
    }
}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl Tool for CountingTool {
    /// 工具名称标识符
    fn name(&self) -> &str {
        "counter"
    }

    /// 工具功能描述
    fn description(&self) -> &str {
        "Counts calls"
    }

    /// 参数 JSON Schema 定义（无参数）
    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({"type": "object"})
    }

    /// 执行计数操作
    ///
    /// # 行为说明
    ///
    /// 1. 获取互斥锁
    /// 2. 递增计数器
    /// 3. 返回包含当前调用次数的成功结果
    ///
    /// # 返回值
    ///
    /// 返回成功的 ToolResult，输出格式为 "call #N"，其中 N 为当前调用序号
    async fn execute(&self, _args: serde_json::Value) -> Result<ToolResult> {
        let mut c = self.count.lock().unwrap();
        *c += 1;
        Ok(ToolResult { success: true, output: format!("call #{}", *c), error: None })
    }
}

/// 创建无后端 Memory 实例
///
/// 使用 "none" 后端创建不进行持久化的内存实例。
/// 适用于不需要记忆功能的测试场景。
///
/// # 返回值
///
/// 返回包装在 Arc 中的 Memory trait 对象
///
/// # 注意事项
///
/// - 使用系统临时目录作为工作空间
/// - 不进行任何实际的记忆存储操作
pub(super) fn make_memory() -> Arc<dyn Memory> {
    let cfg = MemoryConfig { backend: "none".into(), ..MemoryConfig::default() };
    Arc::from(memory::create_memory(&cfg, &std::env::temp_dir(), None).unwrap())
}

/// 创建 SQLite 后端 Memory 实例
///
/// 使用临时目录中的 SQLite 数据库作为记忆后端。
/// 适用于需要测试记忆持久化的场景。
///
/// # 返回值
///
/// 返回元组 (Memory 实例, 临时目录句柄)：
/// - 第一个元素：SQLite 后端的 Memory trait 对象
/// - 第二个元素：临时目录句柄，保持有效以防止目录被自动清理
///
/// # 注意事项
///
/// - TempDir 必须保持存活，否则数据库文件会被删除
/// - 每次调用创建独立的临时目录，测试之间相互隔离
pub(super) fn make_sqlite_memory() -> (Arc<dyn Memory>, tempfile::TempDir) {
    let tmp = tempfile::TempDir::new().unwrap();
    let cfg = MemoryConfig { backend: "sqlite".into(), ..MemoryConfig::default() };
    let mem = Arc::from(memory::create_memory(&cfg, tmp.path(), None).unwrap());
    (mem, tmp)
}

/// 创建空操作 Observer 实例
///
/// 返回不执行任何观测操作的 Observer 实现。
/// 适用于不需要观测功能的测试场景。
///
/// # 返回值
///
/// 返回包装在 Arc 中的 NoopObserver 实例
pub(super) fn make_observer() -> Arc<dyn Observer> {
    Arc::from(NoopObserver {})
}

/// 使用自定义组件构建 Agent
///
/// 使用指定的 Provider、工具列表和分发器构建 Agent 实例。
/// 自动配置无后端 Memory 和空操作 Observer。
///
/// # 参数
///
/// - `provider`: LLM 提供者实现
/// - `tools`: 工具列表
/// - `_dispatcher`: 工具分发器（当前未使用，保留用于未来扩展）
///
/// # 返回值
///
/// 返回构建完成的 Agent 实例
///
/// # Panics
///
/// 如果 Agent 构建失败会 panic（测试环境中不可恢复）
pub(super) fn build_agent_with(
    provider: Box<dyn Provider>,
    tools: Vec<Box<dyn Tool>>,
    _dispatcher: Box<dyn ToolDispatcher>,
) -> Agent {
    Agent::builder()
        .provider(provider)
        .tools(tools)
        .memory(make_memory())
        .observer(make_observer())
        .workspace_dir(std::env::temp_dir())
        .build()
        .unwrap()
}

/// 使用自定义 Memory 构建带记忆功能的 Agent
///
/// 构建支持记忆持久化的 Agent 实例，可配置自动保存行为。
///
/// # 参数
///
/// - `provider`: LLM 提供者实现
/// - `tools`: 工具列表
/// - `mem`: Memory 后端实现
/// - `auto_save`: 是否启用自动保存记忆功能
///
/// # 返回值
///
/// 返回配置了指定 Memory 的 Agent 实例
///
/// # Panics
///
/// 如果 Agent 构建失败会 panic
///
/// # 使用场景
///
/// - 测试记忆的存储和检索
/// - 验证跨会话的上下文保持
/// - 测试自动保存功能
pub(super) fn build_agent_with_memory(
    provider: Box<dyn Provider>,
    tools: Vec<Box<dyn Tool>>,
    mem: Arc<dyn Memory>,
    auto_save: bool,
) -> Agent {
    Agent::builder()
        .provider(provider)
        .tools(tools)
        .memory(mem)
        .observer(make_observer())
        .workspace_dir(std::env::temp_dir())
        .auto_save(auto_save)
        .build()
        .unwrap()
}

/// 使用完整配置构建 Agent
///
/// 使用自定义的 AgentDefinitionConfig 构建实例，支持细粒度的行为配置。
///
/// # 参数
///
/// - `provider`: LLM 提供者实现
/// - `tools`: 工具列表
/// - `config`: Agent 配置对象
///
/// # 返回值
///
/// 返回应用了指定配置的 Agent 实例
///
/// # Panics
///
/// 如果 Agent 构建失败会 panic
///
/// # 使用场景
///
/// - 测试特定配置参数的影响
/// - 验证配置验证逻辑
/// - 测试不同配置组合的行为差异
pub(super) fn build_agent_with_config(
    provider: Box<dyn Provider>,
    tools: Vec<Box<dyn Tool>>,
    config: AgentDefinitionConfig,
) -> Agent {
    Agent::builder()
        .provider(provider)
        .tools(tools)
        .memory(make_memory())
        .observer(make_observer())
        .workspace_dir(std::env::temp_dir())
        .config(config)
        .build()
        .unwrap()
}

/// 创建包含工具调用的 ChatResponse（原生格式）
///
/// 构造带有工具调用的响应对象，用于模拟 LLM 返回的工具调用场景。
///
/// # 参数
///
/// - `calls`: 工具调用列表，每个元素包含工具名称、参数和调用 ID
///
/// # 返回值
///
/// 返回配置了工具调用的 ChatResponse：
/// - `text`: 空字符串（某些提供者可能同时返回文本）
/// - `tool_calls`: 指定的工具调用列表
/// - `usage`: None（不模拟 token 统计）
/// - `reasoning_content`: None（不包含推理过程）
///
/// # 示例
///
/// ```ignore
/// let call = ToolCall { id: "1".into(), name: "echo".into(), arguments: "{}".into() };
/// let response = tool_response(vec![call]);
/// ```
pub(super) fn tool_response(calls: Vec<ToolCall>) -> ChatResponse {
    ChatResponse {
        text: Some(String::new()),
        tool_calls: calls,
        usage: None,
        reasoning_content: None,
    }
}

/// 创建纯文本 ChatResponse
///
/// 构造仅包含文本内容的响应对象，用于模拟 LLM 的普通文本回复。
///
/// # 参数
///
/// - `text`: 响应文本内容
///
/// # 返回值
///
/// 返回纯文本的 ChatResponse：
/// - `text`: 指定的文本内容
/// - `tool_calls`: 空列表
/// - `usage`: None
/// - `reasoning_content`: None
///
/// # 示例
///
/// ```ignore
/// let response = text_response("Hello, world!");
/// ```
pub(super) fn text_response(text: &str) -> ChatResponse {
    ChatResponse {
        text: Some(text.into()),
        tool_calls: vec![],
        usage: None,
        reasoning_content: None,
    }
}

/// 创建 XML 风格的工具调用响应
///
/// 构造文本字段中包含 XML 格式工具调用的响应对象。
/// 某些 LLM 提供者使用这种格式而非结构化的 `tool_calls` 字段。
///
/// # 参数
///
/// - `name`: 工具名称
/// - `args`: JSON 格式的工具参数字符串（需包含引号）
///
/// # 返回值
///
/// 返回包含 XML 格式工具调用的 ChatResponse：
/// - `text`: 包含 `<tool_call>` 标签的 XML 文本
/// - `tool_calls`: 空列表（工具调用信息在文本中）
/// - `usage`: None
/// - `reasoning_content`: None
///
/// # 示例
///
/// ```ignore
/// let response = xml_tool_response("echo", r#""hello""#);
/// // text 字段内容：
/// // <tool_call>
/// // {"name": "echo", "arguments": "hello"}
/// // </tool_call>
/// ```
pub(super) fn xml_tool_response(name: &str, args: &str) -> ChatResponse {
    ChatResponse {
        text: Some(format!(
            "<tool_call>\n{{\"name\": \"{name}\", \"argumensrc/app/agent/agent/loop_/instructions.rsts\": {args}}}\n</tool_call>"
        )),
        tool_calls: vec![],
        usage: None,
        reasoning_content: None,
    }
}

/// 构建工具执行结果
///
/// 创建标准化的工具执行结果对象，用于断言和验证工具执行输出。
///
/// # 参数
///
/// - `name`: 工具名称
/// - `output`: 工具输出内容
/// - `success`: 执行是否成功
/// - `tool_call_id`: 可选的工具调用 ID（用于关联请求和响应）
///
/// # 返回值
///
/// 返回完整的 ToolExecutionResult 实例
///
/// # 示例
///
/// ```ignore
/// let result = build_tool_execution_result("echo", "hello", true, Some("call_123"));
/// assert_eq!(result.name, "echo");
/// assert!(result.success);
/// ```
pub(super) fn build_tool_execution_result(
    name: &str,
    output: &str,
    success: bool,
    tool_call_id: Option<&str>,
) -> ToolExecutionResult {
    ToolExecutionResult {
        name: name.into(),
        output: output.into(),
        success,
        tool_call_id: tool_call_id.map(|id| id.into()),
    }
}
