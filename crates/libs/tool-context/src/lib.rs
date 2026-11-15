//! 工具执行上下文的共享状态与执行契约。

mod decision;
mod executor;
mod pending_images;

pub use decision::PermissionDecision;
pub use executor::{ExecutedToolCall, ToolCallError, ToolResultHistoryEntry};
pub use pending_images::{MAX_PENDING_IMAGE_RELOADS, PendingImageReloadQueue};
