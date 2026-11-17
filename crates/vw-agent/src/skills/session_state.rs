//! 技能会话状态的兼容出口。
#[cfg(any(test, target_arch = "wasm32"))]
pub use agent_skills::session_state::*;
