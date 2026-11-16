//! 应用层兼容入口。
//!
//! 本文件把 crate 内部模块重新组织到 `crate::app` 命名空间下，供历史
//! 调用点、测试和桌面侧集成以稳定路径访问 agent 运行时能力。

/// Agent 运行时模块重导出。
///
/// 该模块不承载业务逻辑，只提供兼容路径，避免调用方直接依赖 crate
/// 根部模块布局。按平台条件隐藏不可用能力，特别是 wasm 目标下的守护
/// 进程、网关和 PTY 相关模块。
pub(crate) mod agent {
    pub use crate::agent;
    pub use crate::approval;
    pub use crate::auth;
    pub use crate::bus;
    pub use crate::channels;
    pub use crate::command;
    pub use crate::config;
    pub(crate) use crate::cron;
    #[cfg(not(target_arch = "wasm32"))]
    pub use crate::daemon;
    #[cfg(not(target_arch = "wasm32"))]
    pub use crate::doctor;
    pub use crate::env;
    pub use crate::file;
    pub use crate::flag;

    #[cfg(not(target_arch = "wasm32"))]
    pub use crate::gateway;
    pub use crate::global;

    pub(crate) use crate::health;
    pub(crate) use crate::heartbeat;
    pub use crate::hooks;
    pub use crate::id;
    pub use crate::installation;
    pub(crate) use crate::integrations;

    pub use crate::memory;
    pub(crate) use crate::multimodal;
    pub use crate::observability;
    pub use crate::patch;
    pub use crate::permission;
    pub use crate::project;
    pub use crate::provider;
    pub use crate::providers;
    #[cfg(not(target_arch = "wasm32"))]
    pub use crate::pty;
    pub use crate::question;
    pub use crate::runtime;
    pub use crate::scheduler;
    pub use crate::security;

    pub use crate::session;
    pub use crate::shell;
    pub use crate::skill;

    pub use crate::skills;
    pub use crate::snapshot;
    pub use crate::storage;
    pub use crate::tools;
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) use crate::tunnel;

    pub use crate::util;
}

/// 启动与网关任务池的兼容导出。
pub(crate) use task_file_store::bootstrap_task as task;

#[cfg(test)]
#[path = "app_tests.rs"]
mod app_tests;
