//! 只读 Git 发现的兼容入口。

#[cfg(any(test, target_arch = "wasm32"))]
pub use git_discovery::git_repository::{DiscoveredRepository, current_branch, discover};
