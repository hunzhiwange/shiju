//! Tenant and session authorization at collaboration entrypoints.
use tool_context::{context::ToolUseContext, host::ContextHost};
use crate::{child_agent::coordinator::ChildAgentCoordinator, session_lifecycle::{self, LifecycleHost}};

pub async fn authorize_mail<L: LifecycleHost, C: ContextHost>(context: &ToolUseContext<C>, target: &str) -> anyhow::Result<()> {
    anyhow::ensure!(context.tenant_scope() == session_store::ui_store::current_tenant_scope(), "team mailbox tenant mismatch");
    session_lifecycle::get::<L>(context.session()).await?;
    session_lifecycle::get::<L>(target).await?;
    Ok(())
}

pub fn deliver_model_events<C: ContextHost>(coordinator: &ChildAgentCoordinator, context: &ToolUseContext<C>, messages: &mut Vec<serde_json::Value>) -> anyhow::Result<bool> {
    anyhow::ensure!(context.tenant_scope() == session_store::ui_store::current_tenant_scope(), "team delivery tenant mismatch");
    coordinator.deliver_model_events(context.session(), messages)
}

/// Authorize both persisted identities before enqueueing any team payload.
pub async fn send_team_message<L: LifecycleHost, C: ContextHost>(
    coordinator: &ChildAgentCoordinator, context: &ToolUseContext<C>, target: &str,
    turn: u64, kind: session_port::child_agent::TeamMessageKind, payload: String,
) -> anyhow::Result<session_port::child_agent::TeamMessage> {
    authorize_mail::<L, C>(context, target).await?;
    coordinator.enqueue_mail(context.session(), target, turn, kind, payload)
}

/// Reject a stale tenant before consuming mailbox contents.
pub async fn consume_team_messages<L: LifecycleHost, C: ContextHost>(
    coordinator: &ChildAgentCoordinator, context: &ToolUseContext<C>,
) -> anyhow::Result<Vec<session_port::child_agent::TeamMessage>> {
    authorize_mail::<L, C>(context, context.session()).await?;
    coordinator.take_mail(context.session())
}
