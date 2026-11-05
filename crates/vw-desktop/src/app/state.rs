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

use iced::widget::{Id, text_editor};
use iced::{Point, Theme};
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

use crate::app::components::color_picker::ColorFormat;
#[cfg(not(target_arch = "wasm32"))]
use crate::app::lsp::LspEvent;
#[cfg(not(target_arch = "wasm32"))]
use crate::app::lsp::LspServiceManager;
use crate::app::message;
use crate::app::models::{self, ChatMessage, ChatSessionStep};
use crate::app::terminal::TerminalState;
use crate::app::{DiffTheme, FocusArea, PreviewTab, Screen, SettingsTab, TodoPanelPlacement};
#[cfg(not(target_arch = "wasm32"))]
use crate::app::{LspHoverPending, LspProgress};
use vw_config_types::tools::HttpRequestConfig;

#[cfg(not(target_arch = "wasm32"))]
mod account;
mod agent;
mod app_state;
mod knowledge;
mod pet;
mod presentation;
mod runtime_tools;
mod settings;
mod workspace;

#[cfg(test)]
mod runtime_tools_tests;

#[cfg(not(target_arch = "wasm32"))]
pub(crate) use account::{
    AccountOperation, AccountPasswordEditorState, AccountProfileEditorState, AccountState,
};
pub(crate) use agent::*;
#[cfg(not(target_arch = "wasm32"))]
pub(crate) use app_state::NativeStartupState;
pub use app_state::{
    App, ChatPanelMode, CookieConfig, QuickMessageGitCommitAction, RecentProjectMeta,
    TaskBoardViewMode, TrajectoryDetailsTab, WebBookmark,
};
pub(crate) use app_state::{
    Notification, QuickMessageHandoffState, Toast, ToastKind, WorkBrowserState,
    default_recent_project_session_auto_refresh,
    default_recent_project_session_refresh_interval_seconds,
};
pub(crate) use knowledge::*;
pub(crate) use pet::*;
pub use presentation::AppTab;
pub(crate) use presentation::{
    ConventionalCommitType, ExternalOpenApp, ModelPopoverHover, RuntimePlatform, TopBarGatewayTab,
    WORK_SIDEBAR_INITIAL_VISIBLE_ITEMS, WORK_SIDEBAR_LOAD_MORE_STEP,
    WORK_SIDEBAR_SPACE_TASKS_INITIAL_VISIBLE, WORK_SIDEBAR_SPACE_TASKS_LOAD_MORE_STEP,
    WorkResourceMode, WorkSidebarFilterScope, WorkSidebarFilterStatus, WorkSidebarFilterTime,
    WorkSidebarTab, WorkspaceMode,
};
pub(crate) use runtime_tools::*;
pub(crate) use settings::*;
pub(crate) use workspace::*;

mod plugin_install;
pub use plugin_install::{PluginInstallForm, PluginInstallPhase};

/// 图标列表所属上下文；不保留认证凭证或读取本机业务路径。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentAppsContext {
    pub(crate) gateway: String,
    pub(crate) tenant: vw_gateway_client::vw_api_types::id::TenantScope,
    pub(crate) project: Option<String>,
}

/// 当前弹窗的输入与请求状态；不缓存业务写入结果。
#[derive(Debug, Clone)]
pub(crate) struct IconRenameState {
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) saving: bool,
    pub(crate) error: Option<String>,
    pub(crate) context: AgentAppsContext,
    pub(crate) generation: u64,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct AgentAppsState {
    pub(crate) rename: Option<IconRenameState>,
    pub(crate) rename_generation: u64,
    pub(crate) icons: Vec<vw_gateway_client::vw_api_types::agent_apps::CustomIconEntry>,
    pub(crate) loading: bool,
    pub(crate) error: Option<String>,
    context: Option<AgentAppsContext>,
    generation: u64,
}

impl AgentAppsState {
    pub(crate) fn belongs_to(&self, context: &AgentAppsContext) -> bool {
        self.context.as_ref() == Some(context)
    }

    pub(crate) fn begin(&mut self, context: AgentAppsContext) -> u64 {
        if self.context.as_ref() != Some(&context) {
            self.rename = None;
            self.icons.clear();
        }
        self.context = Some(context);
        self.generation = self.generation.wrapping_add(1);
        self.loading = true;
        self.error = None;
        self.generation
    }

    pub(crate) fn finish(
        &mut self,
        current: &AgentAppsContext,
        requested: &AgentAppsContext,
        generation: u64,
        result: Result<Vec<vw_gateway_client::vw_api_types::agent_apps::CustomIconEntry>, String>,
    ) -> bool {
        if current != requested
            || self.context.as_ref() != Some(requested)
            || self.generation != generation
        {
            return false;
        }
        self.loading = false;
        match result {
            Ok(icons) => {
                self.icons = icons;
                self.error = None;
            }
            Err(error) => {
                self.icons.clear();
                self.error = Some(error);
            }
        }
        true
    }
}
