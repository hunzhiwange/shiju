//! 快照服务与宿主清理任务注册。

pub use agent_snapshot::{DiffStatus, Error, FileDiff, Patch, cleanup, diff, diff_full, patch, restore, revert, track};

#[cfg(not(target_arch = "wasm32"))]
mod native;
#[cfg(not(target_arch = "wasm32"))]
pub use native::init;
#[cfg(target_arch = "wasm32")]
pub use agent_snapshot::init;
