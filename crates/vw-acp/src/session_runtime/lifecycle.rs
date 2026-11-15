//! 生命周期类型与状态对齐逻辑的兼容导出。

pub use acp_lifecycle::{
    AgentLifecycleExit, AgentLifecycleSnapshot, apply_conversation,
    apply_lifecycle_snapshot_to_record, reconcile_agent_session_id, session_has_agent_messages,
};
