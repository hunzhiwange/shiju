//! 应用程序状态模块
//!
//! 本模块定义了 VibeWindow 应用程序的核心状态结构和相关数据类型。
//! 包含应用程序运行时所需的所有状态信息，包括：
//! - 会话管理和运行时状态
//! - Git 差异和版本控制相关状态
//! - 文件树和文件搜索状态
//! - 提供者和模型配置状态
//! - 各种设置面板的状态
//! - 任务看板状态
//!
//! # 主要结构
//!
//! - [`App`]：应用程序的主状态结构，包含所有子系统和 UI 状态
//! - [`SessionRuntimeState`]：单个会话的运行时状态
//! - [`ProviderSettingsState`]：提供者设置面板状态
//! - [`ModelSettingsState`]：模型设置面板状态

use iced::widget::Id;
use iced::Point;
use std::collections::HashMap;

#[cfg(not(target_arch = "wasm32"))]
use crate::app::lsp::LspEvent;
#[cfg(not(target_arch = "wasm32"))]
use crate::app::lsp::LspServiceManager;
use crate::app::terminal::TerminalState;
use crate::app::FocusArea;

mod account;
mod agent;
mod app_state;
mod knowledge;
mod pet;
mod presentation;
mod runtime_tools;
mod settings;
mod workspace;


pub(crate) use account::AccountState;
pub(crate) use agent::*;
#[cfg(not(target_arch = "wasm32"))]
pub(crate) use app_state::NativeStartupState;
pub use app_state::{
    App, ChatPanelMode, CookieConfig, QuickMessageGitCommitAction, RecentProjectMeta,
    TaskBoardViewMode, TrajectoryDetailsTab, WebBookmark,
};
pub(crate) use app_state::{
    QuickMessageHandoffState, ToastKind, WorkBrowserState,
    default_recent_project_session_auto_refresh,
    default_recent_project_session_refresh_interval_seconds,
};
pub(crate) use knowledge::*;
pub(crate) use pet::*;
pub use presentation::AppTab;
pub(crate) use presentation::{
    LayoutMode, LayoutPreference, MobileLayoutState, ExternalOpenApp, ModelPopoverHover, RuntimePlatform, TopBarGatewayTab, WORK_SIDEBAR_RAIL_WIDTH,
    WorkResourceMode, WorkSidebarFilterScope, WorkSidebarFilterStatus, WorkSidebarFilterTime,
    WorkSidebarTab, WorkspaceMode,
};
pub(crate) use runtime_tools::*;
pub(crate) use settings::*;
pub(crate) use workspace::*;

mod plugin_install;
pub use plugin_install::{PluginInstallForm, PluginInstallPhase};

pub use desktop_provider_apps::AgentAppsContext;

pub(crate) mod agent_apps;
#[cfg(not(target_arch = "wasm32"))]
pub(crate) mod agent_apps_auth;
pub(crate) use agent_apps::AppsCatalogState;

pub(crate) mod app_providers;

pub(crate) mod project_profiles;

#[cfg(test)]
use config_tools::HttpRequestConfig;
#[cfg(test)]
use crate::app::message;
#[cfg(test)]
use crate::app::models;
#[cfg(test)]
use crate::app::models::ChatMessage;
#[cfg(test)]
use crate::app::models::ChatSessionStep;
#[cfg(test)]
use serde_json::Value;
#[cfg(test)]
use std::collections::HashSet;
#[cfg(test)]
use std::path::PathBuf;
#[cfg(test)]
pub(crate) use account::AccountOperation;
#[cfg(test)]
pub(crate) use account::AccountPasswordEditorState;
#[cfg(test)]
pub(crate) use account::AccountProfileEditorState;
#[cfg(test)]
pub(crate) use account::MonthlyQuotaState;
#[cfg(test)]
pub(crate) use app_state::Notification;
#[cfg(test)]
pub(crate) use app_state::Toast;
#[cfg(test)]
pub(crate) use presentation::ConventionalCommitType;
#[cfg(test)]
pub(crate) use presentation::WORK_SIDEBAR_INITIAL_VISIBLE_ITEMS;
#[cfg(test)]
pub(crate) use presentation::WORK_SIDEBAR_LOAD_MORE_STEP;
#[cfg(test)]
pub(crate) use presentation::WORK_SIDEBAR_SPACE_TASKS_INITIAL_VISIBLE;
#[cfg(test)]
pub(crate) use presentation::WORK_SIDEBAR_SPACE_TASKS_LOAD_MORE_STEP;
