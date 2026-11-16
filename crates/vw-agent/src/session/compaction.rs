//! 会话压缩（Compaction）模块
//!
//! 本模块提供了会话历史消息的压缩和修剪功能，用于在会话上下文接近模型限制时
//! 通过独立检查点缩短模型上下文，保留原始聊天记录。
//!
//! # 核心功能
//!
//! - **溢出检测**：判断当前会话是否接近上下文限制，需要执行压缩
//! - **消息修剪**：移除旧的、已完成的工具调用输出，减少上下文占用
//! - **摘要生成**：通过统一模型请求生成单次交接摘要，成功后原子保存检查点
//! - **压缩请求创建**：创建新的压缩请求消息，触发压缩流程
//!
//! # 压缩策略
//!
//! 1. 当会话使用率达到 总上下文的 90%（受输入和输出预算约束）时，触发溢出检测
//! 2. 修剪时会保护最近 2 轮对话和所有 skill 工具调用
//! 3. 只有当可修剪的 token 数量超过最小阈值时才执行实际修剪
//! 4. 压缩后会发布 `session.compacted` 事件通知相关组件

pub use session_engine::compaction::{ProcessInput, CreateInput, CompactionStats, CompactionOutcome, PRUNE_MINIMUM, PRUNE_PROTECT, event, is_overflow, prune, create, restore_invoked_skills_after_compaction};
type Host = crate::tools::context::AgentContextHost;
impl session_engine::compaction::CompactionHost for Host {
    fn snapshot(session: &str) -> anyhow::Result<Option<super::ui_types::ChatSession>> {
        #[cfg(target_arch = "wasm32")]
        let _ = session;
        #[cfg(not(target_arch = "wasm32"))]
        if let Some(project) = crate::project::instance::project() {
            return super::ui_store::load_session_scoped_for_tenant_fallible(&super::ui_store::current_tenant_scope(), session, Some(&project.id));
        }
        Ok(None)
    }
    fn project_message(message: &super::ui_types::ChatMessage) -> Vec<serde_json::Value> {
        let role = match message.role {
            super::ui_types::ChatRole::User => super::session::Role::User,
            super::ui_types::ChatRole::Assistant => super::session::Role::Assistant,
            super::ui_types::ChatRole::Tool => super::session::Role::Tool,
            super::ui_types::ChatRole::System => super::session::Role::System,
            super::ui_types::ChatRole::Developer => super::session::Role::Developer,
        };
        super::processor::llm_messages::session_message_to_llm_messages(&super::session::Message {role, content:message.content.clone()})
    }
    async fn get_model(provider: &str, model: &str) -> Result<crate::provider::provider::Model, String> {
        crate::provider::provider::get_model(provider, model).await.map_err(|e| e.to_string())
    }
    async fn summarize_async(session: String, history: Vec<serde_json::Value>, model: String, reliability: crate::config::ReliabilityConfig, limit: crate::provider::provider::ModelLimit) -> Result<String, String> {
        super::compaction_summary::summarize_async(session, history, model, reliability, limit).await
    }
}
pub async fn process(input: ProcessInput, model: &crate::provider::provider::Model) -> Result<CompactionOutcome, super::session::Error> {
    session_engine::compaction::process::<Host>(input, model).await
}
pub async fn process_session(session: &str, parent: &str, auto: bool) -> Result<CompactionOutcome, super::session::Error> {
    session_engine::compaction::process_session::<Host>(session, parent, auto).await
}
#[cfg(test)]
use session_engine::compaction::now_ms;
#[cfg(test)]
#[path = "compaction_tests.rs"]
pub(crate) mod compaction_tests;
