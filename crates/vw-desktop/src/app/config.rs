//! 桌面应用配置模块的分层导出与平台差异说明。
//!
//! 本模块集中处理配置读取、保存、归一化和跨平台回退边界。
//! 注释聚焦调用边界、返回值和错误传播方式，便于后续维护配置持久化流程。

#![cfg_attr(
    target_arch = "wasm32",
    doc = "On WASM targets, config saves have moved to `spawn_gateway_task`; remaining synchronous loads degrade through `run_gateway_call` by returning defaults."
)]

#[path = "config_agent.rs"]
mod agent;
#[cfg(not(target_arch = "wasm32"))]
#[path = "config_cli_bootstrap.rs"]
mod cli_bootstrap;
#[path = "config_cron_jobs.rs"]
mod cron_jobs;
#[path = "config_desktop.rs"]
mod desktop;
#[path = "config_gateway.rs"]
mod gateway;
#[path = "config_system_settings.rs"]
mod system_settings;

/// 重新导出 `agent::*`，作为本模块对外暴露的稳定入口。
///
/// 调用方通过该入口使用配置或组件能力，无需依赖内部文件布局。
pub use agent::*;
#[cfg(not(target_arch = "wasm32"))]
pub use cli_bootstrap::*;
pub use cron_jobs::*;
/// 重新导出 `desktop::*`，作为本模块对外暴露的稳定入口。
///
/// 调用方通过该入口使用配置或组件能力，无需依赖内部文件布局。
pub use desktop::*;
/// 重新导出 `gateway::{gateway_client, gateway_client_endpoint, load_tools_list_via_gateway, server_config_unreachable_error, spawn_gateway_task}`，作为本模块对外暴露的稳定入口。
///
/// 调用方通过该入口使用配置或组件能力，无需依赖内部文件布局。
pub use gateway::{
    gateway_client, gateway_client_endpoint, load_tools_list_via_gateway,
    server_config_unreachable_error, spawn_gateway_task,
};
/// 重新导出 `system_settings::*`，作为本模块对外暴露的稳定入口。
///
/// 调用方通过该入口使用配置或组件能力，无需依赖内部文件布局。
pub use system_settings::*;

#[cfg(test)]
#[path = "config_tests.rs"]
mod config_tests;

#[cfg(test)]
#[allow(unused_imports)] // 仅自定义窗口测试目标调用，常规 libtest 不启动窗口。
pub(crate) use system_settings::install_window_test_bootstrap;
