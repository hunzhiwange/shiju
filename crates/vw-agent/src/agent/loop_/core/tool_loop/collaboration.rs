//! 原协调能力绑定的测试入口；规则唯一归属agent-loop。
use super::*;
pub(super) async fn integrate(context: &ToolUseContext, history: &mut Vec<ChatMessage>, cancellation: Option<&CancellationToken>, budget_exhausted: bool) -> Result<bool> {
    agent_loop::core::tool_loop::integrate(&super::services(), context, history, cancellation, budget_exhausted).await
}
