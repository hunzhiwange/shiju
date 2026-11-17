//! Host authorization adapter for the single session-engine coordinator.
use anyhow::Result;
use std::ops::Deref;
use std::time::Duration;
use tokio_util::sync::CancellationToken;
use crate::tools::ToolUseContext;
#[cfg(all(not(test), not(target_arch = "wasm32")))]
pub(crate) use session_engine::child_agent::coordinator::ChildAgentTaskSnapshot;
#[cfg(any(test, target_arch = "wasm32"))]
pub(crate) use session_engine::child_agent::coordinator::{
    ChildAgentTaskState, ChildAgentTaskSnapshot, WaitCondition, ChildAgentWaitOutcome,
};
#[path = "mailbox.rs"]
pub(crate) mod mailbox;
#[path = "delivery.rs"]
mod delivery;

#[derive(Debug, Clone)]
pub(crate) struct ChildAgentCoordinator(session_engine::child_agent::coordinator::ChildAgentCoordinator);

impl Deref for ChildAgentCoordinator {
    type Target = session_engine::child_agent::coordinator::ChildAgentCoordinator;
    fn deref(&self) -> &Self::Target { &self.0 }
}

pub(crate) fn global_child_agent_coordinator() -> ChildAgentCoordinator {
    ChildAgentCoordinator(session_engine::child_agent::coordinator::global_child_agent_coordinator())
}

impl ChildAgentCoordinator {
    pub(crate) fn new(max_terminal_per_parent: usize) -> Self {
        Self(session_engine::child_agent::coordinator::ChildAgentCoordinator::new(max_terminal_per_parent))
    }

    async fn authorize_mail(&self, context: &ToolUseContext, target: &str) -> Result<()> {
        session_engine::child_agent_access::authorize_mail::<crate::session::session_io::NativeSessionIo, _>(context, target).await
    }

    pub(crate) async fn team_snapshots(&self, context: &ToolUseContext) -> Result<Vec<ChildAgentTaskSnapshot>> {
        self.authorize_mail(context, context.session()).await?;
        self.0.team_snapshots(context.session())
    }

    pub(crate) async fn send_team_message(&self, context: &ToolUseContext, target: &str, turn: u64,
        kind: mailbox::TeamMessageKind, payload: String) -> Result<mailbox::TeamMessage> {
        session_engine::child_agent_access::send_team_message::<crate::session::session_io::NativeSessionIo, _>(
            &self.0, context, target, turn, kind, payload).await
    }

    pub(crate) async fn wait_team_activity(&self, context: &ToolUseContext, timeout: Duration,
        cancellation: &CancellationToken) -> Result<mailbox::TeamActivity> {
        self.authorize_mail(context, context.session()).await?;
        self.0.wait_mail(context.session(), timeout, cancellation).await
    }

    pub(crate) async fn consume_team_messages(&self, context: &ToolUseContext) -> Result<Vec<mailbox::TeamMessage>> {
        session_engine::child_agent_access::consume_team_messages::<crate::session::session_io::NativeSessionIo, _>(&self.0, context).await
    }
}

#[cfg(test)]
#[path = "continuation_tests.rs"]
mod continuation_tests;
