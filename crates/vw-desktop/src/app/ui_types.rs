//! 应用界面共享类型定义。
//!
//! 该模块集中放置跨视图复用的轻量 UI 数据结构，避免各个视图重复定义同一类展示状态。

use crate::app::message;

/// 公开的 FocusArea 枚举，描述该模块支持的一组离散状态或事件。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FocusArea {
    None,
    Preview,
    Terminal,
}

/// Todo 面板在会话界面中的承载位置。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TodoPanelPlacement {
    InputBottom,
    ChatTopRight,
}

impl TodoPanelPlacement {
    pub(crate) fn label(self) -> &'static str {
        match self {
            TodoPanelPlacement::InputBottom => "输入底部",
            TodoPanelPlacement::ChatTopRight => "右上角",
        }
    }
}

/// 公开的 SettingsTab 枚举，描述该模块支持的一组离散状态或事件。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingsTab {
    Settings,
    SettingsJson,
    Files,
    Projects,
    Sessions,
}

impl SettingsTab {
    /// 公开的 all 函数。
    ///
    /// 参数由调用方提供，返回值表达该步骤的计算结果；遇到不可恢复的外部状态时通过现有返回类型向上层传播错误或空结果。
    #[allow(dead_code)]
    pub(crate) fn all() -> [SettingsTab; 3] {
        [SettingsTab::Settings, SettingsTab::Files, SettingsTab::Projects]
    }
}

impl std::fmt::Display for SettingsTab {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SettingsTab::Settings => write!(f, "设置"),
            SettingsTab::SettingsJson => write!(f, "JSON配置"),
            SettingsTab::Files => write!(f, "文件管理器"),
            SettingsTab::Projects => write!(f, "项目管理器"),
            SettingsTab::Sessions => write!(f, "历史会话"),
        }
    }
}

/// 公开的 Message 枚举，描述该模块支持的一组离散状态或事件。
#[derive(Debug, Clone)]
pub enum Message {
    #[cfg(not(target_arch = "wasm32"))]
    StartupCliServiceBootstrapped(Result<(), String>),
    StartupAppConfigLoaded(Result<serde_json::Value, String>),
    StartupSystemSettingsLoaded(Result<vw_config_types::ui::AppSystemSettingsConfig, String>),
    StartupAgentConfigLoaded(Result<Box<vw_config_types::config::Config>, String>),
    StartupBrowserConfigLoaded(Result<vw_config_types::tools::BrowserConfig, String>),
    BootstrapAppConfig(Result<serde_json::Value, String>),
    BootstrapSystemSettings(Result<vw_config_types::ui::AppSystemSettingsConfig, String>),
    BootstrapBrowserConfig(Result<vw_config_types::tools::BrowserConfig, String>),
    BootstrapAcpAgentsLoaded(Result<Vec<String>, String>),
    BootstrapArchivedSessions(Result<std::collections::HashSet<String>, String>),
    ExternalAppsLoaded(Result<vw_gateway_client::ExternalAppsStateDto, String>),
    SessionPreviewsLoaded(Result<Vec<crate::app::models::ChatSessionMeta>, String>),
    ProjectChatPreferencesLoaded(String, Option<(String, bool, Option<String>)>),
    GatewayHealthTick,
    GatewayHealthChecked(Vec<(String, bool)>),
    #[cfg(not(target_arch = "wasm32"))]
    #[allow(private_interfaces)]
    Account(message::AccountMessage),
    #[cfg(not(target_arch = "wasm32"))]
    #[allow(private_interfaces)]
    ShijuApps(message::ShijuAppsMessage),
    #[cfg(not(target_arch = "wasm32"))]
    #[allow(private_interfaces)]
    SkillHub(message::SkillHubMessage),
    Project(message::ProjectMessage),
    View(message::ViewMessage),
    Search(message::SearchMessage),
    Settings(message::SettingsMessage),
    Terminal(message::TerminalMessage),
    Git(message::GitMessage),
    Chat(message::ChatMessage),
    Preview(message::PreviewMessage),
    Editor(message::EditorMessage),
    Knowledge(message::KnowledgeToolMessage),
    WorkflowTool(crate::apps::workflow::WorkflowMessage),
    WorkflowManager(message::WorkflowManagerMessage),
    Notification(message::NotificationMessage),
    TaskBoard(message::TaskBoardMessage),
    #[cfg(target_os = "macos")]
    NativeMenu(crate::app::native_menu::NativeMenuAction),
    #[cfg(not(target_arch = "wasm32"))]
    PreviewLspTick,
    CopyShortcut,
    CopyCode(String),
    CopyFile(String),
    CopyDone,
    CopyFeedbackExpired(u64),
    CloseError,
    None,
    Batch(Vec<Message>),
}

/// 公开的 DiffTheme 枚举，描述该模块支持的一组离散状态或事件。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiffTheme {
    GitHub,
    Monokai,
}

/// 公开的 Screen 枚举，描述该模块支持的一组离散状态或事件。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Screen {
    Home,
    Project,
    Preview,
    Apps,
    Usage,
    Knowledge,
    WorkflowTool,
    WorkflowManager,
    TaskBoard,
    #[cfg(not(target_arch = "wasm32"))]
    ShijuApps,
    #[cfg(not(target_arch = "wasm32"))]
    ShijuSkills,
    #[cfg(not(target_arch = "wasm32"))]
    ExpertHub,
}

#[cfg(test)]
#[path = "ui_types_tests.rs"]
mod ui_types_tests;
