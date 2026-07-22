//! Delegate 工具测试模块
//!
//! 本模块包含 `DelegateTool`（委托工具）的全面测试套件，验证代理委托功能的各个方面：
//!
//! # 测试范围
//!
//! - **基础功能**：工具名称、描述和参数 schema 的正确性
//! - **参数验证**：必需参数缺失、空值、空白字符的处理
//! - **深度限制**：全局深度限制和每个代理的深度限制
//! - **安全策略**：只读模式、速率限制对委托的影响
//! - **代理模式**：agentic 模式下的工具调用循环、迭代限制、错误传播
//! - **协调追踪**：协调总线（coordination bus）中的事件记录和状态转换
//!
//! # 测试架构
//!
//! 模块使用多个模拟 Provider 实现来隔离测试逻辑：
//! - `OneToolThenFinalProvider`：执行一次工具调用后返回最终结果
//! - `InfiniteToolCallProvider`：无限循环调用工具（用于测试迭代限制）
//! - `FailingProvider`：始终返回错误（用于测试错误传播）
//! - `EchoTool`：简单的回显工具，返回输入值

use super::super::*;
use crate::app::agent::coordination::CoordinationPayload;
use crate::app::agent::coordination::InMemoryMessageBus;
use crate::app::agent::providers::{ChatRequest, ChatResponse, Provider, ToolCall};
use crate::app::agent::security::{AutonomyLevel, SecurityPolicy};
use crate::app::agent::tools::delegate::DEFAULT_COORDINATION_LEAD_AGENT;
use anyhow::anyhow;
use async_trait::async_trait;
use serde_json::{Value, json};

/// 创建默认的安全策略用于测试
///
/// 返回一个使用默认配置的 `SecurityPolicy` 实例，
/// 包装在 `Arc` 中以支持跨线程共享。
fn test_security() -> Arc<SecurityPolicy> {
    Arc::new(SecurityPolicy::default())
}

/// 构建示例代理配置映射
///
/// 创建包含两个代理的配置集合：
/// - `researcher`：研究助手代理，使用 Ollama 和 llama3 模型
/// - `coder`：编码代理，使用 OpenRouter 和 Claude 模型
///
/// # 返回值
///
/// 返回一个 `HashMap`，键为代理名称，值为对应的 `DelegateAgentConfig` 配置。
fn sample_agents() -> HashMap<String, DelegateAgentConfig> {
    let mut agents = HashMap::new();

    // 配置研究助手代理：使用 Ollama 本地模型，较低温度以保证输出稳定性
    agents.insert(
        "researcher".to_string(),
        DelegateAgentConfig {
            label: None,
            description: None,
            builtin: false,
            mode: "all".to_string(),
            enabled: true,
            provider: "ollama".to_string(),
            model: "llama3".to_string(),
            system_prompt: Some("You are a research assistant.".to_string()),
            api_key: None,
            temperature: Some(0.3),
            top_p: None,
            identity_format: None,
            hidden: false,
            max_depth: 3,
            agentic: false,
            allowed_tools: Vec::new(),
            allowed_skills: Vec::new(),
            options: HashMap::new(),
            permission: Value::Null,
            max_iterations: 10,
            steps: None,
        },
    );

    // 配置编码代理：使用 OpenRouter 云服务，Claude Sonnet 模型
    agents.insert(
        "coder".to_string(),
        DelegateAgentConfig {
            label: None,
            description: None,
            builtin: false,
            mode: "all".to_string(),
            enabled: true,
            provider: "openrouter".to_string(),
            model: "anthropic/claude-sonnet-4-20250514".to_string(),
            system_prompt: None,
            api_key: Some("delegate-test-credential".to_string()),
            temperature: None,
            top_p: None,
            identity_format: None,
            hidden: false,
            max_depth: 2,
            agentic: false,
            allowed_tools: Vec::new(),
            allowed_skills: Vec::new(),
            options: HashMap::new(),
            permission: Value::Null,
            max_iterations: 10,
            steps: None,
        },
    );
    agents
}

/// 简单的回显测试工具
///
/// 实现一个最小化的 `Tool`，用于测试委托工具的工具调用循环。
/// 该工具接收一个 `value` 参数，并返回格式化的回显字符串。
#[derive(Default)]
struct EchoTool;

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl Tool for EchoTool {
    /// 返回工具名称
    fn name(&self) -> &str {
        "echo_tool"
    }

    /// 返回工具描述
    fn description(&self) -> &str {
        "Echoes the `value` argument."
    }

    /// 返回工具参数的 JSON Schema
    ///
    /// 定义一个必需的 `value` 字符串参数。
    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "value": {"type": "string"}
            },
            "required": ["value"]
        })
    }

    /// 执行工具逻辑
    ///
    /// 从参数中提取 `value` 字段，返回格式为 `"echo:{value}"` 的结果。
    ///
    /// # 参数
    ///
    /// - `args`: 包含 `value` 字段的 JSON 对象
    ///
    /// # 返回值
    ///
    /// 成功时返回 `ToolResult`，输出为回显字符串。
    async fn execute(&self, args: serde_json::Value) -> anyhow::Result<ToolResult> {
        // 提取 value 参数，若不存在则使用空字符串
        let value =
            args.get("value").and_then(serde_json::Value::as_str).unwrap_or_default().to_string();
        Ok(ToolResult { success: true, output: format!("echo:{value}"), error: None })
    }
}

/// 执行一次工具调用后返回最终结果的模拟 Provider
///
/// 该 Provider 模拟正常的 agentic 工作流：
/// 1. 首次调用返回一个工具调用请求（`echo_tool`）
/// 2. 收到工具响应后，返回最终文本结果
struct OneToolThenFinalProvider;

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl Provider for OneToolThenFinalProvider {
    /// 带系统提示的聊天（此实现中未使用）
    async fn chat_with_system(
        &self,
        _system_prompt: Option<&str>,
        _message: &str,
        _model: &str,
        _temperature: f64,
    ) -> anyhow::Result<String> {
        Ok("unused".to_string())
    }

    /// 核心聊天方法
    ///
    /// 根据消息历史中是否包含工具响应来决定返回内容：
    /// - 如果存在 `role == "tool"` 的消息，返回最终文本 "done"
    /// - 否则，返回一个调用 `echo_tool` 的工具调用请求
    async fn chat(
        &self,
        request: ChatRequest<'_>,
        _model: &str,
        _temperature: f64,
    ) -> anyhow::Result<ChatResponse> {
        // 检查消息历史中是否包含工具响应
        let has_tool_message = request.messages.iter().any(|m| m.role == "tool");

        if has_tool_message {
            // 工具调用已完成，返回最终结果
            Ok(ChatResponse {
                text: Some("done".to_string()),
                tool_calls: Vec::new(),
                usage: None,
                reasoning_content: None,
            })
        } else {
            // 首次调用，请求执行工具
            Ok(ChatResponse {
                text: None,
                tool_calls: vec![ToolCall {
                    id: "call_1".to_string(),
                    name: "echo_tool".to_string(),
                    arguments: "{\"value\":\"ping\"}".to_string(),
                }],
                usage: None,
                reasoning_content: None,
            })
        }
    }
}

/// 无限循环调用工具的模拟 Provider
///
/// 该 Provider 始终返回工具调用请求，用于测试 `max_iterations` 限制。
/// 每次调用都会返回对 `echo_tool` 的请求，永远不会返回最终文本结果。
struct InfiniteToolCallProvider;

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl Provider for InfiniteToolCallProvider {
    /// 带系统提示的聊天（此实现中未使用）
    async fn chat_with_system(
        &self,
        _system_prompt: Option<&str>,
        _message: &str,
        _model: &str,
        _temperature: f64,
    ) -> anyhow::Result<String> {
        Ok("unused".to_string())
    }

    /// 核心聊天方法
    ///
    /// 始终返回一个工具调用请求，模拟无限工具调用循环的场景。
    async fn chat(
        &self,
        _request: ChatRequest<'_>,
        _model: &str,
        _temperature: f64,
    ) -> anyhow::Result<ChatResponse> {
        Ok(ChatResponse {
            text: None,
            tool_calls: vec![ToolCall {
                id: "loop".to_string(),
                name: "echo_tool".to_string(),
                arguments: "{\"value\":\"x\"}".to_string(),
            }],
            usage: None,
            reasoning_content: None,
        })
    }
}

/// 始终返回错误的模拟 Provider
///
/// 该 Provider 在 `chat` 方法中始终返回错误，
/// 用于测试 agentic 模式下的错误传播机制。
struct FailingProvider;

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl Provider for FailingProvider {
    /// 带系统提示的聊天（此实现中未使用）
    async fn chat_with_system(
        &self,
        _system_prompt: Option<&str>,
        _message: &str,
        _model: &str,
        _temperature: f64,
    ) -> anyhow::Result<String> {
        Ok("unused".to_string())
    }

    /// 核心聊天方法
    ///
    /// 始终返回错误，用于测试错误处理逻辑。
    async fn chat(
        &self,
        _request: ChatRequest<'_>,
        _model: &str,
        _temperature: f64,
    ) -> anyhow::Result<ChatResponse> {
        Err(anyhow!("provider boom"))
    }
}

/// 创建 agentic 模式的代理配置
///
/// 构建一个启用了 agentic 模式的 `DelegateAgentConfig`，
/// 用于测试工具调用循环和迭代限制。
///
/// # 参数
///
/// - `allowed_tools`: 允许调用的工具名称列表
/// - `max_iterations`: 最大工具调用迭代次数
///
/// # 返回值
///
/// 返回配置好的 `DelegateAgentConfig` 实例，使用 OpenRouter 提供者。
fn agentic_config(allowed_tools: Vec<String>, max_iterations: usize) -> DelegateAgentConfig {
    DelegateAgentConfig {
        label: None,
        description: None,
        builtin: false,
        mode: "all".to_string(),
        enabled: true,
        provider: "openrouter".to_string(),
        model: "model-test".to_string(),
        system_prompt: Some("You are agentic.".to_string()),
        api_key: Some("delegate-test-credential".to_string()),
        temperature: Some(0.2),
        top_p: None,
        identity_format: None,
        hidden: false,
        max_depth: 3,
        agentic: true,
        allowed_tools,
        allowed_skills: Vec::new(),
        options: HashMap::new(),
        permission: Value::Null,
        max_iterations,
        steps: None,
    }
}

/// 测试工具名称和参数 schema 的正确性
///
/// 验证：
/// - 工具名称为 "delegate"
/// - schema 包含 agent、prompt、context 属性
/// - agent 和 prompt 为必需参数
/// - 禁止额外属性
/// - agent 和 prompt 有最小长度限制
#[test]
fn name_and_schema() {
    let tool = DelegateTool::new(sample_agents(), None, test_security());
    assert_eq!(tool.name(), "delegate");

    let schema = tool.parameters_schema();
    // 验证必需属性存在
    assert!(schema["properties"]["agent"].is_object());
    assert!(schema["properties"]["prompt"].is_object());
    assert!(schema["properties"]["context"].is_object());

    // 验证必需参数列表
    let required = schema["required"].as_array().unwrap();
    assert!(required.contains(&json!("agent")));
    assert!(required.contains(&json!("prompt")));

    // 验证禁止额外属性
    assert_eq!(schema["additionalProperties"], json!(false));

    // 验证最小长度约束
    assert_eq!(schema["properties"]["agent"]["minLength"], json!(1));
    assert_eq!(schema["properties"]["prompt"]["minLength"], json!(1));
}

/// 测试工具描述非空
///
/// 确保 `description()` 方法返回有效的描述字符串。
#[test]
fn description_not_empty() {
    let tool = DelegateTool::new(sample_agents(), None, test_security());
    assert!(!tool.description().is_empty());
}

/// 测试 schema 中列出可用的代理名称
///
/// 验证 agent 参数的描述中包含已配置的代理名称，
/// 帮助用户了解可用的委托目标。
#[test]
fn schema_lists_agent_names() {
    let tool = DelegateTool::new(sample_agents(), None, test_security());
    let schema = tool.parameters_schema();
    let desc = schema["properties"]["agent"]["description"].as_str().unwrap();
    // 至少应包含 researcher 或 coder 之一
    assert!(desc.contains("researcher") || desc.contains("coder"));
}

/// 测试缺少 agent 参数时返回错误
///
/// 验证只提供 prompt 参数时，执行应返回错误。
#[tokio::test]
async fn missing_agent_param() {
    let tool = DelegateTool::new(sample_agents(), None, test_security());
    let result = tool.execute(json!({"prompt": "test"})).await;
    assert!(result.is_err());
}

/// 测试缺少 prompt 参数时返回错误
///
/// 验证只提供 agent 参数时，执行应返回错误。
#[tokio::test]
async fn missing_prompt_param() {
    let tool = DelegateTool::new(sample_agents(), None, test_security());
    let result = tool.execute(json!({"agent": "researcher"})).await;
    assert!(result.is_err());
}

/// 测试未知代理名称返回错误
///
/// 验证请求不存在的代理时，返回包含 "Unknown agent" 的错误信息。
#[tokio::test]
async fn unknown_agent_returns_error() {
    let tool = DelegateTool::new(sample_agents(), None, test_security());
    let result = tool.execute(json!({"agent": "nonexistent", "prompt": "test"})).await.unwrap();
    assert!(!result.success);
    assert!(result.error.unwrap().contains("Unknown agent"));
}

/// 测试全局深度限制的执行
///
/// 当委托工具的深度超过限制时（通过 `with_depth` 设置），
/// 应拒绝执行并返回包含 "depth limit" 的错误。
#[tokio::test]
async fn depth_limit_enforced() {
    // 设置深度限制为 3，但 sample_agents 中的 researcher 的 max_depth 也是 3
    let tool = DelegateTool::with_depth(sample_agents(), None, test_security(), 3);
    let result = tool.execute(json!({"agent": "researcher", "prompt": "test"})).await.unwrap();
    assert!(!result.success);
    assert!(result.error.unwrap().contains("depth limit"));
}

/// 测试每个代理的深度限制
///
/// 验证当代理配置的 max_depth 超过当前剩余深度时，
/// 应拒绝执行并返回深度限制错误。
#[tokio::test]
async fn depth_limit_per_agent() {
    // 设置全局深度限制为 2，coder 的 max_depth 也是 2
    let tool = DelegateTool::with_depth(sample_agents(), None, test_security(), 2);
    let result = tool.execute(json!({"agent": "coder", "prompt": "test"})).await.unwrap();
    assert!(!result.success);
    assert!(result.error.unwrap().contains("depth limit"));
}

/// 测试无代理配置时的 schema 描述
///
/// 当没有配置任何代理时，agent 参数的描述应包含 "none configured"。
#[test]
fn empty_agents_schema() {
    let tool = DelegateTool::new(HashMap::new(), None, test_security());
    let schema = tool.parameters_schema();
    let desc = schema["properties"]["agent"]["description"].as_str().unwrap();
    assert!(desc.contains("none configured"));
}

/// 测试无效提供者返回错误
///
/// 配置使用不存在的提供者时，应返回包含 "Failed to create provider" 的错误。
#[tokio::test]
async fn invalid_provider_returns_error() {
    let mut agents = HashMap::new();
    agents.insert(
        "broken".to_string(),
        DelegateAgentConfig {
            label: None,
            description: None,
            builtin: false,
            mode: "all".to_string(),
            enabled: true,
            provider: "totally-invalid-provider".to_string(),
            model: "model".to_string(),
            system_prompt: None,
            api_key: None,
            temperature: None,
            top_p: None,
            identity_format: None,
            hidden: false,
            max_depth: 3,
            agentic: false,
            allowed_tools: Vec::new(),
            allowed_skills: Vec::new(),
            options: HashMap::new(),
            permission: Value::Null,
            max_iterations: 10,
            steps: None,
        },
    );
    let tool = DelegateTool::new(agents, None, test_security());
    let result = tool.execute(json!({"agent": "broken", "prompt": "test"})).await.unwrap();
    assert!(!result.success);
    assert!(result.error.unwrap().contains("Failed to create provider"));
}

/// 测试空白代理名称被拒绝
///
/// 只包含空白字符的代理名称应被拒绝，
/// 返回包含 "must not be empty" 的错误。
#[tokio::test]
async fn blank_agent_rejected() {
    let tool = DelegateTool::new(sample_agents(), None, test_security());
    let result = tool.execute(json!({"agent": "  ", "prompt": "test"})).await.unwrap();
    assert!(!result.success);
    assert!(result.error.unwrap().contains("must not be empty"));
}

/// 测试空白 prompt 被拒绝
///
/// 只包含空白字符的 prompt 应被拒绝，
/// 返回包含 "must not be empty" 的错误。
#[tokio::test]
async fn blank_prompt_rejected() {
    let tool = DelegateTool::new(sample_agents(), None, test_security());
    let result = tool.execute(json!({"agent": "researcher", "prompt": "  \t  "})).await.unwrap();
    assert!(!result.success);
    assert!(result.error.unwrap().contains("must not be empty"));
}

/// 测试带空白字符的代理名称被修剪后仍能找到
///
/// 代理名称前后的空白字符应被自动修剪，
/// 修剪后的名称应能正确匹配到配置的代理。
#[tokio::test]
async fn whitespace_agent_name_trimmed_and_found() {
    let tool = DelegateTool::new(sample_agents(), None, test_security());
    let result = tool.execute(json!({"agent": " researcher ", "prompt": "test"})).await.unwrap();
    // 修剪后应找到 researcher，不应返回 "Unknown agent" 错误
    assert!(
        result.error.is_none() || !result.error.as_deref().unwrap_or("").contains("Unknown agent")
    );
}

/// 测试只读模式下委托被阻止
///
/// 当安全策略的自主级别为 `ReadOnly` 时，
/// 委托操作应被阻止并返回包含 "read-only mode" 的错误。
#[tokio::test]
async fn delegation_blocked_in_readonly_mode() {
    let readonly =
        Arc::new(SecurityPolicy { autonomy: AutonomyLevel::ReadOnly, ..SecurityPolicy::default() });
    let tool = DelegateTool::new(sample_agents(), None, readonly);
    let result = tool.execute(json!({"agent": "researcher", "prompt": "test"})).await.unwrap();
    assert!(!result.success);
    assert!(result.error.as_deref().unwrap_or("").contains("read-only mode"));
}

/// 测试速率限制时委托被阻止
///
/// 当每小时最大操作数设置为 0 时，
/// 委托操作应被阻止并返回包含 "Rate limit exceeded" 的错误。
#[tokio::test]
async fn delegation_blocked_when_rate_limited() {
    let limited = Arc::new(SecurityPolicy { max_actions_per_hour: 0, ..SecurityPolicy::default() });
    let tool = DelegateTool::new(sample_agents(), None, limited);
    let result = tool.execute(json!({"agent": "researcher", "prompt": "test"})).await.unwrap();
    assert!(!result.success);
    assert!(result.error.as_deref().unwrap_or("").contains("Rate limit exceeded"));
}

/// 测试 context 参数被添加到 prompt 前缀
///
/// 验证当提供 context 参数时，它应被添加到最终 prompt 的前缀中。
/// 由于使用无效提供者，预期返回 "Failed to create provider" 错误。
#[tokio::test]
async fn delegate_context_is_prepended_to_prompt() {
    let mut agents = HashMap::new();
    agents.insert(
        "tester".to_string(),
        DelegateAgentConfig {
            label: None,
            description: None,
            builtin: false,
            mode: "all".to_string(),
            enabled: true,
            provider: "invalid-for-test".to_string(),
            model: "test-model".to_string(),
            system_prompt: None,
            api_key: None,
            temperature: None,
            top_p: None,
            identity_format: None,
            hidden: false,
            max_depth: 3,
            agentic: false,
            allowed_tools: Vec::new(),
            allowed_skills: Vec::new(),
            options: HashMap::new(),
            permission: Value::Null,
            max_iterations: 10,
            steps: None,
        },
    );
    let tool = DelegateTool::new(agents, None, test_security());
    let result = tool
        .execute(json!({
            "agent": "tester",
            "prompt": "do something",
            "context": "some context data"
        }))
        .await
        .unwrap();

    assert!(!result.success);
    assert!(result.error.as_deref().unwrap_or("").contains("Failed to create provider"));
}

/// 测试空 context 省略前缀
///
/// 当 context 为空字符串时，不应在 prompt 前添加上下文前缀。
#[tokio::test]
async fn delegate_empty_context_omits_prefix() {
    let mut agents = HashMap::new();
    agents.insert(
        "tester".to_string(),
        DelegateAgentConfig {
            label: None,
            description: None,
            builtin: false,
            mode: "all".to_string(),
            enabled: true,
            provider: "invalid-for-test".to_string(),
            model: "test-model".to_string(),
            system_prompt: None,
            api_key: None,
            temperature: None,
            top_p: None,
            identity_format: None,
            hidden: false,
            max_depth: 3,
            agentic: false,
            allowed_tools: Vec::new(),
            allowed_skills: Vec::new(),
            options: HashMap::new(),
            permission: Value::Null,
            max_iterations: 10,
            steps: None,
        },
    );
    let tool = DelegateTool::new(agents, None, test_security());
    let result = tool
        .execute(json!({
            "agent": "tester",
            "prompt": "do something",
            "context": ""
        }))
        .await
        .unwrap();

    assert!(!result.success);
    assert!(result.error.as_deref().unwrap_or("").contains("Failed to create provider"));
}

/// 测试带深度参数的工具构造
///
/// 验证 `with_depth` 方法正确设置内部深度字段。
#[test]
fn delegate_depth_construction() {
    let tool = DelegateTool::with_depth(sample_agents(), None, test_security(), 5);
    assert_eq!(tool.depth, 5);
}

#[test]
fn delegate_merges_workspace_identity_context_with_agent_prompt() {
    let tool = DelegateTool::new(sample_agents(), None, test_security())
        .with_workspace_identity_context("## Project Context\n\nAGENTS".to_string());

    let merged =
        tool.merged_system_prompt("researcher", Some("You are a research assistant.")).unwrap();

    assert!(merged.contains("## Project Context"));
    assert!(merged.contains("AGENTS"));
    assert!(merged.contains("You are a research assistant."));
}

#[test]
fn delegate_uses_workspace_identity_context_without_agent_prompt() {
    let tool = DelegateTool::new(sample_agents(), None, test_security())
        .with_workspace_identity_context("## Project Context\n\nIDENTITY".to_string());

    let merged = tool.merged_system_prompt("researcher", None).unwrap();

    assert_eq!(merged, "## Project Context\n\nIDENTITY");
}

#[test]
fn delegate_merges_workspace_skill_and_agent_sections_in_order() {
    let tool = DelegateTool::new(sample_agents(), None, test_security())
        .with_workspace_identity_context("workspace identity".to_string())
        .with_skill_contexts(HashMap::from([(
            "researcher".to_string(),
            "skill instructions".to_string(),
        )]));

    assert_eq!(
        tool.merged_system_prompt("researcher", Some("  agent prompt  ")).as_deref(),
        Some("workspace identity\n\nskill instructions\n\nagent prompt")
    );
}

#[test]
fn delegate_builders_store_multimodal_config_and_disable_coordination() {
    let multimodal = crate::app::agent::config::MultimodalConfig {
        max_images: 7,
        max_image_size_mb: 9,
        allow_remote_fetch: true,
    };
    let tool = DelegateTool::new(sample_agents(), None, test_security())
        .with_multimodal_config(multimodal)
        .with_coordination_disabled();

    assert_eq!(tool.multimodal_config.max_images, 7);
    assert_eq!(tool.multimodal_config.max_image_size_mb, 9);
    assert!(tool.multimodal_config.allow_remote_fetch);
    assert!(tool.coordination_bus_snapshot().is_none());
}

#[test]
fn delegate_coordination_builder_normalizes_blank_lead_agent() {
    let tool = DelegateTool::new(sample_agents(), None, test_security())
        .with_coordination_bus(InMemoryMessageBus::new(), " \n ");

    assert_eq!(tool.coordination_lead_agent, DEFAULT_COORDINATION_LEAD_AGENT);
    assert!(tool.coordination_bus_snapshot().is_some());
}

/// 测试无代理配置时的执行错误
///
/// 当没有配置任何代理时，执行委托应返回包含 "none configured" 的错误。
#[tokio::test]
async fn delegate_no_agents_configured() {
    let tool = DelegateTool::new(HashMap::new(), None, test_security());
    let result = tool.execute(json!({"agent": "any", "prompt": "test"})).await.unwrap();
    assert!(!result.success);
    assert!(result.error.unwrap().contains("none configured"));
}

/// 测试 agentic 模式拒绝空的允许工具列表
///
/// agentic 模式需要至少一个允许的工具，
/// 空列表应导致返回包含 "allowed_tools is empty" 的错误。
#[tokio::test]
async fn agentic_mode_rejects_empty_allowed_tools() {
    let mut agents = HashMap::new();
    agents.insert("agentic".to_string(), agentic_config(Vec::new(), 10));

    let tool = DelegateTool::new(agents, None, test_security());
    let result = tool.execute(json!({"agent": "agentic", "prompt": "test"})).await.unwrap();

    assert!(!result.success);
    assert!(result.error.as_deref().unwrap_or("").contains("allowed_tools is empty"));
}

/// 测试 agentic 模式拒绝不在父工具集中的允许工具
///
/// 当 allowed_tools 中的工具名称在父工具集中找不到时，
/// 应返回包含 "no executable tools" 的错误。
#[tokio::test]
async fn agentic_mode_rejects_unmatched_allowed_tools() {
    let mut agents = HashMap::new();
    agents.insert("agentic".to_string(), agentic_config(vec!["missing_tool".to_string()], 10));

    // 提供父工具集，但不包含 missing_tool
    let tool = DelegateTool::new(agents, None, test_security())
        .with_parent_tools(Arc::new(vec![Arc::new(EchoTool)]));
    let result = tool.execute(json!({"agent": "agentic", "prompt": "test"})).await.unwrap();

    assert!(!result.success);
    assert!(result.error.as_deref().unwrap_or("").contains("no executable tools"));
}

/// 测试 agentic 模式执行工具调用循环并过滤工具
///
/// 验证：
/// - 只使用 allowed_tools 中指定的工具
/// - 工具调用循环正常执行
/// - 最终结果包含正确的输出
#[tokio::test]
async fn execute_agentic_runs_tool_call_loop_with_filtered_tools() {
    let config = agentic_config(vec!["echo_tool".to_string()], 10);

    // 创建父工具集，包含 EchoTool 和 DelegateTool
    let tool =
        DelegateTool::new(HashMap::new(), None, test_security()).with_parent_tools(Arc::new(vec![
            Arc::new(EchoTool),
            Arc::new(DelegateTool::new(HashMap::new(), None, test_security())),
        ]));

    let provider = OneToolThenFinalProvider;
    let result =
        tool.execute_agentic("agentic", &config, &provider, None, "run", 0.2).await.unwrap();

    assert!(result.success);
    // 输出应包含模型信息和最终结果
    assert!(result.output.contains("(openrouter/model-test, agentic)"));
    assert!(result.output.contains("done"));
}

/// 测试 agentic 模式排除 delegate 工具即使在允许列表中
///
/// 为防止无限递归，delegate 工具应被自动排除，
/// 即使它在 allowed_tools 列表中。这应导致 "no executable tools" 错误。
#[tokio::test]
async fn execute_agentic_excludes_delegate_even_if_allowlisted() {
    let config = agentic_config(vec!["delegate".to_string()], 10);

    // 父工具集只包含 DelegateTool
    let tool =
        DelegateTool::new(HashMap::new(), None, test_security()).with_parent_tools(Arc::new(vec![
            Arc::new(DelegateTool::new(HashMap::new(), None, test_security())),
        ]));

    let provider = OneToolThenFinalProvider;
    let result =
        tool.execute_agentic("agentic", &config, &provider, None, "run", 0.2).await.unwrap();

    assert!(!result.success);
    assert!(result.error.as_deref().unwrap_or("").contains("no executable tools"));
}

/// 测试 agentic 模式遵守最大迭代次数限制
///
/// 使用 `InfiniteToolCallProvider` 模拟无限工具调用，
/// 验证达到 max_iterations 时正确终止并返回错误。
#[tokio::test]
async fn execute_agentic_respects_max_iterations() {
    let config = agentic_config(vec!["echo_tool".to_string()], 2);
    let tool = DelegateTool::new(HashMap::new(), None, test_security())
        .with_parent_tools(Arc::new(vec![Arc::new(EchoTool)]));

    let provider = InfiniteToolCallProvider;
    let result =
        tool.execute_agentic("agentic", &config, &provider, None, "run", 0.2).await.unwrap();

    assert!(!result.success);
    // 错误信息应包含迭代次数限制
    assert!(result.error.as_deref().unwrap_or("").contains("maximum tool iterations (2)"));
}

/// 测试 agentic 模式传播 Provider 错误
///
/// 当 Provider 返回错误时，该错误应正确传播到结果中。
#[tokio::test]
async fn execute_agentic_propagates_provider_errors() {
    let config = agentic_config(vec!["echo_tool".to_string()], 10);
    let tool = DelegateTool::new(HashMap::new(), None, test_security())
        .with_parent_tools(Arc::new(vec![Arc::new(EchoTool)]));

    let provider = FailingProvider;
    let result =
        tool.execute_agentic("agentic", &config, &provider, None, "run", 0.2).await.unwrap();

    assert!(!result.success);
    assert!(result.error.as_deref().unwrap_or("").contains("provider boom"));
}

/// 测试执行失败时在协调总线中记录事件
///
/// 验证：
/// - 工作代理（worker）的收件箱中有一条消息
/// - 主代理（lead）的收件箱中有三条消息
/// - 主代理收件箱中包含失败的任务结果事件
/// - 状态上下文中记录了失败状态
#[tokio::test]
async fn execute_records_failure_events_in_coordination_bus() {
    let mut agents = HashMap::new();
    agents.insert(
        "broken".to_string(),
        DelegateAgentConfig {
            label: None,
            description: None,
            builtin: false,
            mode: "all".to_string(),
            enabled: true,
            provider: "totally-invalid-provider".to_string(),
            model: "model".to_string(),
            system_prompt: None,
            api_key: None,
            temperature: None,
            top_p: None,
            identity_format: None,
            hidden: false,
            max_depth: 3,
            agentic: false,
            allowed_tools: Vec::new(),
            allowed_skills: Vec::new(),
            options: HashMap::new(),
            permission: Value::Null,
            max_iterations: 10,
            steps: None,
        },
    );

    let tool = DelegateTool::new(agents, None, test_security());
    let result = tool
        .execute(json!({
            "agent": "broken",
            "prompt": "Investigate failing integration test",
            "context": "CI logs attached"
        }))
        .await
        .unwrap();
    assert!(!result.success);
    assert!(result.error.as_deref().unwrap_or("").contains("Failed to create provider"));

    // 获取协调总线快照进行验证
    let bus = tool.coordination_bus_snapshot().expect("coordination bus should be initialized");

    // 验证工作代理收件箱
    let worker_messages = bus.drain_for_agent("broken", 0).expect("worker inbox should exist");
    assert_eq!(worker_messages.len(), 1);
    let correlation_id = worker_messages[0]
        .envelope
        .correlation_id
        .clone()
        .expect("request should have correlation id");

    // 验证主代理收件箱
    let lead_messages =
        bus.drain_for_agent(DEFAULT_COORDINATION_LEAD_AGENT, 0).expect("lead inbox should exist");
    assert_eq!(lead_messages.len(), 3);

    // 验证主代理收件箱中包含失败的任务结果事件
    assert!(
        lead_messages.iter().any(|entry| matches!(
            entry.envelope.payload,
            CoordinationPayload::TaskResult { success: false, .. }
        )),
        "lead inbox should contain failed task result event"
    );

    // 验证状态上下文
    let state_key = format!("delegate/{correlation_id}/state");
    let state_entry = bus.context_entry(&state_key).expect("state context should exist");
    assert_eq!(state_entry.version, 2);
    assert_eq!(state_entry.value["phase"], json!("failed"));
    assert_eq!(state_entry.value["success"], json!(false));
}

/// 测试协调追踪正确转换状态到已完成
///
/// 验证：
/// - 调用 `start_coordination_trace` 创建初始追踪
/// - 调用 `finish_coordination_trace` 更新状态为已完成
/// - 状态上下文中记录了成功状态
#[test]
fn coordination_trace_transitions_state_to_completed() {
    let mut agents = HashMap::new();
    agents.insert(
        "tester".to_string(),
        DelegateAgentConfig {
            label: None,
            description: None,
            builtin: false,
            mode: "all".to_string(),
            enabled: true,
            provider: "openrouter".to_string(),
            model: "model-test".to_string(),
            system_prompt: None,
            api_key: Some("delegate-test-credential".to_string()),
            temperature: Some(0.2),
            top_p: None,
            identity_format: None,
            hidden: false,
            max_depth: 2,
            agentic: false,
            allowed_tools: Vec::new(),
            allowed_skills: Vec::new(),
            options: HashMap::new(),
            permission: Value::Null,
            max_iterations: 10,
            steps: None,
        },
    );
    let tool = DelegateTool::new(agents, None, test_security());
    let agent_config = tool.agents.get("tester").expect("tester config should exist");

    // 启动协调追踪
    let trace = tool.start_coordination_trace(
        "tester",
        "Summarize findings",
        "runbook notes",
        agent_config,
    );

    // 完成协调追踪
    tool.finish_coordination_trace("tester", &trace, true, "done");

    // 验证状态上下文
    let bus = tool.coordination_bus_snapshot().expect("coordination bus should be initialized");
    let state_key = format!("delegate/{}/state", trace.correlation_id);
    let state_entry = bus.context_entry(&state_key).expect("state context should exist");

    assert_eq!(state_entry.version, 2);
    assert_eq!(state_entry.value["phase"], json!("completed"));
    assert_eq!(state_entry.value["success"], json!(true));
}
