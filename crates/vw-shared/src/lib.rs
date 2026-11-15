#![allow(
	// 测试目录沿用 `tests/tests.rs` 结构，暂不将 module_inception 作为阻塞项
	clippy::module_inception
)]

//! 共享职责库的历史公开路径兼容门面。
//!
//! 所有类型与实现均由职责库重导出；仓库内部消费者直接依赖对应职责库。
//!
//! # 主要模块
//!
//! - `auth`：提供商鉴权信息及持久化读写
//! - `message`：消息、片段与工具调用状态协议
//! - `provider`：模型提供商与模型元数据定义
//! - `session`：会话元数据、UI 会话结构与存储路径
//! - `task`：任务面板与任务存储相关共享类型
//! - `todo`：待办事项结构与兼容性反序列化工具
//! - `time` / `util` / `json`：通用辅助工具

pub mod auth;
pub use build_identity;
pub use ordered_id as id;
pub mod json;
pub use local_app_auth as local_apps;
pub mod message;
pub use pasted_text_cache as pasted_text;
pub mod patch;
pub mod permission;
pub mod project;
pub mod provider;
pub mod question;
#[cfg(not(target_arch = "wasm32"))]
pub use runtime_log;
pub mod session;
pub mod shell;
pub mod snapshot;
pub mod task;
pub use runtime_time as time;
pub mod todo;
pub mod update;
pub mod util;
