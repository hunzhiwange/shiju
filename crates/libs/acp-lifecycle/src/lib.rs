//! Agent 生命周期快照与会话状态对齐。

pub mod lifecycle;

pub use lifecycle::{
    AgentLifecycleExit, AgentLifecycleSnapshot, apply_conversation,
    apply_lifecycle_snapshot_to_record, reconcile_agent_session_id, session_has_agent_messages,
};

#[cfg(test)]
mod lifecycle_tests;
