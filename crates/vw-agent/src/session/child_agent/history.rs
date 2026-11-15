//! 调用边界的可见历史快照；不继承父角色身份或工具执行轨迹。
// Projection and persistence live in session-engine.

pub use session_port::child_agent::ForkTurns;

pub(super) use session_engine::child_agent::history::snapshot;

pub(super) use session_engine::child_agent::run::stored_snapshot;
