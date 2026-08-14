#![allow(
	// 测试目录沿用 `tests/tests.rs` 结构，暂不将 module_inception 作为阻塞项
	clippy::module_inception
)]

//! vw-shared 公共共享类型库。
//!
//! 本 crate 汇总多个子系统共享的数据结构、序列化协议和少量跨平台工具，
//! 供 vw-agent、vw-desktop、vw-cli 以及其他 crate 复用。
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
pub mod build_identity;
pub mod id;
pub mod json;
pub mod message;
pub mod pasted_text;
pub mod patch;
pub mod permission;
pub mod project;
pub mod provider;
pub mod question;
#[cfg(not(target_arch = "wasm32"))]
pub mod runtime_log;
pub mod session;
pub mod shell;
pub mod snapshot;
pub mod task;
pub mod time;
pub mod todo;
pub mod update;
pub mod util;

#[cfg(test)]
#[path = "build_identity_tests.rs"]
mod build_identity_tests;
