//! Concrete bindings for the engine-owned request and turn state.
pub type Request = session_engine::runner_types::Request<crate::approval::ApprovalManager, crate::acp::mounts::SessionSkillSources>;
pub(crate) use session_engine::runner_types::{ResolvedRequestModel, ToolSessionState};
pub use session_port::{
    CompactionPhase, ModelRecovery, RecoveryPhase, ToolCallStatus, ToolEvent, ToolEventKind,
};

pub use session_engine::processor_events::StreamEvent;

// Turn state has a single owner in session-engine.
#[cfg(test)]
#[path = "../../../../libs/session-engine/tests/host_suites/types_tests.rs"]
mod types_tests;
