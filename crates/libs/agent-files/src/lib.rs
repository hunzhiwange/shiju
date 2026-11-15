//! 文件操作模块
//!
//! 本模块提供文件系统操作的核心功能，包括文件读写、目录列表、文件状态追踪、
//! 文件搜索以及文件变更事件发布。
//!
//! # 子模块
//!
//! - [`ignore`]: 文件忽略规则处理（基于 .gitignore 等）
//! - [`ripgrep`]: 基于 ripgrep 的高性能文件搜索
//! - [`time`]: 文件时间戳相关操作
//! - [`watcher`]: 文件系统变更监控

mod fs_ops;
mod git_status;
mod pathing;
mod search;
mod text;
mod types;

#[cfg(test)]
mod text_tests;

#[cfg(test)]
#[path = "fs_ops_tests.rs"]
mod fs_ops_tests;
#[cfg(test)]
#[path = "git_status_tests.rs"]
mod git_status_tests;
#[cfg(test)]
#[path = "pathing_tests.rs"]
mod pathing_tests;
#[cfg(test)]
#[path = "search_tests.rs"]
mod search_tests;
#[cfg(test)]
#[path = "types_tests.rs"]
mod types_tests;

pub mod event;
pub mod ignore;
pub mod ripgrep;
pub mod time;
pub mod watcher;

#[cfg(test)]
mod tests;

pub use fs_ops::{init_preload, list, publish_edited, read};
pub use git_status::status_git;
pub use search::search;
pub use text::read_text;
pub use types::{Content, ContentType, Error, Info, Node, NodeType, SearchInput, Status};
