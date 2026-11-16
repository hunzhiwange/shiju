mod app_aux;

pub use app_aux::{CookieConfig, RecentProjectMeta, WebBookmark};
pub(crate) use app_aux::{
    Notification, Toast, ToastKind, WorkBrowserState, default_recent_project_session_auto_refresh,
    default_recent_project_session_refresh_interval_seconds,
};

use super::*;

pub use desktop_task_board::state::TaskBoardViewMode;

pub use desktop_chat_view::view_snapshot::ChatPanelMode;

pub use desktop_chat_view::trajectory::model::TrajectoryDetailsTab;

pub use desktop_session::message::chat::git_handoff::QuickMessageGitCommitAction;
pub(crate) use desktop_session::message::chat::git_handoff::QuickMessageHandoffState;

pub(crate) type UsageStepRawKey = SessionStepRawKey;

#[cfg(not(target_arch = "wasm32"))]
pub(crate) use desktop_navigation::components::startup_transition::NativeStartupState;

#[allow(dead_code)]
pub struct App {
    pub(crate) git: desktop_git_ui::state::State,
    pub(crate) usage_center: crate::app::views::usage::center::state::State,
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) voice: crate::app::message::voice::State,
    pub(crate) session_ui_projection: crate::app::session::ui_projection::SessionUiProjection,
    /// 原生端本地服务启动状态，用于门控主界面。
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) startup_state: NativeStartupState,
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) startup_started_at: std::time::Instant,
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) startup_configuration: crate::app::impls::StartupConfiguration,
    pub(crate) navigation_workspace: desktop_navigation::panel::work_workspace::State,
    pub(crate) navigation_mode: desktop_navigation::panel::workspace_mode::State,
    pub(crate) layout_drag: desktop_navigation::view_layout::drag::State,
    pub(crate) popover_visibility: desktop_chat_input::popovers::Visibility,
    pub(crate) navigation_panels: desktop_navigation::panel::settings::State,
    pub(crate) usage_panel: desktop_usage::panel::State,
    pub(crate) update_panel: desktop_update::panel::PanelState,
    pub(crate) navigation_tabs: desktop_navigation::panel::tabs::State,
    pub(crate) companion_navigation: desktop_navigation::panel::companion::State,
    pub(crate) web_navigation: desktop_apps_ui::web::State,
    pub(crate) external_navigation: desktop_navigation::panel::external::State,
    pub(crate) navigation_sidebar: desktop_navigation::panel::work_sidebar::State,
    /// 工作模式停靠浏览器的输入、错误与焦点状态
    pub(crate) work_browser: WorkBrowserState,
    /// 工作模式停靠浏览器的受控子进程
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) work_browser_controller: crate::app::webview_runtime::WorkBrowserController,
    pub(crate) workspace_state: desktop_workspace::project_state::State<crate::app::Message>,

    /// 项目输入与查找领域状态
    pub(crate) project_input: desktop_workspace::basic::InputState,
    pub(crate) project_find: desktop_workspace::project_message::find::FindState,
    pub(crate) session_state: desktop_session::session::state::State<desktop_chat_view::stream_render::StreamRenderState, crate::app::session::view_cache::SessionViewSnapshot>,

    pub(crate) acp_settings: AcpSettingsState,
    pub(crate) mcp_settings: McpSettingsState,

    pub(crate) chat_view: desktop_chat_view::view_snapshot::SessionViewSnapshot<
        ActiveSessionViewState,
        ChatPanelMode,
    >,

    pub(crate) chat_interaction: desktop_chat_view::interaction::State,

    pub(crate) todo_state: desktop_chat_view::todo::State,

    /// 窗口尺寸（宽，高）
    pub(crate) window_size: (f32, f32),
    /// 当前会话的布局选择，独立于工作区模式与桌面面板尺寸。
    pub(crate) mobile_layout: MobileLayoutState,
    /// 全屏切换后的布局稳定期是否进行中
    pub(crate) fullscreen_layout_settling: bool,
    pub(crate) startup_resize_checked: bool,
    /// 窗口位置（X，Y）
    pub(crate) window_position: (f32, f32),
    /// 主窗口 ID
    pub(crate) main_window_id: Option<iced::window::Id>,
    /// 主动释放后尚未收到 Closed 的窗口；迟到事件不能终止后台运行。
    pub(crate) released_main_windows: Vec<iced::window::Id>,
    /// 独立小宠物窗口 ID
    pub(crate) task_pet_window_id: Option<iced::window::Id>,
    /// 主窗口是否已进入一次性关机流程
    pub(crate) shutdown_requested: bool,
    pub(crate) desktop_preferences_persistence: crate::app::config::DesktopPreferencesPersistence,
    pub(crate) general_settings: desktop_settings_appearance::general_state::State,
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) web_install: crate::app::web_install::State,
    /// 应用升级来源、检测快照与安装器交接状态
    pub(crate) desktop_update: crate::app::desktop_update::DesktopUpdate,

    /// 外部应用检测结果对应的运行时平台
    pub(crate) open_external_platform: Option<RuntimePlatform>,
    pub(crate) file_manager: desktop_workspace::file_manager::FileManagerState,
    /// 通用状态动画帧
    pub(crate) status_animation_frame: usize,

    /// 模型选择弹出框中的悬停项
    pub(crate) model_popover_hover: Option<ModelPopoverHover>,

    /// 搜索领域状态
    pub(crate) search: desktop_search::State,

    pub(crate) project_editor: desktop_workspace::project_message::edit::Editor,
    /// 终端状态（主终端）
    pub terminal: TerminalState,
    /// 终端偏好设置写入队列；切换项目时仍持续保存。
    pub(crate) terminal_config_persistence:
        crate::app::message::terminal::config::ConfigPersistence,
    /// 按项目分组的终端状态映射
    pub(crate) terminals_by_project: HashMap<String, TerminalState>,
    pub(crate) appearance: desktop_settings_appearance::theme_state::State,
    pub(crate) input_feedback: desktop_i18n::input::State,

    pub(crate) launch_at_login: bool,
    pub(crate) launch_at_login_busy: bool,
    pub(crate) close_to_tray: bool,

    pub(crate) preview_state: desktop_editor::preview::state::State,

    pub(crate) clipboard_state: desktop_navigation::clipboard::Feedback,

    pub(crate) pet_state: desktop_task_board::pet_state::State,

    pub(crate) task_pet_size_pending: Option<crate::app::config::PreferencesScope>,

    /// 文件搜索滚动容器 ID
    pub(crate) file_search_scroll_id: Id,
    /// Slash 面板滚动容器 ID
    pub(crate) slash_palette_scroll_id: Id,

    /// 独立的原生归档聊天元数据与查询状态。
    pub(crate) archived_chats: ArchivedChatsState,
    pub(crate) worktree_settings: WorktreeSettingsState,

    /// 提供者设置状态
    pub(crate) provider_settings: ProviderSettingsState,
    /// 模型设置状态
    pub(crate) model_settings: ModelSettingsState,
    /// 嵌入路由设置状态
    pub(crate) embedding_routes_settings: EmbeddingRoutesSettingsState,
    /// 模型路由设置状态
    pub(crate) model_routes_settings: ModelRoutesSettingsState,
    /// 查询分类设置状态
    pub(crate) query_classification_settings: QueryClassificationSettingsState,
    /// 心跳设置状态
    pub(crate) heartbeat_settings: HeartbeatSettingsState,
    /// Cron 设置状态
    pub(crate) cron_settings: CronSettingsState,
    /// 调度器设置状态
    pub(crate) scheduler_settings: SchedulerSettingsState,
    /// Hooks 设置状态
    pub(crate) hooks_settings: HooksSettingsState,
    /// 运行时设置状态
    pub(crate) runtime_settings: RuntimeSettingsState,
    /// 技能设置状态
    pub(crate) skills_settings: SkillsSettingsState,
    /// Web 搜索设置状态
    pub(crate) web_search_settings: WebSearchSettingsState,
    /// 浏览器设置状态
    pub(crate) browser_settings: BrowserSettingsState,
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) computer_control_settings: ComputerControlSettingsState,
    /// HTTP 请求设置状态
    pub(crate) http_request_settings: HttpRequestSettingsState,
    /// 网关设置状态
    pub(crate) gateway_settings: GatewaySettingsState,
    pub(crate) apps_catalog: AppsCatalogState,
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) agent_apps_auth: super::agent_apps_auth::State,
    pub(crate) app_providers: super::app_providers::list::ListState,
    /// 客户端网关连接设置状态
    pub(crate) gateway_client_settings: GatewayClientSettingsState,
    /// 委托代理设置状态
    pub(crate) agents_settings: AgentsSettingsState,
    /// 成本控制设置状态
    pub(crate) cost_settings: CostSettingsState,
    /// 记忆系统设置状态
    pub(crate) memory_settings: MemorySettingsState,
    /// 多通道集成设置状态
    pub(crate) channels_settings: ChannelsSettingsState,
    /// 可靠性设置状态
    pub(crate) reliability_settings: ReliabilitySettingsState,
    /// 多模态设置状态
    pub(crate) multimodal_settings: MultimodalSettingsState,
    /// 安全设置状态
    pub(crate) security_settings: SecuritySettingsState,
    /// 自主性设置状态
    pub(crate) autonomy_settings: AutonomySettingsState,
    /// 可观测性设置状态
    pub(crate) observability_settings: ObservabilitySettingsState,
    /// 存储设置状态
    pub(crate) storage_settings: StorageSettingsState,
    /// 代理设置状态
    pub(crate) proxy_settings: ProxySettingsState,
    /// 隧道设置状态
    pub(crate) tunnel_settings: TunnelSettingsState,
    /// 主题设置状态
    pub(crate) themes_settings: ThemesSettingsState,

    pub(crate) file_tree_interaction: desktop_workspace::interaction::InteractionState,
    pub(crate) input_suggestions: desktop_chat_input::input_panel::suggestions::State,
    /// LSP 事件接收器（仅非 wasm32 平台）
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) lsp_events: Option<std::sync::mpsc::Receiver<LspEvent>>,
    /// LSP 事件发送器（仅非 wasm32 平台）
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) lsp_event_sender: Option<std::sync::mpsc::Sender<LspEvent>>,
    /// LSP 服务管理器（仅非 wasm32 平台）
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) lsp_manager: Option<LspServiceManager>,

    /// 当前聚焦区域
    pub(crate) focus_area: FocusArea,

    /// 知识库工作台状态
    pub knowledge: KnowledgeUiState,
    /// Dify 工作流无限画布状态
    pub workflow_state: crate::apps::workflow::state::WorkflowState,


    /// 鼠标光标位置
    pub cursor_position: Point,

    /// 加载动画帧索引
    pub spinner_frame: usize,
    pub(crate) notification_state: desktop_navigation::message::notification::OwnedState,

    pub(crate) toast_state: desktop_ui_controls::toast::State,

    /// Shiju 本地多账户与登录状态。
    pub(crate) account: AccountState,
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) expert_hub: crate::app::skill_hub::ExpertHubState,
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) shiju_apps: crate::app::shiju_apps::ShijuAppsState,
    /// 工作流管理面板状态
    pub workflow_manager: crate::app::views::workflow_manager::WorkflowManagerState,
    pub task_board: desktop_task_board::state::State,
}

#[cfg(test)]
#[path = "app_state_tests.rs"]
mod app_state_tests;

#[cfg(test)]
mod chrome_fields_tests;

#[cfg(test)]
mod core_fields_tests;

#[cfg(test)]
mod fields_navigation_tests;

#[cfg(test)]
mod fields_task_board_tests;

#[cfg(test)]
mod fields_workspace_tests;

#[cfg(test)]
mod git_fields_tests;

#[cfg(test)]
mod settings_preview_fields_tests;

#[cfg(test)]
mod workspace_task_fields_tests;
