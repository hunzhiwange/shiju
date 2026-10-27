//! Hook 运行器模块
//!
//! 本模块提供了 `HookRunner` 结构体，用于管理和分发已注册的钩子处理器。
//!
//! # 核心功能
//!
//! - **钩子注册**：支持动态注册钩子处理器，并按优先级自动排序
//! - **并行分发**：Void 类型钩子（无返回值的观察型钩子）通过 `join_all` 并行执行
//! - **顺序执行**：修改型钩子按优先级顺序执行（高优先级先执行），支持管道式传递输出和短路取消
//!
//! # 钩子类型
//!
//! ## Void 钩子（观察型）
//!
//! 这些钩子仅用于观察和记录，不修改数据，并行执行以提高性能：
//! - 网关生命周期事件（启动/停止）
//! - 会话生命周期事件（开始/结束）
//! - LLM 交互事件（输入/输出）
//! - 工具调用后事件
//! - 消息发送事件
//! - 心跳事件
//!
//! ## 修改型钩子
//!
//! 这些钩子可以修改数据，按优先级顺序执行，任一钩子返回 `Cancel` 时短路终止：
//! - 模型解析前
//! - 提示词构建前
//! - LLM 调用前
//! - 工具调用前
//! - 消息接收时
//! - 消息发送时

use std::time::Duration;

use futures_util::{FutureExt, future::join_all};
use serde_json::Value;
use std::panic::AssertUnwindSafe;
use tracing::info;

use crate::app::agent::channels::traits::ChannelMessage;
use crate::app::agent::providers::traits::{ChatMessage, ChatResponse};
use crate::app::agent::tools::traits::ToolResult;

use super::traits::{HookHandler, HookResult};

/// 钩子运行器，管理已注册的钩子处理器集合
///
/// `HookRunner` 负责协调多个钩子处理器的执行，根据钩子类型采用不同的执行策略：
///
/// # 执行策略
///
/// - **Void 钩子**：通过 `join_all` 并行分发，适用于观察型钩子（如日志记录、监控）
/// - **修改型钩子**：按优先级降序顺序执行，输出作为下一个钩子的输入，遇到 `Cancel` 时短路终止
///
/// # 优先级排序
///
/// 处理器在注册时自动按优先级降序排列，确保高优先级钩子先执行。
///
/// # 错误处理
///
/// 修改型钩子使用 `catch_unwind` 捕获 panic，防止单个钩子崩溃影响整个系统。
/// 执行前门禁遇到 panic 时拒绝操作，其余修改钩子保留先前值。
pub struct HookRunner {
    /// 已注册的钩子处理器列表，按优先级降序排列
    handlers: Vec<Box<dyn HookHandler>>,
}

impl HookRunner {
    /// 创建一个空的钩子运行器
    ///
    /// # 返回值
    ///
    /// 返回一个不包含任何处理器的 `HookRunner` 实例
    ///
    /// # 示例
    ///
    /// ```ignore
    /// let runner = HookRunner::new();
    /// ```
    pub fn new() -> Self {
        Self { handlers: Vec::new() }
    }

    /// 注册一个钩子处理器并重新排序
    ///
    /// 处理器注册后会按优先级降序重新排列整个处理器列表，
    /// 确保高优先级处理器在执行时排在前面。
    ///
    /// # 参数
    ///
    /// - `handler`：要注册的钩子处理器，必须是实现了 `HookHandler` trait 的类型
    ///
    /// # 示例
    ///
    /// ```ignore
    /// let mut runner = HookRunner::new();
    /// runner.register(Box::new(MyHook::new()));
    /// ```
    pub fn register(&mut self, handler: Box<dyn HookHandler>) {
        self.handlers.push(handler);
        // 按优先级降序排序，使用 Reverse 确保高优先级在前
        self.handlers.sort_by_key(|h| std::cmp::Reverse(h.priority()));
    }

    // ============================================================
    // Void 分发器（并行执行、即发即忘）
    // 这些方法不返回修改后的数据，仅用于观察和记录事件
    // ============================================================

    /// 触发网关启动事件
    ///
    /// 并行通知所有处理器网关已启动。
    ///
    /// # 参数
    ///
    /// - `host`：网关监听的主机地址
    /// - `port`：网关监听的端口号
    pub async fn fire_gateway_start(&self, host: &str, port: u16) {
        let futs: Vec<_> = self.handlers.iter().map(|h| h.on_gateway_start(host, port)).collect();
        join_all(futs).await;
    }

    /// 触发网关停止事件
    ///
    /// 并行通知所有处理器网关已停止。
    pub async fn fire_gateway_stop(&self) {
        let futs: Vec<_> = self.handlers.iter().map(|h| h.on_gateway_stop()).collect();
        join_all(futs).await;
    }

    /// 触发会话开始事件
    ///
    /// 并行通知所有处理器新会话已开始。
    ///
    /// # 参数
    ///
    /// - `session_id`：会话的唯一标识符
    /// - `channel`：会话所属的通道名称
    pub async fn fire_session_start(&self, session_id: &str, channel: &str) {
        let futs: Vec<_> =
            self.handlers.iter().map(|h| h.on_session_start(session_id, channel)).collect();
        join_all(futs).await;
    }

    /// 触发会话结束事件
    ///
    /// 并行通知所有处理器会话已结束。
    ///
    /// # 参数
    ///
    /// - `session_id`：会话的唯一标识符
    /// - `channel`：会话所属的通道名称
    pub async fn fire_session_end(&self, session_id: &str, channel: &str) {
        let futs: Vec<_> =
            self.handlers.iter().map(|h| h.on_session_end(session_id, channel)).collect();
        join_all(futs).await;
    }

    /// 触发 LLM 输入事件
    ///
    /// 并行通知所有处理器即将发送给 LLM 的消息。
    ///
    /// # 参数
    ///
    /// - `messages`：发送给 LLM 的消息列表
    /// - `model`：使用的模型名称
    pub async fn fire_llm_input(&self, messages: &[ChatMessage], model: &str) {
        let futs: Vec<_> = self.handlers.iter().map(|h| h.on_llm_input(messages, model)).collect();
        join_all(futs).await;
    }

    /// 触发 LLM 输出事件
    ///
    /// 并行通知所有处理器 LLM 返回的响应。
    ///
    /// # 参数
    ///
    /// - `response`：LLM 的响应结果
    pub async fn fire_llm_output(&self, response: &ChatResponse) {
        let futs: Vec<_> = self.handlers.iter().map(|h| h.on_llm_output(response)).collect();
        join_all(futs).await;
    }

    /// 触发工具调用完成事件
    ///
    /// 并行通知所有处理器工具调用已完成。
    ///
    /// # 参数
    ///
    /// - `tool`：被调用的工具名称
    /// - `result`：工具执行的返回结果
    /// - `duration`：工具执行耗时
    pub async fn fire_after_tool_call(&self, tool: &str, result: &ToolResult, duration: Duration) {
        let futs: Vec<_> =
            self.handlers.iter().map(|h| h.on_after_tool_call(tool, result, duration)).collect();
        join_all(futs).await;
    }

    /// 触发消息发送完成事件
    ///
    /// 并行通知所有处理器消息已发送。
    ///
    /// # 参数
    ///
    /// - `channel`：消息发送的通道名称
    /// - `recipient`：消息接收者标识
    /// - `content`：消息内容
    pub async fn fire_message_sent(&self, channel: &str, recipient: &str, content: &str) {
        let futs: Vec<_> =
            self.handlers.iter().map(|h| h.on_message_sent(channel, recipient, content)).collect();
        join_all(futs).await;
    }

    /// 触发心跳事件
    ///
    /// 并行通知所有处理器心跳触发，用于定期健康检查或状态更新。
    pub async fn fire_heartbeat_tick(&self) {
        let futs: Vec<_> = self.handlers.iter().map(|h| h.on_heartbeat_tick()).collect();
        join_all(futs).await;
    }

    // ============================================================
    // 修改型分发器（按优先级顺序执行、遇到 Cancel 短路终止）
    // 这些方法可以修改数据，前一个钩子的输出作为后一个钩子的输入
    // ============================================================

    /// 运行模型解析前的修改钩子
    ///
    /// 按优先级顺序执行所有处理器的 `before_model_resolve` 钩子。
    /// 每个钩子可以修改 provider 和 model，修改后的值传递给下一个钩子。
    ///
    /// # 参数
    ///
    /// - `provider`：初始的 provider 名称
    /// - `model`：初始的 model 名称
    ///
    /// # 返回值
    ///
    /// - `HookResult::Continue((provider, model))`：所有钩子执行完成后的最终值
    /// - `HookResult::Cancel(reason)`：任一钩子取消操作并返回取消原因
    ///
    /// # 错误处理
    ///
    /// 如果钩子发生 panic，记录错误日志并继续使用之前的值。
    pub async fn run_before_model_resolve(
        &self,
        mut provider: String,
        mut model: String,
    ) -> HookResult<(String, String)> {
        for h in &self.handlers {
            let hook_name = h.name();
            // 使用 catch_unwind 捕获 panic，防止单个钩子崩溃影响系统
            match AssertUnwindSafe(h.before_model_resolve(provider.clone(), model.clone()))
                .catch_unwind()
                .await
            {
                // 钩子正常执行，更新值继续传递
                Ok(HookResult::Continue((p, m))) => {
                    provider = p;
                    model = m;
                }
                // 钩子返回取消，短路终止并返回取消原因
                Ok(HookResult::Cancel(reason)) => {
                    info!(hook = hook_name, reason, "before_model_resolve cancelled by hook");
                    return HookResult::Cancel(reason);
                }
                // 钩子发生 panic，记录错误但继续使用之前的值
                Err(_) => {
                    tracing::error!(
                        hook = hook_name,
                        "before_model_resolve hook panicked; continuing with previous values"
                    );
                }
            }
        }
        HookResult::Continue((provider, model))
    }

    /// 运行提示词构建前的修改钩子
    ///
    /// 按优先级顺序执行所有处理器的 `before_prompt_build` 钩子。
    /// 每个钩子可以修改提示词内容，修改后的值传递给下一个钩子。
    ///
    /// # 参数
    ///
    /// - `prompt`：初始的提示词内容
    ///
    /// # 返回值
    ///
    /// - `HookResult::Continue(prompt)`：所有钩子执行完成后的最终提示词
    /// - `HookResult::Cancel(reason)`：任一钩子取消操作并返回取消原因
    ///
    /// # 错误处理
    ///
    /// 如果钩子发生 panic，记录错误日志并继续使用之前的值。
    pub async fn run_before_prompt_build(&self, mut prompt: String) -> HookResult<String> {
        for h in &self.handlers {
            let hook_name = h.name();
            match AssertUnwindSafe(h.before_prompt_build(prompt.clone())).catch_unwind().await {
                Ok(HookResult::Continue(p)) => prompt = p,
                Ok(HookResult::Cancel(reason)) => {
                    info!(hook = hook_name, reason, "before_prompt_build cancelled by hook");
                    return HookResult::Cancel(reason);
                }
                Err(_) => {
                    tracing::error!(
                        hook = hook_name,
                        "before_prompt_build hook panicked; continuing with previous value"
                    );
                }
            }
        }
        HookResult::Continue(prompt)
    }

    /// 运行 LLM 调用前的修改钩子
    ///
    /// 按优先级顺序执行所有处理器的 `before_llm_call` 钩子。
    /// 每个钩子可以修改消息列表和模型名称，修改后的值传递给下一个钩子。
    ///
    /// # 参数
    ///
    /// - `messages`：初始的消息列表
    /// - `model`：初始的模型名称
    ///
    /// # 返回值
    ///
    /// - `HookResult::Continue((messages, model))`：所有钩子执行完成后的最终值
    /// - `HookResult::Cancel(reason)`：任一钩子取消操作并返回取消原因
    ///
    /// # 错误处理
    ///
    /// 如果钩子发生 panic，记录错误日志并拒绝当前执行。
    pub async fn run_before_llm_call(
        &self,
        mut messages: Vec<ChatMessage>,
        mut model: String,
    ) -> HookResult<(Vec<ChatMessage>, String)> {
        for h in &self.handlers {
            let hook_name = h.name();
            match AssertUnwindSafe(h.before_llm_call(messages.clone(), model.clone()))
                .catch_unwind()
                .await
            {
                Ok(HookResult::Continue((m, mdl))) => {
                    messages = m;
                    model = mdl;
                }
                Ok(HookResult::Cancel(reason)) => {
                    info!(hook = hook_name, reason, "before_llm_call cancelled by hook");
                    return HookResult::Cancel(reason);
                }
                Err(_) => {
                    tracing::error!(
                        hook = hook_name,
                        "before_llm_call hook panicked; rejecting execution"
                    );
                    return HookResult::Cancel("before_llm_call hook panicked".into());
                }
            }
        }
        HookResult::Continue((messages, model))
    }

    /// 运行工具调用前的修改钩子
    ///
    /// 按优先级顺序执行所有处理器的 `before_tool_call` 钩子。
    /// 每个钩子可以修改工具名称和参数，修改后的值传递给下一个钩子。
    ///
    /// # 参数
    ///
    /// - `name`：初始的工具名称
    /// - `args`：初始的工具参数（JSON 格式）
    ///
    /// # 返回值
    ///
    /// - `HookResult::Continue((name, args))`：所有钩子执行完成后的最终值
    /// - `HookResult::Cancel(reason)`：任一钩子取消操作并返回取消原因
    ///
    /// # 错误处理
    ///
    /// 如果钩子发生 panic，记录错误日志并拒绝当前执行。
    pub async fn run_before_tool_call(
        &self,
        mut name: String,
        mut args: Value,
    ) -> HookResult<(String, Value)> {
        for h in &self.handlers {
            let hook_name = h.name();
            match AssertUnwindSafe(h.before_tool_call(name.clone(), args.clone()))
                .catch_unwind()
                .await
            {
                Ok(HookResult::Continue((n, a))) => {
                    name = n;
                    args = a;
                }
                Ok(HookResult::Cancel(reason)) => {
                    info!(hook = hook_name, reason, "before_tool_call cancelled by hook");
                    return HookResult::Cancel(reason);
                }
                Err(_) => {
                    tracing::error!(
                        hook = hook_name,
                        "before_tool_call hook panicked; rejecting execution"
                    );
                    return HookResult::Cancel("before_tool_call hook panicked".into());
                }
            }
        }
        HookResult::Continue((name, args))
    }

    /// 运行消息接收时的修改钩子
    ///
    /// 按优先级顺序执行所有处理器的 `on_message_received` 钩子。
    /// 每个钩子可以修改接收到的消息，修改后的值传递给下一个钩子。
    ///
    /// # 参数
    ///
    /// - `message`：初始接收到的通道消息
    ///
    /// # 返回值
    ///
    /// - `HookResult::Continue(message)`：所有钩子执行完成后的最终消息
    /// - `HookResult::Cancel(reason)`：任一钩子取消操作并返回取消原因
    ///
    /// # 错误处理
    ///
    /// 如果钩子发生 panic，记录错误日志并继续使用之前的消息。
    pub async fn run_on_message_received(
        &self,
        mut message: ChannelMessage,
    ) -> HookResult<ChannelMessage> {
        for h in &self.handlers {
            let hook_name = h.name();
            match AssertUnwindSafe(h.on_message_received(message.clone())).catch_unwind().await {
                Ok(HookResult::Continue(m)) => message = m,
                Ok(HookResult::Cancel(reason)) => {
                    info!(hook = hook_name, reason, "on_message_received cancelled by hook");
                    return HookResult::Cancel(reason);
                }
                Err(_) => {
                    tracing::error!(
                        hook = hook_name,
                        "on_message_received hook panicked; continuing with previous message"
                    );
                }
            }
        }
        HookResult::Continue(message)
    }

    /// 运行消息发送时的修改钩子
    ///
    /// 按优先级顺序执行所有处理器的 `on_message_sending` 钩子。
    /// 每个钩子可以修改通道、接收者和内容，修改后的值传递给下一个钩子。
    ///
    /// # 参数
    ///
    /// - `channel`：初始的通道名称
    /// - `recipient`：初始的接收者标识
    /// - `content`：初始的消息内容
    ///
    /// # 返回值
    ///
    /// - `HookResult::Continue((channel, recipient, content))`：所有钩子执行完成后的最终值
    /// - `HookResult::Cancel(reason)`：任一钩子取消操作并返回取消原因
    ///
    /// # 错误处理
    ///
    /// 如果钩子发生 panic，记录错误日志并继续使用之前的值。
    pub async fn run_on_message_sending(
        &self,
        mut channel: String,
        mut recipient: String,
        mut content: String,
    ) -> HookResult<(String, String, String)> {
        for h in &self.handlers {
            let hook_name = h.name();
            match AssertUnwindSafe(h.on_message_sending(
                channel.clone(),
                recipient.clone(),
                content.clone(),
            ))
            .catch_unwind()
            .await
            {
                Ok(HookResult::Continue((c, r, ct))) => {
                    channel = c;
                    recipient = r;
                    content = ct;
                }
                Ok(HookResult::Cancel(reason)) => {
                    info!(hook = hook_name, reason, "on_message_sending cancelled by hook");
                    return HookResult::Cancel(reason);
                }
                Err(_) => {
                    tracing::error!(
                        hook = hook_name,
                        "on_message_sending hook panicked; continuing with previous message"
                    );
                }
            }
        }
        HookResult::Continue((channel, recipient, content))
    }
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
