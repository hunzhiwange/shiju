//! Provider trait 及相关类型的单元测试模块
//!
//! 本模块包含针对 `Provider` trait 及其关联类型的全面测试用例，主要覆盖以下内容：
//!
//! - **消息类型测试**：验证 `ChatMessage` 构造器、`ChatResponse` 辅助方法、`TokenUsage` 默认值
//! - **序列化测试**：确保 `ToolCall` 和 `ConversationMessage` 可正确序列化为 JSON
//! - **能力声明测试**：验证 `ProviderCapabilities` 的默认值、相等性比较及与 Provider 的集成
//! - **工具载荷测试**：覆盖 `ToolsPayload` 的所有变体（Gemini、Anthropic、OpenAI、PromptGuided）
//! - **工具指令生成测试**：验证 `build_tool_instructions_text` 函数的输出格式
//! - **Provider trait 默认实现测试**：测试 `convert_tools` 和 `chat` 方法的默认行为
//!
//! # 测试策略
//!
//! 本模块使用多个 Mock Provider 实现来隔离测试 Provider trait 的默认方法行为，
//! 确保在不依赖具体 Provider 实现的情况下验证核心逻辑的正确性。

use super::*;

#[allow(dead_code)]
mod tests {
    use super::*;

    /// 具备完整能力的 Mock Provider
    ///
    /// 此 Provider 用于测试能力查询方法（`supports_native_tools` 和 `supports_vision`），
    /// 它声明同时支持原生工具调用和视觉能力。
    struct CapabilityMockProvider;

    #[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
    #[cfg_attr(not(target_arch = "wasm32"), async_trait)]
    impl Provider for CapabilityMockProvider {
        /// 返回完整启用的能力配置
        ///
        /// # 返回值
        ///
        /// 始终返回 `native_tool_calling: true` 和 `vision: true` 的能力声明
        fn capabilities(&self) -> ProviderCapabilities {
            ProviderCapabilities { native_tool_calling: true, vision: true }
        }

        /// 模拟聊天请求处理
        ///
        /// # 参数
        ///
        /// - `_system_prompt`: 系统提示词（测试中被忽略）
        /// - `_message`: 用户消息（测试中被忽略）
        /// - `_model`: 模型标识符（测试中被忽略）
        /// - `_temperature`: 生成温度（测试中被忽略）
        ///
        /// # 返回值
        ///
        /// 始终返回 `"ok"` 字符串，表示请求处理成功
        async fn chat_with_system(
            &self,
            _system_prompt: Option<&str>,
            _message: &str,
            _model: &str,
            _temperature: f64,
        ) -> anyhow::Result<String> {
            Ok("ok".into())
        }
    }

    /// 测试 ChatMessage 各角色构造器的正确性
    ///
    /// 验证 `system`、`user`、`assistant`、`tool` 四种构造器是否正确设置
    /// 对应的角色标识和消息内容。
    #[test]
    fn chat_message_constructors() {
        let sys = ChatMessage::system("Be helpful");
        assert_eq!(sys.role, "system");
        assert_eq!(sys.content, "Be helpful");

        let user = ChatMessage::user("Hello");
        assert_eq!(user.role, "user");

        let asst = ChatMessage::assistant("Hi there");
        assert_eq!(asst.role, "assistant");

        let tool = ChatMessage::tool("{}");
        assert_eq!(tool.role, "tool");
    }

    /// 测试 ChatResponse 辅助方法的行为
    ///
    /// 验证 `has_tool_calls()` 和 `text_or_empty()` 方法在以下场景中的表现：
    /// - 空响应（无文本、无工具调用）
    /// - 包含工具调用的响应
    #[test]
    fn chat_response_helpers() {
        let empty =
            ChatResponse { text: None, tool_calls: vec![], usage: None, reasoning_content: None };
        assert!(!empty.has_tool_calls());
        assert_eq!(empty.text_or_empty(), "");

        let with_tools = ChatResponse {
            text: Some("Let me check".into()),
            tool_calls: vec![ToolCall {
                id: "1".into(),
                name: "shell".into(),
                arguments: "{}".into(),
            }],
            usage: None,
            reasoning_content: None,
        };
        assert!(with_tools.has_tool_calls());
        assert_eq!(with_tools.text_or_empty(), "Let me check");
    }

    /// 测试 TokenUsage 默认值是否为 None
    ///
    /// 验证 `TokenUsage::default()` 创建的实例中，
    /// `input_tokens` 和 `output_tokens` 字段均为 `None`。
    #[test]
    fn token_usage_default_is_none() {
        let usage = TokenUsage::default();
        assert!(usage.input_tokens.is_none());
        assert!(usage.output_tokens.is_none());
        assert!(usage.cached_tokens.is_none());
        assert!(usage.reasoning_tokens.is_none());
    }

    /// 测试 ChatResponse 中 TokenUsage 的存储和访问
    ///
    /// 验证包含使用统计的 ChatResponse 可以正确存储和检索 token 计数。
    #[test]
    fn chat_response_with_usage() {
        let resp = ChatResponse {
            text: Some("Hello".into()),
            tool_calls: vec![],
            usage: Some(TokenUsage {
                input_tokens: Some(100),
                output_tokens: Some(50),
                cached_tokens: Some(10),
                reasoning_tokens: Some(5),
            }),
            reasoning_content: None,
        };
        assert_eq!(resp.usage.as_ref().unwrap().input_tokens, Some(100));
        assert_eq!(resp.usage.as_ref().unwrap().output_tokens, Some(50));
        assert_eq!(resp.usage.as_ref().unwrap().cached_tokens, Some(10));
        assert_eq!(resp.usage.as_ref().unwrap().reasoning_tokens, Some(5));
    }

    /// 测试 ToolCall 的 JSON 序列化
    ///
    /// 验证 ToolCall 结构体可以正确序列化为 JSON 格式，
    /// 并且序列化结果包含所有必要字段。
    #[test]
    fn tool_call_serialization() {
        let tc = ToolCall {
            id: "call_123".into(),
            name: "file_read".into(),
            arguments: r#"{"path":"test.txt"}"#.into(),
        };
        let json = serde_json::to_string(&tc).unwrap();
        assert!(json.contains("call_123"));
        assert!(json.contains("file_read"));
    }

    /// 测试 ConversationMessage 变体的序列化
    ///
    /// 验证 `Chat` 和 `ToolResults` 两种变体在序列化时
    /// 是否生成正确的类型标记（`"type":"Chat"` 或 `"type":"ToolResults"`）。
    #[test]
    fn conversation_message_variants() {
        let chat = ConversationMessage::Chat(ChatMessage::user("hi"));
        let json = serde_json::to_string(&chat).unwrap();
        assert!(json.contains("\"type\":\"Chat\""));

        let tool_result = ConversationMessage::ToolResults(vec![ToolResultMessage {
            tool_call_id: "1".into(),
            content: "done".into(),
        }]);
        let json = serde_json::to_string(&tool_result).unwrap();
        assert!(json.contains("\"type\":\"ToolResults\""));
    }

    /// 测试 ProviderCapabilities 的默认值
    ///
    /// 验证默认能力配置中 `native_tool_calling` 和 `vision` 均为 `false`。
    #[test]
    fn provider_capabilities_default() {
        let caps = ProviderCapabilities::default();
        assert!(!caps.native_tool_calling);
        assert!(!caps.vision);
    }

    /// 测试 ProviderCapabilities 的相等性比较
    ///
    /// 验证具有相同字段值的能力声明相等，
    /// 不同字段值的能力声明不相等。
    #[test]
    fn provider_capabilities_equality() {
        let caps1 = ProviderCapabilities { native_tool_calling: true, vision: false };
        let caps2 = ProviderCapabilities { native_tool_calling: true, vision: false };
        let caps3 = ProviderCapabilities { native_tool_calling: false, vision: false };

        assert_eq!(caps1, caps2);
        assert_ne!(caps1, caps3);
    }

    /// 测试 supports_native_tools 方法正确反映能力配置
    ///
    /// 验证 Provider trait 的默认 `supports_native_tools` 实现
    /// 能正确返回 `capabilities()` 中声明的原生工具调用能力。
    #[test]
    fn supports_native_tools_reflects_capabilities_default_mapping() {
        let provider = CapabilityMockProvider;
        assert!(provider.supports_native_tools());
    }

    /// 测试 supports_vision 方法正确反映能力配置
    ///
    /// 验证 Provider trait 的默认 `supports_vision` 实现
    /// 能正确返回 `capabilities()` 中声明的视觉能力。
    #[test]
    fn supports_vision_reflects_capabilities_default_mapping() {
        let provider = CapabilityMockProvider;
        assert!(provider.supports_vision());
    }

    /// 测试 ToolsPayload 的所有变体
    ///
    /// 验证四种工具载荷变体可以正确构造：
    /// - `Gemini`：包含 `function_declarations` 字段
    /// - `Anthropic`：包含 `tools` 字段
    /// - `OpenAI`：包含 `tools` 字段（格式不同）
    /// - `PromptGuided`：包含文本指令
    #[test]
    fn tools_payload_variants() {
        // 测试 Gemini 变体
        let gemini = ToolsPayload::Gemini {
            function_declarations: vec![serde_json::json!({"name": "test"})],
        };
        assert!(matches!(gemini, ToolsPayload::Gemini { .. }));

        // 测试 Anthropic 变体
        let anthropic =
            ToolsPayload::Anthropic { tools: vec![serde_json::json!({"name": "test"})] };
        assert!(matches!(anthropic, ToolsPayload::Anthropic { .. }));

        // 测试 OpenAI 变体
        let openai = ToolsPayload::OpenAI { tools: vec![serde_json::json!({"type": "function"})] };
        assert!(matches!(openai, ToolsPayload::OpenAI { .. }));

        // 测试 PromptGuided 变体
        let prompt_guided = ToolsPayload::PromptGuided { instructions: "Use tools...".to_string() };
        assert!(matches!(prompt_guided, ToolsPayload::PromptGuided { .. }));
    }

    /// 测试 build_tool_instructions_text 函数的输出格式
    ///
    /// 验证工具指令文本生成函数是否：
    /// - 包含协议描述（"Tool Use Protocol"）
    /// - 包含工具调用标记（`<tool_call` 和 `</tool_call`）
    /// - 列出所有工具的名称和描述
    /// - 包含参数 schema 信息
    #[test]
    fn build_tool_instructions_text_format() {
        let tools = vec![
            ToolSpec::new(
                "shell",
                "Execute commands",
                serde_json::json!({
                    "type": "object",
                    "properties": {
                        "command": {"type": "string"}
                    }
                }),
            ),
            ToolSpec::new(
                "file_read",
                "Read files",
                serde_json::json!({
                    "type": "object",
                    "properties": {
                        "path": {"type": "string"}
                    }
                }),
            ),
        ];

        let instructions = build_tool_instructions_text(&tools);

        // 检查协议描述是否存在
        assert!(instructions.contains("Tool Use Protocol"));
        assert!(instructions.contains("<tool_call>"));
        assert!(instructions.contains("</tool_call"));

        // 检查工具列表是否正确
        assert!(instructions.contains("**shell**"));
        assert!(instructions.contains("Execute commands"));
        assert!(instructions.contains("**file_read**"));
        assert!(instructions.contains("Read files"));

        // 检查参数信息是否包含
        assert!(instructions.contains("Parameters:"));
        assert!(instructions.contains(r#""type":"object""#));
    }

    /// 测试 build_tool_instructions_text 函数对空工具列表的处理
    ///
    /// 验证当传入空工具列表时，函数仍能生成包含协议描述
    /// 和空工具区段的完整输出。
    #[test]
    fn build_tool_instructions_text_empty() {
        let instructions = build_tool_instructions_text(&[]);

        // 应仍然包含协议描述
        assert!(instructions.contains("Tool Use Protocol"));

        // 应包含空的工具区段标题
        assert!(instructions.contains("Available Tools"));
    }

    /// 可配置原生工具支持状态的 Mock Provider
    ///
    /// 此 Provider 用于测试 `convert_tools` 和 `chat` 的默认实现，
    /// 可通过 `supports_native` 字段控制是否声明支持原生工具调用。
    struct MockProvider {
        /// 是否支持原生工具调用
        supports_native: bool,
    }

    #[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
    #[cfg_attr(not(target_arch = "wasm32"), async_trait)]
    impl Provider for MockProvider {
        /// 返回配置的原生工具支持状态
        fn supports_native_tools(&self) -> bool {
            self.supports_native
        }

        /// 模拟聊天请求处理
        ///
        /// # 参数
        ///
        /// - `_system`: 系统提示词（测试中被忽略）
        /// - `_message`: 用户消息（测试中被忽略）
        /// - `_model`: 模型标识符（测试中被忽略）
        /// - `_temperature`: 生成温度（测试中被忽略）
        ///
        /// # 返回值
        ///
        /// 始终返回 `"response"` 字符串
        async fn chat_with_system(
            &self,
            _system: Option<&str>,
            _message: &str,
            _model: &str,
            _temperature: f64,
        ) -> anyhow::Result<String> {
            Ok("response".to_string())
        }
    }

    /// 测试 Provider 的 convert_tools 默认实现
    ///
    /// 验证当 Provider 不支持原生工具调用时，
    /// `convert_tools` 方法默认返回 `PromptGuided` 变体，
    /// 并且生成的指令文本包含工具名称和描述。
    #[test]
    fn provider_convert_tools_default() {
        let provider = MockProvider { supports_native: false };

        let tools =
            vec![ToolSpec::new("test_tool", "A test tool", serde_json::json!({"type": "object"}))];

        let payload = provider.convert_tools(&tools);

        // 默认实现应返回 PromptGuided 变体
        assert!(matches!(payload, ToolsPayload::PromptGuided { .. }));

        if let ToolsPayload::PromptGuided { instructions } = payload {
            assert!(instructions.contains("test_tool"));
            assert!(instructions.contains("A test tool"));
        }
    }

    /// 测试 Provider 的 chat 方法在 PromptGuided 模式下的回退行为
    ///
    /// 验证当使用不支持原生工具的 Provider 并传入工具列表时，
    /// `chat` 方法能正确回退到 PromptGuided 模式并返回响应。
    #[tokio::test]
    async fn provider_chat_prompt_guided_fallback() {
        let provider = MockProvider { supports_native: false };

        let tools =
            vec![ToolSpec::new("shell", "Run commands", serde_json::json!({"type": "object"}))];

        let request = ChatRequest { messages: &[ChatMessage::user("Hello")], tools: Some(&tools) };

        let response = provider.chat(request, "model", 0.7).await.unwrap();

        // 应返回包含文本的响应（默认实现调用 chat_with_history）
        assert!(response.text.is_some());
    }

    /// 测试 Provider 的 chat 方法在不使用工具时的正常行为
    ///
    /// 验证当请求中不包含工具时，即使 Provider 声明支持原生工具，
    /// `chat` 方法也能正常处理请求。
    #[tokio::test]
    async fn provider_chat_without_tools() {
        let provider = MockProvider { supports_native: true };

        let request = ChatRequest { messages: &[ChatMessage::user("Hello")], tools: None };

        let response = provider.chat(request, "model", 0.7).await.unwrap();

        // 无工具时应正常工作
        assert!(response.text.is_some());
    }

    /// 回显系统提示词的 Mock Provider
    ///
    /// 此 Provider 将接收到的系统提示词作为响应返回，
    /// 用于断言 `chat` 方法中系统提示词的合并逻辑是否正确。
    struct EchoSystemProvider {
        /// 是否支持原生工具调用
        supports_native: bool,
    }

    #[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
    #[cfg_attr(not(target_arch = "wasm32"), async_trait)]
    impl Provider for EchoSystemProvider {
        /// 返回配置的原生工具支持状态
        fn supports_native_tools(&self) -> bool {
            self.supports_native
        }

        /// 返回系统提示词作为响应
        ///
        /// # 参数
        ///
        /// - `system`: 系统提示词（将被原样返回）
        /// - `_message`: 用户消息（测试中被忽略）
        /// - `_model`: 模型标识符（测试中被忽略）
        /// - `_temperature`: 生成温度（测试中被忽略）
        ///
        /// # 返回值
        ///
        /// 返回系统提示词内容，若不存在则返回空字符串
        async fn chat_with_system(
            &self,
            system: Option<&str>,
            _message: &str,
            _model: &str,
            _temperature: f64,
        ) -> anyhow::Result<String> {
            Ok(system.unwrap_or_default().to_string())
        }
    }

    /// 使用自定义工具转换逻辑的 Mock Provider
    ///
    /// 此 Provider 覆盖了 `convert_tools` 方法，
    /// 返回固定的自定义工具指令，用于测试 `convert_tools` 覆盖行为。
    struct CustomConvertProvider;

    #[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
    #[cfg_attr(not(target_arch = "wasm32"), async_trait)]
    impl Provider for CustomConvertProvider {
        /// 声明不支持原生工具调用
        fn supports_native_tools(&self) -> bool {
            false
        }

        /// 返回自定义的 PromptGuided 工具指令
        ///
        /// # 参数
        ///
        /// - `_tools`: 工具列表（被忽略）
        ///
        /// # 返回值
        ///
        /// 始终返回包含 `"CUSTOM_TOOL_INSTRUCTIONS"` 的 PromptGuided 变体
        fn convert_tools(&self, _tools: &[ToolSpec]) -> ToolsPayload {
            ToolsPayload::PromptGuided { instructions: "CUSTOM_TOOL_INSTRUCTIONS".to_string() }
        }

        /// 返回系统提示词作为响应
        async fn chat_with_system(
            &self,
            system: Option<&str>,
            _message: &str,
            _model: &str,
            _temperature: f64,
        ) -> anyhow::Result<String> {
            Ok(system.unwrap_or_default().to_string())
        }
    }

    /// 返回无效载荷类型的 Mock Provider
    ///
    /// 此 Provider 在非原生模式下返回 OpenAI 格式的工具载荷，
    /// 用于测试对无效载荷类型的错误处理。
    struct InvalidConvertProvider;

    #[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
    #[cfg_attr(not(target_arch = "wasm32"), async_trait)]
    impl Provider for InvalidConvertProvider {
        /// 声明不支持原生工具调用
        fn supports_native_tools(&self) -> bool {
            false
        }

        /// 返回与声明能力不匹配的 OpenAI 载荷类型
        ///
        /// # 参数
        ///
        /// - `_tools`: 工具列表（被忽略）
        ///
        /// # 返回值
        ///
        /// 返回 OpenAI 格式的载荷，与非原生模式不兼容
        fn convert_tools(&self, _tools: &[ToolSpec]) -> ToolsPayload {
            ToolsPayload::OpenAI { tools: vec![serde_json::json!({"type": "function"})] }
        }

        /// 此方法不应被调用
        async fn chat_with_system(
            &self,
            _system: Option<&str>,
            _message: &str,
            _model: &str,
            _temperature: f64,
        ) -> anyhow::Result<String> {
            Ok("should_not_reach".to_string())
        }
    }

    /// 测试 PromptGuided 模式下保留非首位的现有系统提示词
    ///
    /// 验证当系统提示词不在消息列表首位时，
    /// `chat` 方法仍能正确合并现有系统提示词和工具指令。
    #[tokio::test]
    async fn provider_chat_prompt_guided_preserves_existing_system_not_first() {
        let provider = EchoSystemProvider { supports_native: false };

        let tools =
            vec![ToolSpec::new("shell", "Run commands", serde_json::json!({"type": "object"}))];

        // 系统提示词位于用户消息之后（非标准顺序）
        let request = ChatRequest {
            messages: &[ChatMessage::user("Hello"), ChatMessage::system("BASE_SYSTEM_PROMPT")],
            tools: Some(&tools),
        };

        let response = provider.chat(request, "model", 0.7).await.unwrap();
        let text = response.text.unwrap_or_default();

        // 验证现有系统提示词被保留
        assert!(text.contains("BASE_SYSTEM_PROMPT"));
        // 验证工具协议指令被追加
        assert!(text.contains("Tool Use Protocol"));
    }

    /// 测试 PromptGuided 模式使用自定义 convert_tools 覆盖
    ///
    /// 验证当 Provider 覆盖 `convert_tools` 方法时，
    /// `chat` 方法会使用自定义的工具指令而非默认生成。
    #[tokio::test]
    async fn provider_chat_prompt_guided_uses_convert_tools_override() {
        let provider = CustomConvertProvider;

        let tools =
            vec![ToolSpec::new("shell", "Run commands", serde_json::json!({"type": "object"}))];

        let request = ChatRequest {
            messages: &[ChatMessage::system("BASE"), ChatMessage::user("Hello")],
            tools: Some(&tools),
        };

        let response = provider.chat(request, "model", 0.7).await.unwrap();
        let text = response.text.unwrap_or_default();

        // 验证基础系统提示词被保留
        assert!(text.contains("BASE"));
        // 验证自定义工具指令被使用
        assert!(text.contains("CUSTOM_TOOL_INSTRUCTIONS"));
    }

    /// 测试 PromptGuided 模式拒绝非 PromptGuided 载荷类型
    ///
    /// 验证当 Provider 在非原生模式下返回非 PromptGuided 载荷时，
    /// `chat` 方法会返回包含 "non-prompt-guided" 的错误信息。
    #[tokio::test]
    async fn provider_chat_prompt_guided_rejects_non_prompt_payload() {
        let provider = InvalidConvertProvider;

        let tools =
            vec![ToolSpec::new("shell", "Run commands", serde_json::json!({"type": "object"}))];

        let request = ChatRequest { messages: &[ChatMessage::user("Hello")], tools: Some(&tools) };

        let err = provider.chat(request, "model", 0.7).await.unwrap_err();
        let message = err.to_string();

        // 验证错误信息包含 non-prompt-guided 关键字
        assert!(message.contains("non-prompt-guided"));
    }

    #[tokio::test]
    async fn provider_chat_prompt_guided_prepends_missing_system_message() {
        let provider = EchoSystemProvider { supports_native: false };
        let tools =
            vec![ToolSpec::new("shell", "Run commands", serde_json::json!({"type": "object"}))];
        let request = ChatRequest { messages: &[ChatMessage::user("Hello")], tools: Some(&tools) };

        let response = provider.chat(request, "model", 0.7).await.unwrap();
        let text = response.text.unwrap_or_default();

        assert!(text.starts_with("## Tool Use Protocol"));
        assert!(text.contains("**shell**: Run commands"));
    }

    #[tokio::test]
    async fn provider_chat_prompt_guided_fills_empty_system_without_separator() {
        let provider = CustomConvertProvider;
        let tools =
            vec![ToolSpec::new("shell", "Run commands", serde_json::json!({"type": "object"}))];
        let request = ChatRequest {
            messages: &[ChatMessage::system(""), ChatMessage::user("Hello")],
            tools: Some(&tools),
        };

        let response = provider.chat(request, "model", 0.7).await.unwrap();

        assert_eq!(response.text_or_empty(), "CUSTOM_TOOL_INSTRUCTIONS");
    }

    #[tokio::test]
    async fn provider_chat_with_native_tools_bypasses_prompt_injection() {
        let provider = EchoSystemProvider { supports_native: true };
        let tools =
            vec![ToolSpec::new("shell", "Run commands", serde_json::json!({"type": "object"}))];
        let request = ChatRequest {
            messages: &[ChatMessage::system("BASE"), ChatMessage::user("Hello")],
            tools: Some(&tools),
        };

        let response = provider.chat(request, "model", 0.7).await.unwrap();

        assert_eq!(response.text_or_empty(), "BASE");
        assert!(!response.has_tool_calls());
        assert!(response.usage.is_none());
        assert!(response.reasoning_content.is_none());
    }
}

struct HistoryEchoProvider;

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl Provider for HistoryEchoProvider {
    async fn chat_with_system(
        &self,
        system: Option<&str>,
        message: &str,
        _model: &str,
        _temperature: f64,
    ) -> anyhow::Result<String> {
        Ok(format!("system={}; message={message}", system.unwrap_or("")))
    }
}

struct StreamEchoProvider;

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl Provider for StreamEchoProvider {
    async fn chat_with_system(
        &self,
        _system: Option<&str>,
        _message: &str,
        _model: &str,
        _temperature: f64,
    ) -> anyhow::Result<String> {
        Ok(String::new())
    }

    fn stream_chat_with_system(
        &self,
        system_prompt: Option<&str>,
        message: &str,
        model: &str,
        temperature: f64,
        options: StreamOptions,
    ) -> stream::BoxStream<'static, StreamResult<StreamChunk>> {
        let summary = format!(
            "system={}; message={message}; model={model}; temperature={temperature}; enabled={}; count_tokens={}",
            system_prompt.unwrap_or(""),
            options.enabled,
            options.count_tokens
        );
        Box::pin(stream::iter([Ok(StreamChunk::delta(summary))]))
    }
}

#[tokio::test]
async fn default_history_and_tool_chat_use_system_and_last_user() {
    let provider = HistoryEchoProvider;
    let messages = vec![
        ChatMessage::system("sys"),
        ChatMessage::user("first"),
        ChatMessage::assistant("assistant"),
        ChatMessage::user("last"),
    ];

    let text = provider.chat_with_history(&messages, "model", 0.0).await.unwrap();
    assert_eq!(text, "system=sys; message=last");

    let response = provider.chat_with_tools(&messages, &[], "model", 0.0).await.unwrap();
    assert_eq!(response.text_or_empty(), "system=sys; message=last");
    assert!(!response.has_tool_calls());
}

#[tokio::test]
async fn default_history_handles_empty_messages_and_uses_first_system() {
    let provider = HistoryEchoProvider;

    let empty = provider.chat_with_history(&[], "model", 0.0).await.unwrap();
    assert_eq!(empty, "system=; message=");

    let messages = [
        ChatMessage::system("first-system"),
        ChatMessage::system("second-system"),
        ChatMessage::assistant("no user message"),
    ];
    let without_user = provider.chat_with_history(&messages, "model", 0.0).await.unwrap();
    assert_eq!(without_user, "system=first-system; message=");
}

#[test]
fn provider_default_capabilities_reject_optional_features() {
    let provider = HistoryEchoProvider;

    assert_eq!(provider.capabilities(), ProviderCapabilities::default());
    assert!(!provider.supports_native_tools());
    assert!(!provider.supports_vision());
}

#[tokio::test]
async fn simple_chat_delegates_without_system_prompt_and_warmup_is_noop() {
    let provider = HistoryEchoProvider;
    assert_eq!(
        provider.simple_chat("hello", "model", 0.0).await.unwrap(),
        "system=; message=hello"
    );
    provider.warmup().await.unwrap();
}

#[tokio::test]
async fn chat_with_empty_tools_uses_plain_history_path() {
    let provider = HistoryEchoProvider;
    let messages = [ChatMessage::user("plain")];
    let tools: Vec<ToolSpec> = Vec::new();

    let response = provider
        .chat(ChatRequest { messages: &messages, tools: Some(&tools) }, "model", 0.0)
        .await
        .unwrap();

    assert_eq!(response.text_or_empty(), "system=; message=plain");
}

#[test]
fn stream_chunks_and_options_helpers_are_stable() {
    let chunk = StreamChunk::delta("abcdef").with_token_estimate();
    assert_eq!(chunk.delta, "abcdef");
    assert_eq!(chunk.token_count, 2);
    assert!(!chunk.is_final);

    let final_chunk = StreamChunk::final_chunk();
    assert!(final_chunk.is_final);
    assert!(final_chunk.delta.is_empty());

    let error = StreamChunk::error("boom");
    assert!(error.is_final);
    assert_eq!(error.delta, "boom");

    let options = StreamOptions::new(true).with_token_count();
    assert!(options.enabled);
    assert!(options.count_tokens);
}

#[test]
fn stream_token_estimate_rounds_up_at_four_character_boundaries() {
    for (text, expected) in [("", 0), ("a", 1), ("abcd", 1), ("abcde", 2)] {
        assert_eq!(StreamChunk::delta(text).with_token_estimate().token_count, expected);
    }

    let defaults = StreamOptions::default();
    assert!(!defaults.enabled);
    assert!(!defaults.count_tokens);
}

#[tokio::test]
async fn default_streaming_returns_empty_stream() {
    use futures_util::StreamExt;

    let provider = HistoryEchoProvider;
    let messages = [ChatMessage::user("hi")];
    let mut stream =
        provider.stream_chat_with_history(&messages, "model", 0.0, StreamOptions::new(true));

    assert!(stream.next().await.is_none());
    assert!(!provider.supports_streaming());
}

#[tokio::test]
async fn stream_history_forwards_first_system_last_user_and_options() {
    use futures_util::StreamExt;

    let provider = StreamEchoProvider;
    let messages = [
        ChatMessage::user("first"),
        ChatMessage::system("first-system"),
        ChatMessage::system("second-system"),
        ChatMessage::user("last"),
    ];
    let options = StreamOptions::new(true).with_token_count();
    let mut stream = provider.stream_chat_with_history(&messages, "model", 0.25, options);

    let chunk = stream.next().await.unwrap().unwrap();

    assert_eq!(
        chunk.delta,
        "system=first-system; message=last; model=model; temperature=0.25; enabled=true; count_tokens=true"
    );
    assert!(stream.next().await.is_none());

    let mut empty_stream =
        provider.stream_chat_with_history(&[], "empty-model", 0.0, StreamOptions::default());
    let empty_chunk = empty_stream.next().await.unwrap().unwrap();
    assert_eq!(
        empty_chunk.delta,
        "system=; message=; model=empty-model; temperature=0; enabled=false; count_tokens=false"
    );
}

#[test]
fn conversation_messages_round_trip_all_tagged_variants() {
    let messages = [
        ConversationMessage::Chat(ChatMessage::assistant("answer")),
        ConversationMessage::AssistantToolCalls {
            text: Some("calling".to_string()),
            tool_calls: vec![ToolCall {
                id: "call-1".to_string(),
                name: "lookup".to_string(),
                arguments: r#"{"query":"rust"}"#.to_string(),
            }],
            reasoning_content: Some("reasoning".to_string()),
        },
        ConversationMessage::ToolResults(vec![ToolResultMessage {
            tool_call_id: "call-1".to_string(),
            content: "result".to_string(),
        }]),
    ];

    for message in messages {
        let json = serde_json::to_string(&message).unwrap();
        let decoded: ConversationMessage = serde_json::from_str(&json).unwrap();

        match decoded {
            ConversationMessage::Chat(chat) => {
                assert_eq!(chat.role, "assistant");
                assert_eq!(chat.content, "answer");
            }
            ConversationMessage::AssistantToolCalls { text, tool_calls, reasoning_content } => {
                assert_eq!(text.as_deref(), Some("calling"));
                assert_eq!(tool_calls.len(), 1);
                assert_eq!(tool_calls[0].id, "call-1");
                assert_eq!(tool_calls[0].name, "lookup");
                assert_eq!(tool_calls[0].arguments, r#"{"query":"rust"}"#);
                assert_eq!(reasoning_content.as_deref(), Some("reasoning"));
            }
            ConversationMessage::ToolResults(results) => {
                assert_eq!(results.len(), 1);
                assert_eq!(results[0].tool_call_id, "call-1");
                assert_eq!(results[0].content, "result");
            }
        }
    }
}

#[test]
fn capability_error_and_stream_errors_display_context() {
    let err = ProviderCapabilityError {
        provider: "p".to_string(),
        capability: "vision".to_string(),
        message: "disabled".to_string(),
    };
    assert!(err.to_string().contains("provider=p"));
    assert!(StreamError::Provider("bad".to_string()).to_string().contains("bad"));
    assert!(StreamError::InvalidSse("line".to_string()).to_string().contains("line"));

    let json_error = serde_json::from_str::<serde_json::Value>("{").unwrap_err();
    assert!(StreamError::Json(json_error).to_string().contains("JSON parse error"));

    let io_error = std::io::Error::new(std::io::ErrorKind::PermissionDenied, "denied");
    assert!(StreamError::from(io_error).to_string().contains("denied"));

    let http_error = reqwest::Client::new().get("\n").build().unwrap_err();
    assert!(StreamError::Http(http_error).to_string().contains("HTTP error"));
}
