//! 应用程序初始化模块
//!
//! 本模块负责应用程序启动时的初始化工作，包括：
//! - 加载和解析各种配置文件
//! - 初始化应用程序状态
//! - 检测外部应用程序的可用性
//! - 设置 UI 组件的初始状态
//!
//! 该模块是应用程序启动流程的核心，所有状态初始化都在此完成。

use std::collections::HashMap;

pub(super) use super::config;
pub(super) use super::state;
pub(super) use super::{
    App, FocusArea, Message,
    TerminalState, load_recent_projects,
    load_recent_projects_meta,
};
#[cfg(not(target_arch = "wasm32"))]
pub(super) use crate::app::lsp::LspServiceManager;

mod external_apps;
mod new;
mod settings_builders;
pub(in crate::app) mod startup;

pub(super) use desktop_settings_agents::acp::sort_acp_agents;
pub(super) use desktop_workspace::basic::display_name_for_path;
pub(super) use desktop_apps_ui::bookmark::load_web_bookmarks;

mod preferences;
#[cfg(not(target_arch = "wasm32"))]
mod startup_snapshot;
mod system_settings;
pub(in crate::app) mod dispatch;
