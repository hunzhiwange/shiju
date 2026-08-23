//! 本模块集中实现 `mod` 在底部面板中的职责。


//! 默认使用的统一提及弹窗。 `mentions_v2` 特性标志暂时作为回滚路径：禁用该功能将恢复旧版分提及和文件搜索弹窗。
//! The unified mention popup used by default in the TUI.
//!
//! The `mentions_v2` feature flag remains temporarily as a rollback path: disabling it restores
//! the legacy split mention and file-search popups.

mod candidate;
mod filter;
mod footer;
mod popup;
mod render;
mod search_catalog;
mod search_mode;

pub(crate) use candidate::Selection as MentionV2Selection;
pub(crate) use popup::Popup as MentionV2Popup;
pub(crate) use search_catalog::build_search_catalog;
