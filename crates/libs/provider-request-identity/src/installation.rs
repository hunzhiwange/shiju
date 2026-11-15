//! 安装通道与版本信息辅助函数。
//!
//! 当前主要用于拼接访问远端服务时携带的 User-Agent。

use std::env;

use crate::flag;

/// 返回当前运行版本。
///
/// 若环境变量缺失，则在本地开发场景下回退为 `local`。
fn version() -> String {
    env::var("VIBEWINDOW_VERSION").unwrap_or_else(|_| "local".to_string())
}

/// 返回当前发布通道。
///
/// 若环境变量缺失，则在本地开发场景下回退为 `local`。
fn channel() -> String {
    env::var("VIBEWINDOW_CHANNEL").unwrap_or_else(|_| "local".to_string())
}

/// 生成发往远端服务的 User-Agent。
///
/// 格式为 `vibewindow/<channel>/<version>/<client>`。
pub fn user_agent() -> String {
    format!("vibewindow/{}/{}/{}", channel(), version(), flag::vibewindow_client())
}

#[cfg(test)]
#[path = "installation_tests.rs"]
mod installation_tests;
