//! # 钩子处理器特质模块
//!
//! 本模块定义了 VibeWindow 代理系统中的钩子机制核心特质和类型。
//! 钩子系统允许在代理生命周期的关键点插入自定义逻辑，用于日志记录、
//! 监控、审计、数据转换或流程控制等场景。
//!
//! ## 核心概念
//!
//! - **钩子处理器 (HookHandler)**：实现此特质以定义自定义钩子行为
//! - **钩子结果 (HookResult)**：修改型钩子的返回值，可继续或取消操作
//! - **空钩子 (Void Hooks)**：并行执行的观察型钩子，无法修改数据或取消操作
//! - **修改型钩子 (Modifying Hooks)**：按优先级顺序执行，可修改数据或取消操作
//!
//! ## 使用示例
//!
//! ```rust
//! use crate::app::agent::hooks::traits::{HookHandler, HookResult};
//!
//! struct LoggingHook;
//!
//! #[async_trait]
//! impl HookHandler for LoggingHook {
//!     fn name(&self) -> &str {
//!         "logging-hook"
//!     }
//!
//!     async fn on_gateway_start(&self, host: &str, port: u16) {
//!         println!("Gateway started on {}:{}", host, port);
//!     }
//! }
//! ```

use async_trait::async_trait;
use serde_json::Value;
use std::time::Duration;

use crate::app::agent::channels::traits::ChannelMessage;
use crate::app::agent::providers::traits::{ChatMessage, ChatResponse};
use crate::app::agent::tools::traits::ToolResult;

/// 修改型钩子的执行结果
///
/// 此枚举用于可修改数据的钩子方法，允许钩子决定是继续执行（可能使用修改后的数据）
/// 还是取消整个操作并返回错误原因。
///
/// # 类型参数
///
/// - `T`：继续执行时携带的数据类型
///
/// # 变体
///
/// - `Continue(T)`：继续执行，携带可能被修改的数据
/// - `Cancel(String)`：取消操作，包含取消原因的描述信息
///
/// # 示例
///
/// ```rust
/// use crate::app::agent::hooks::traits::HookResult;
///
/// let result: HookResult<String> = HookResult::Continue("processed".to_string());
/// assert!(!result.is_cancel());
///
/// let cancelled: HookResult<String> = HookResult::Cancel("Validation failed".to_string());
/// assert!(cancelled.is_cancel());
/// ```
#[derive(Debug, Clone)]
pub enum HookResult<T> {
    /// 继续执行，携带（可能修改后的）数据
    Continue(T),
    /// 取消操作，携带取消原因
    Cancel(String),
}

impl<T> HookResult<T> {
    /// 检查此结果是否为取消状态
    ///
    /// # 返回值
    ///
    /// 如果是 `Cancel` 变体则返回 `true`，否则返回 `false`
    ///
    /// # 示例
    ///
    /// ```rust
    /// use crate::app::agent::hooks::traits::HookResult;
    ///
    /// let result = HookResult::<i32>::Cancel("error".to_string());
    /// assert!(result.is_cancel());
    /// ```
    pub fn is_cancel(&self) -> bool {
        matches!(self, HookResult::Cancel(_))
    }
}

/// 钩子处理器的特质边界约束
///
/// 在非 WASM32 目标平台上，要求钩子处理器必须实现 `Send + Sync`，
/// 以支持多线程并发执行。在 WASM32 目标平台上，由于 WebAssembly 的
/// 单线程特性，不施加任何约束。
///
/// 这种条件编译确保了钩子系统在不同平台上的最佳兼容性和性能。
#[cfg(not(target_arch = "wasm32"))]
pub trait HookHandlerBounds: Send + Sync {}

/// 为所有满足 `Send + Sync` 的类型自动实现特质边界
#[cfg(not(target_arch = "wasm32"))]
impl<T: Send + Sync> HookHandlerBounds for T {}

/// 钩子处理器的特质边界约束（WASM32 平台）
///
/// 在 WASM32 目标平台上，不施加任何特质约束，以兼容 WebAssembly 的执行环境。
#[cfg(target_arch = "wasm32")]
pub trait HookHandlerBounds {}

/// 为所有类型自动实现特质边界（WASM32 平台）
#[cfg(target_arch = "wasm32")]
impl<T> HookHandlerBounds for T {}

/// 钩子处理器核心特质
///
/// 定义了代理系统中所有可用的钩子事件。所有方法都提供了默认的空实现，
/// 实现者只需覆盖关心的事件即可。
///
/// # 钩子类型
///
/// 钩子分为两类：
///
/// 1. **空钩子 (Void Hooks)**：观察型钩子，无法修改数据或取消操作，并行执行
///    - 性能开销小，适合日志、监控等场景
///    - 即使某个钩子失败，其他钩子仍会执行
///
/// 2. **修改型钩子 (Modifying Hooks)**：可修改数据或取消操作，按优先级顺序执行
///    - 返回 `HookResult::Continue` 继续执行（可能携带修改后的数据）
///    - 返回 `HookResult::Cancel` 取消整个操作
///    - 任一钩子取消后，后续钩子不再执行
///
/// # 优先级
///
/// 修改型钩子按 `priority()` 返回值排序执行，数值越小越先执行。
/// 默认优先级为 0。空钩子的执行顺序不保证。
///
/// # 线程安全
///
/// 在非 WASM32 平台上，钩子处理器必须是线程安全的（`Send + Sync`），
/// 因为它们可能在多个线程间共享和调用。
///
/// # 示例
///
/// ```rust
/// use crate::app::agent::hooks::traits::{HookHandler, HookResult};
/// use async_trait::async_trait;
///
/// struct SecurityHook;
///
/// #[async_trait]
/// impl HookHandler for SecurityHook {
///     fn name(&self) -> &str {
///         "security-hook"
///     }
///
///     fn priority(&self) -> i32 {
///         -100  // 高优先级，优先执行
///     }
///
///     async fn before_tool_call(&self, name: String, args: Value) -> HookResult<(String, Value)> {
///         if name == "dangerous_tool" {
///             HookResult::Cancel("Tool not allowed".to_string())
///         } else {
///             HookResult::Continue((name, args))
///         }
///     }
/// }
/// ```
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
pub trait HookHandler: HookHandlerBounds {
    /// 返回钩子处理器的名称
    ///
    /// 名称用于日志记录、调试和钩子管理。建议使用唯一且有描述性的名称。
    ///
    /// # 返回值
    ///
    /// 钩子处理器的字符串标识符
    fn name(&self) -> &str;

    /// 返回钩子的执行优先级
    ///
    /// 对于修改型钩子，优先级决定了执行顺序。数值越小，越先执行。
    /// 空钩子的执行顺序不受优先级影响。
    ///
    /// # 返回值
    ///
    /// 优先级数值，默认为 0
    fn priority(&self) -> i32 {
        0
    }

    // ============ 空钩子（并行执行，即发即忘）============

    /// 网关启动时触发
    ///
    /// 当代理的 HTTP/WebSocket 网关成功启动时调用此钩子。
    /// 适合用于健康检查通知、启动日志记录等场景。
    ///
    /// # 参数
    ///
    /// - `_host`：网关监听的主机地址
    /// - `_port`：网关监听的端口号
    async fn on_gateway_start(&self, _host: &str, _port: u16) {}

    /// 网关停止时触发
    ///
    /// 当代理的网关停止服务时调用此钩子。
    /// 适合用于清理资源、发送停机通知等场景。
    async fn on_gateway_stop(&self) {}

    /// 会话开始时触发
    ///
    /// 当新用户会话建立时调用此钩子。会话代表与特定通道上特定用户的
    /// 连续对话上下文。
    ///
    /// # 参数
    ///
    /// - `_session_id`：唯一会话标识符
    /// - `_channel`：会话所在的通道标识（如 "telegram"、"discord"）
    async fn on_session_start(&self, _session_id: &str, _channel: &str) {}

    /// 会话结束时触发
    ///
    /// 当用户会话终止时调用此钩子。会话可能因超时、用户主动结束
    /// 或系统关闭而终止。
    ///
    /// # 参数
    ///
    /// - `_session_id`：唯一会话标识符
    /// - `_channel`：会话所在的通道标识
    async fn on_session_end(&self, _session_id: &str, _channel: &str) {}

    /// LLM 输入数据准备完成时触发
    ///
    /// 在构建完成发送给大语言模型的消息列表后调用此钩子。
    /// 适合用于记录对话历史、分析输入模式等场景。
    ///
    /// # 参数
    ///
    /// - `_messages`：准备发送给 LLM 的消息列表
    /// - `_model`：目标模型标识符
    async fn on_llm_input(&self, _messages: &[ChatMessage], _model: &str) {}

    /// LLM 响应返回时触发
    ///
    /// 当从大语言模型接收到响应时调用此钩子。
    /// 适合用于记录模型输出、质量监控等场景。
    ///
    /// # 参数
    ///
    /// - `_response`：LLM 返回的完整响应对象
    async fn on_llm_output(&self, _response: &ChatResponse) {}

    /// 工具调用完成后触发
    ///
    /// 当工具执行完成时调用此钩子，无论执行成功或失败。
    /// 适合用于工具使用统计、性能监控、审计日志等场景。
    ///
    /// # 参数
    ///
    /// - `_tool`：被执行的工具名称
    /// - `_result`：工具执行的返回结果
    /// - `_duration`：工具执行的耗时
    async fn on_after_tool_call(&self, _tool: &str, _result: &ToolResult, _duration: Duration) {}

    /// 消息发送完成时触发
    ///
    /// 当代理成功向用户发送消息后调用此钩子。
    /// 适合用于发送确认、消息追踪等场景。
    ///
    /// # 参数
    ///
    /// - `_channel`：消息发送的通道标识
    /// - `_recipient`：消息接收者标识
    /// - `_content`：发送的消息内容
    async fn on_message_sent(&self, _channel: &str, _recipient: &str, _content: &str) {}

    /// 心跳定时器触发
    ///
    /// 周期性调用此钩子，用于执行定期任务，如健康检查、
    /// 统计上报、缓存清理等。
    async fn on_heartbeat_tick(&self) {}

    // ============ 修改型钩子（按优先级顺序执行，可取消）============

    /// 模型解析前触发
    ///
    /// 在系统将模型标识符解析为实际 Provider 和模型配置之前调用。
    /// 可用于动态路由、模型别名映射或访问控制。
    ///
    /// # 参数
    ///
    /// - `provider`：原始 Provider 标识符
    /// - `model`：原始模型标识符
    ///
    /// # 返回值
    ///
    /// - `Continue((provider, model))`：继续使用（可能修改后的）Provider 和模型
    /// - `Cancel(reason)`：取消操作，指定取消原因
    ///
    /// # 示例
    ///
    /// ```rust
    /// async fn before_model_resolve(
    ///     &self,
    ///     provider: String,
    ///     model: String,
    /// ) -> HookResult<(String, String)> {
    ///     // 将 "gpt4" 别名映射到实际模型
    ///     let actual_model = if model == "gpt4" {
    ///         "gpt-4-turbo-preview".to_string()
    ///     } else {
    ///         model
    ///     };
    ///     HookResult::Continue((provider, actual_model))
    /// }
    /// ```
    async fn before_model_resolve(
        &self,
        provider: String,
        model: String,
    ) -> HookResult<(String, String)> {
        HookResult::Continue((provider, model))
    }

    /// 提示词构建前触发
    ///
    /// 在系统构建最终提示词模板之前调用。可用于动态调整提示词、
    /// 注入上下文信息或实现提示词策略。
    ///
    /// # 参数
    ///
    /// - `prompt`：原始提示词内容
    ///
    /// # 返回值
    ///
    /// - `Continue(prompt)`：继续使用（可能修改后的）提示词
    /// - `Cancel(reason)`：取消操作，指定取消原因
    async fn before_prompt_build(&self, prompt: String) -> HookResult<String> {
        HookResult::Continue(prompt)
    }

    /// LLM 调用前触发
    ///
    /// 在实际调用大语言模型 API 之前调用。这是修改消息或模型的最后机会。
    /// 可用于消息过滤、安全检查、成本控制等场景。
    ///
    /// # 参数
    ///
    /// - `messages`：准备发送的消息列表
    /// - `model`：目标模型标识符
    ///
    /// # 返回值
    ///
    /// - `Continue((messages, model))`：继续使用（可能修改后的）消息和模型
    /// - `Cancel(reason)`：取消操作，指定取消原因
    ///
    /// # 注意
    ///
    /// 此钩子在 `before_model_resolve` 和 `before_prompt_build` 之后执行。
    async fn before_llm_call(
        &self,
        messages: Vec<ChatMessage>,
        model: String,
    ) -> HookResult<(Vec<ChatMessage>, String)> {
        HookResult::Continue((messages, model))
    }

    /// 工具调用前触发
    ///
    /// 在工具实际执行之前调用。可用于参数验证、权限检查、
    /// 工具替换或调用限制。
    ///
    /// # 参数
    ///
    /// - `name`：工具名称
    /// - `args`：工具参数（JSON 格式）
    ///
    /// # 返回值
    ///
    /// - `Continue((name, args))`：继续使用（可能修改后的）工具名和参数
    /// - `Cancel(reason)`：取消操作，指定取消原因
    ///
    /// # 示例
    ///
    /// ```rust
    /// async fn before_tool_call(
    ///     &self,
    ///     name: String,
    ///     args: Value,
    /// ) -> HookResult<(String, Value)> {
    ///     // 禁止执行危险的 shell 命令
    ///     if name == "shell" {
    ///         return HookResult::Cancel("Shell tool is disabled".to_string());
    ///     }
    ///     HookResult::Continue((name, args))
    /// }
    /// ```
    async fn before_tool_call(&self, name: String, args: Value) -> HookResult<(String, Value)> {
        HookResult::Continue((name, args))
    }

    /// 消息接收时触发
    ///
    /// 当从通道接收到用户消息时调用。可用于消息过滤、
    /// 内容转换、反垃圾或审计记录。
    ///
    /// # 参数
    ///
    /// - `message`：接收到的原始消息
    ///
    /// # 返回值
    ///
    /// - `Continue(message)`：继续处理（可能修改后的）消息
    /// - `Cancel(reason)`：取消处理，消息将被丢弃
    async fn on_message_received(&self, message: ChannelMessage) -> HookResult<ChannelMessage> {
        HookResult::Continue(message)
    }

    /// 消息发送前触发
    ///
    /// 在代理向用户发送消息之前调用。可用于消息审查、
    /// 内容过滤、格式转换或添加签名。
    ///
    /// # 参数
    ///
    /// - `channel`：目标通道标识
    /// - `recipient`：接收者标识
    /// - `content`：待发送的消息内容
    ///
    /// # 返回值
    ///
    /// - `Continue((channel, recipient, content))`：继续发送（可能修改后的）消息
    /// - `Cancel(reason)`：取消发送
    async fn on_message_sending(
        &self,
        channel: String,
        recipient: String,
        content: String,
    ) -> HookResult<(String, String, String)> {
        HookResult::Continue((channel, recipient, content))
    }
}

/// 测试模块
///
/// 包含钩子特质和结果类型的单元测试，位于 `tests.rs` 文件中。
#[cfg(test)]
#[path = "tests.rs"]
mod tests;

#[cfg(test)]
#[path = "mod_tests.rs"]
mod mod_tests;
