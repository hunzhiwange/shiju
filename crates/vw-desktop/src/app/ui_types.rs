//! 应用界面共享类型定义。
//!
//! 该模块集中放置跨视图复用的轻量 UI 数据结构，避免各个视图重复定义同一类展示状态。

use crate::app::message;

pub use desktop_navigation::ui_types::{FocusArea, TodoPanelPlacement, ClipboardShortcutKind, SettingsTab, Screen};

/// 公开的 Message 枚举，描述该模块支持的一组离散状态或事件。
#[derive(Debug, Clone)]
pub enum Message {
    #[cfg(not(target_arch = "wasm32"))]
    Voice(message::voice::Event),
    #[cfg(not(target_arch = "wasm32"))]
    StartupCliServiceBootstrapped(Result<(), String>),
    #[cfg(not(target_arch = "wasm32"))]
    StartupGatewaySnapshotLoaded(u64, Box<crate::app::impls::StartupGatewaySnapshot>),
    #[cfg(not(target_arch = "wasm32"))]
    StartupSessionsLoaded(u64, Box<crate::app::message::ProjectMessage>),
    StartupAppConfigLoaded(Result<serde_json::Value, String>),
    StartupSystemSettingsLoaded(Result<config_ui::AppSystemSettingsConfig, String>),
    StartupAgentConfigLoaded(Result<Box<config_aggregate::Config>, String>),
    StartupBrowserConfigLoaded(Result<config_tools::BrowserConfig, String>),
    BootstrapAppConfig(Result<serde_json::Value, String>),
    BootstrapSystemSettings(Result<config_ui::AppSystemSettingsConfig, String>),
    BootstrapBrowserConfig(Result<config_tools::BrowserConfig, String>),
    BootstrapAcpAgentsLoaded(Result<Vec<String>, String>),
    BootstrapArchivedSessions(Result<std::collections::HashSet<String>, String>),
    ExternalAppsLoaded(Result<gateway_desktop_settings::ExternalAppsStateDto, String>),
    SessionPreviewsLoaded(Result<Vec<crate::app::models::ChatSessionMeta>, String>),
    ProjectChatPreferencesLoaded(
        crate::app::config::ProjectPreferencesScope,
        u64,
        Result<Option<crate::app::config::ChatModelPreferences>, String>,
    ),
    ProjectChatPreferencesSaved(crate::app::config::ProjectPreferencesScope, Result<(), String>),
    SessionDetailsHovered {
        session_id: String,
        project_id: String,
        directory: String,
    },
    SessionDetailsLoaded(String, crate::app::session::hover_details::SessionHoverDetails),
    SessionPinsLoaded(u64, Result<std::collections::HashSet<(String, String)>, String>),
    DesktopPreferencesSaved(
        (String, vw_api_transport::TenantScope),
        Result<(), String>,
    ),
    SystemSettingsSaved(
        crate::app::config::PreferencesScope,
        crate::app::config::SystemSettingsSection,
        Result<(), String>,
    ),
    GatewayHealthTick,
    GatewayHealthChecked(Vec<(String, bool)>),
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
    CutShortcut,
    PasteShortcut,
    ClipboardShortcutRouted {
        kind: ClipboardShortcutKind,
        input_focused: bool,
    },
    CopyCode(String),
    ChatMarkdownSelectionChanged {
        session_id: String,
        message_idx: usize,
        text_idx: usize,
        text: Option<String>,
    },
    CopyChatMessage {
        session_id: String,
        message_idx: usize,
    },
    CopyFile(String),
    ClipboardCopyFinished {
        hash: u64,
        result: Result<(), String>,
    },
    CopyDone,
    CopyFeedbackExpired(u64),
    CloseError,
    None,
    Batch(Vec<Message>),
}

pub(crate) use desktop_chat_view::chat_ui::ChatMarkdownSelection;

pub use desktop_git_ui::DiffTheme;

#[cfg(test)] #[path="ui_types_tests.rs"] mod ui_types_tests;
