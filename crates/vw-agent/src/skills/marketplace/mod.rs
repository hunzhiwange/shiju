//! 技能市场领域库的兼容入口。

#[cfg(any(test, target_arch = "wasm32"))]
pub use agent_skills::marketplace::*;
