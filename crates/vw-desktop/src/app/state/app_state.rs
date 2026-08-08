mod app_aux;

pub use app_aux::{CookieConfig, RecentProjectMeta, WebBookmark};
pub(crate) use app_aux::{
    Notification, Toast, ToastKind, WorkBrowserState, default_recent_project_session_auto_refresh,
    default_recent_project_session_refresh_interval_seconds,
};

use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TaskBoardViewMode {
    #[default]
    Kanban,
    Staff,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QuickMessageGitCommitAction {
    StageAllAndCommit,
    CommitSelected,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum QuickMessageHandoffState {
    WaitingForSummary {
        request_id: u64,
        source_session_id: String,
        project_path: String,
    },
    WaitingForSession {
        prompt: String,
    },
    WaitingForGitCommitMessage {
        request_id: u64,
        source_session_id: String,
        project_path: String,
        action: QuickMessageGitCommitAction,
    },
}

pub(crate) type UsageStepRawKey = (String, u32);
pub(crate) type UsageStepRawPayloadResult = Result<Option<serde_json::Value>, String>;

#[allow(dead_code)]
pub struct App {
    /// 当前显示的屏幕
    pub(crate) screen: Screen,
    /// 当前项目工作区的界面呈现模式
    pub(crate) workspace_mode: WorkspaceMode,
    /// 工作模式右侧资源区的呈现模式
    pub(crate) work_resource_mode: WorkResourceMode,
    /// 是否展示工作模式右侧工作空间模块
    pub(crate) work_resources_visible: bool,
    /// 工作模式对话区占可用内容宽度的比例
    pub(crate) work_split_ratio: f32,
    /// 工作模式左栏宽度
    pub(crate) work_sidebar_width: f32,
    /// 工作模式左栏展开时恢复的宽度
    pub(crate) work_sidebar_expanded_width: f32,
    /// 工作模式左栏是否收起为紧凑工具栏
    pub(crate) work_sidebar_collapsed: bool,
    /// 工作模式左栏过滤弹层是否打开
    pub(crate) work_sidebar_filter_open: bool,
    /// 工作模式左栏“更多”应用菜单是否打开
    pub(crate) work_sidebar_more_open: bool,
    /// 工作模式左栏“项目”导航项是否悬停（展开态显示新建入口）
    pub(crate) work_sidebar_project_nav_hovered: bool,
    /// 工作模式左栏项目范围过滤
    pub(crate) work_sidebar_filter_scope: WorkSidebarFilterScope,
    /// 工作模式左栏会话运行状态过滤
    pub(crate) work_sidebar_filter_status: WorkSidebarFilterStatus,
    /// 工作模式左栏更新时间过滤
    pub(crate) work_sidebar_filter_time: WorkSidebarFilterTime,
    /// 工作模式左栏当前任务/空间标签
    pub(crate) work_sidebar_tab: WorkSidebarTab,
    /// 工作模式左栏当前可见任务数量
    pub(crate) work_sidebar_visible_tasks: usize,
    /// 工作模式左栏当前可见空间数量
    pub(crate) work_sidebar_visible_spaces: usize,
    /// 工作模式输入区空间选择器是否打开
    pub(crate) work_space_picker_open: bool,
    /// 是否正在拖动工作模式左栏
    pub(crate) dragging_work_sidebar: bool,
    /// 工作模式左栏拖动起始 X 坐标
    pub(crate) work_sidebar_drag_anchor_x: Option<f32>,
    /// 工作模式左栏拖动起始宽度
    pub(crate) work_sidebar_drag_start_width: f32,
    /// 是否正在拖动工作模式聊天/资源分割条
    pub(crate) dragging_work_split: bool,
    /// 工作模式聊天/资源分割条拖动起始 X 坐标
    pub(crate) work_split_drag_anchor_x: Option<f32>,
    /// 工作模式聊天/资源分割条拖动起始比例
    pub(crate) work_split_drag_start_ratio: f32,
    /// 工作模式资源区是否全屏
    pub(crate) work_resource_fullscreen: bool,
    /// 工作模式资源区是否保留左栏并独占右侧
    pub(crate) work_resource_half_fullscreen: bool,
    /// 工作模式聊天区是否独占整个工作内容
    pub(crate) work_chat_fullscreen: bool,
    /// 工作模式聊天区是否保留左栏并独占右侧
    pub(crate) work_chat_half_fullscreen: bool,
    /// 工作模式停靠浏览器的输入、错误与焦点状态
    pub(crate) work_browser: WorkBrowserState,
    /// 工作模式停靠浏览器的受控子进程
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) work_browser_controller: crate::app::webview_runtime::WorkBrowserController,
    /// 工作模式左栏中按真实项目路径展开的空间
    pub(crate) work_expanded_spaces: HashSet<String>,
    /// 工作模式左栏各空间当前可见任务数量（项目路径 -> 可见条数）
    pub(crate) work_sidebar_space_task_visible: HashMap<String, usize>,
    /// 当前项目路径
    pub(crate) project_path: Option<String>,
    /// 当前项目是否为 Git 仓库（网关 ProjectInfo.vcs）
    pub(crate) project_is_git_repo: bool,
    /// 当前项目唯一标识符
    pub(crate) project_id: Option<String>,
    /// 项目路径输入框内容
    pub(crate) project_path_input: String,
    /// 输入文本（旧版，已废弃）
    pub(crate) input_text: String,
    /// 输入编辑器内容
    pub(crate) input_editor: text_editor::Content,
    /// 当前选择的模型标识符
    pub(crate) model: String,
    /// 是否自动选择模型
    pub(crate) auto_model: bool,
    pub(crate) reasoning_effort: Option<vw_gateway_client::vw_api_types::ReasoningEffortDto>,
    pub(crate) acp_agent: Option<String>,
    pub(crate) acp_history_mode: AcpHistoryReplayMode,
    pub(crate) acp_recent_count: usize,
    pub(crate) chat_send_behavior: ChatSendBehavior,
    pub(crate) acp_agents: Vec<String>,
    pub(crate) acp_settings: AcpSettingsState,
    pub(crate) mcp_settings: McpSettingsState,
    /// 文件 URL 输入框内容
    pub(crate) file_url_input: String,
    /// 附加文件列表
    pub(crate) files: Vec<String>,
    /// 聊天消息列表
    pub(crate) chat: Vec<models::ChatMessage>,
    /// 当前聊天消息对应的会话消息 ID
    pub(crate) chat_message_ids: Vec<Option<String>>,
    /// 聊天消息渲染缓存（按消息索引）
    pub(crate) chat_render_cache: HashMap<usize, models::ChatRenderCacheEntry>,
    /// 聊天消息可复制文本缓存（按消息索引）
    pub(crate) chat_visible_text_cache: Vec<Option<String>>,
    /// 聊天消息复制哈希缓存（按消息索引）
    pub(crate) chat_copy_hash_cache: Vec<Option<u64>>,
    /// 聊天消息折叠状态
    pub(crate) chat_message_expanded: HashSet<usize>,
    /// 聊天消息持久化高度缓存
    pub(crate) chat_message_estimated_heights: Vec<f32>,
    /// 聊天虚拟滚动统一状态源
    pub(crate) chat_virtualizer: crate::app::components::chat_panel::virtualizer::ChatVirtualizer,
    /// 聊天消息高度索引
    pub(crate) chat_height_index: crate::app::components::chat_panel::height_index::ChatHeightIndex,
    /// 聊天消息测量高度缓存
    pub(crate) chat_message_measured_heights: HashMap<usize, f32>,
    /// 聊天消息编辑器列表（用于消息编辑）
    pub(crate) chat_message_editors: Vec<text_editor::Content>,
    /// 特殊消息中的普通文本编辑器映射（消息索引 + 文本块索引 -> 编辑器内容）
    pub(crate) chat_special_text_editors: HashMap<u64, text_editor::Content>,
    /// 助手消息正文的 Markdown 块结构缓存（消息索引 + 文本块索引 -> 块列表）
    pub(crate) chat_markdown_blocks:
        HashMap<u64, Vec<crate::app::components::chat_panel::message_view::MarkdownBlock>>,
    /// Markdown 块的可划选只读编辑器映射（消息索引 + 文本块索引 + 块索引 -> 编辑器内容）
    pub(crate) chat_markdown_block_editors: HashMap<u128, text_editor::Content>,
    /// 工具卡片文本编辑器映射（消息索引 + 工具索引 + 文本索引 -> 编辑器内容）
    pub(crate) chat_tool_text_editors: HashMap<u128, text_editor::Content>,
    /// 思考块编辑器映射（消息索引 -> 编辑器内容）
    pub(crate) chat_think_editors: HashMap<u64, text_editor::Content>,
    /// 展开的思考块消息索引集合
    pub(crate) chat_think_expanded: HashSet<u64>,
    /// 手动折叠的思考块消息索引集合
    pub(crate) chat_think_collapsed: HashSet<u64>,
    /// 鼠标悬停的思考块消息索引
    pub(crate) chat_think_hovered_idx: Option<u64>,
    /// 展开的工具内文件项集合
    pub(crate) chat_tool_file_expanded: HashSet<String>,
    /// 鼠标悬停的工具内文件项键
    pub(crate) chat_tool_file_hovered: Option<String>,
    /// 展开的工具调用块集合
    pub(crate) chat_tool_expanded: HashSet<u64>,
    /// 鼠标悬停的工具调用块消息索引
    pub(crate) chat_tool_hovered_idx: Option<u64>,
    /// 已探索摘要块展开状态
    pub(crate) chat_explore_expanded: HashSet<u64>,
    /// 已探索摘要数字翻转动画状态
    pub(crate) chat_explore_summary_animations: HashMap<u128, ExploreSummaryAnimationState>,
    /// 工具详情对话框状态
    pub(crate) tool_detail_dialog: Option<ToolDetailDialog>,
    /// 思考块滚动容器 ID 映射
    pub(crate) chat_think_scroll_ids: HashMap<u64, Id>,
    /// 流式响应中当前思考块的消息索引
    pub(crate) chat_stream_think_msg_idx: Option<usize>,
    /// 流式响应中的思考块计数
    pub(crate) chat_stream_think_count: usize,
    /// 流式响应中自动展开的思考块索引
    pub(crate) chat_stream_think_open_idx: Option<usize>,
    /// 当前打开的聊天右键菜单目标键
    pub(crate) chat_context_menu_target: Option<u64>,
    /// 聊天右键菜单在消息区域内的锚点位置
    pub(crate) chat_context_menu_pos: Option<(f32, f32)>,
    /// 聊天右键菜单关联的文本内容
    pub(crate) chat_context_menu_text: String,
    /// 输入框右键菜单是否显示
    pub(crate) input_context_menu_open: bool,
    /// 输入框右键菜单在编辑器区域内的锚点位置
    pub(crate) input_context_menu_pos: Option<(f32, f32)>,
    /// 当前打开的“重置到此点”菜单消息索引
    pub(crate) chat_reset_menu_idx: Option<usize>,
    /// 当前打开的“分叉到新会话”对话框消息索引
    pub(crate) chat_fork_dialog_idx: Option<usize>,
    /// Todo 面板是否展开
    pub(crate) chat_todo_expanded: bool,
    /// Todo 面板动画进度
    pub(crate) chat_todo_anim: f32,
    /// Todo 面板显示位置
    pub(crate) chat_todo_placement: TodoPanelPlacement,
    /// Todo 缓存所属会话 ID
    pub(crate) chat_todo_session_id: Option<String>,
    /// 当前活跃会话的 Todo 缓存
    pub(crate) chat_todo_items: Vec<vw_shared::todo::Todo>,
    /// Token 使用量统计
    pub(crate) usage: models::TokenUsage,
    /// 当前活跃会话的轻量视图状态
    pub(crate) active_session_view_state: ActiveSessionViewState,
    /// 当前模型的使用信息
    pub(crate) usage_model_info: Option<UsageModelInfo>,
    /// 展开的使用量步骤索引集合
    pub(crate) usage_step_expanded: HashSet<u32>,
    /// 使用量步骤原始 LLM 载荷缓存
    pub(crate) usage_step_raw_payloads: HashMap<UsageStepRawKey, UsageStepRawPayloadResult>,
    /// 正在加载原始 LLM 载荷的使用量步骤
    pub(crate) usage_step_raw_loading: HashSet<UsageStepRawKey>,
    /// 使用量步骤原始记录弹窗
    pub(crate) usage_step_raw_modal: Option<UsageStepRawModal>,
    /// 是否显示合并视图
    pub(crate) merge_view: bool,
    /// 展开的文件路径列表
    pub(crate) expanded_files: Vec<String>,
    /// 展开的文件路径集合（用于快速查询）
    pub(crate) expanded_files_set: HashSet<String>,
    /// Git 分支列表
    pub(crate) branches: Vec<String>,
    /// 当前选中的分支
    pub(crate) selected_branch: Option<String>,
    /// Git 面板可切换的工作树选项
    pub(crate) git_worktree_options: Vec<GitWorktreeOption>,
    /// Git 面板当前选中的工作树目录
    pub(crate) selected_git_worktree_directory: Option<String>,
    /// Git 面板工作树选项是否正在加载
    pub(crate) git_worktree_options_loading: bool,
    /// Git 面板工作树选项当前对应的项目路径
    pub(crate) git_worktree_options_project_path: Option<String>,
    /// Git 面板工作树菜单是否打开
    pub(crate) git_worktree_menu_open: bool,
    /// 当前项目最近更新时间（毫秒时间戳）
    pub(crate) project_updated_at_ms: Option<u64>,
    /// 最近打开的项目路径列表
    pub(crate) recent_projects: Vec<String>,
    /// 是否正在处理请求
    pub(crate) is_requesting: bool,
    /// 提交按钮动画计数器
    pub(crate) submit_anim: u8,
    /// 当前活跃的 Agent 请求
    pub(crate) active_agent_request: Option<AgentRequest>,
    /// Agent 流式响应的唯一标识符
    pub(crate) agent_stream_id: u64,
    /// 请求队列
    pub(crate) queue: Vec<QueueItem>,
    /// 主分割面板比例
    pub(crate) split_ratio: f32,
    /// 是否正在拖动主分割面板
    pub(crate) dragging_split: bool,
    /// 主分割面板拖动起始 X 坐标
    pub(crate) split_drag_anchor_x: Option<f32>,
    /// 主分割面板拖动起始比例
    pub(crate) split_drag_start_ratio: f32,
    /// 窗口尺寸（宽，高）
    pub(crate) window_size: (f32, f32),
    /// 全屏切换后的布局稳定期是否进行中
    pub(crate) fullscreen_layout_settling: bool,
    pub(crate) startup_resize_checked: bool,
    /// 窗口位置（X，Y）
    pub(crate) window_position: (f32, f32),
    /// 主窗口 ID
    pub(crate) main_window_id: Option<iced::window::Id>,
    /// 独立小宠物窗口 ID
    pub(crate) task_pet_window_id: Option<iced::window::Id>,
    /// 是否显示设置面板
    pub(crate) show_settings: bool,
    /// 设置侧边栏是否折叠
    pub(crate) settings_sidebar_collapsed: bool,
    /// 是否显示系统设置面板
    pub(crate) show_system_settings: bool,
    /// 是否显示关于对话框
    pub(crate) show_about_modal: bool,
    /// 是否显示 CLI 安装对话框
    pub(crate) show_cli_install_modal: bool,
    /// CLI 安装对话框标题
    pub(crate) cli_install_modal_title: String,
    /// CLI 安装对话框消息内容
    pub(crate) cli_install_modal_message: String,
    /// CLI 更新检测对话框中的当前版本
    pub(crate) cli_install_modal_current_version: String,
    /// CLI 更新检测对话框中的服务器版本
    pub(crate) cli_install_modal_server_version: String,
    /// CLI 更新检测对话框是否显示检测按钮
    pub(crate) cli_install_modal_show_update_action: bool,
    /// CLI 更新检测对话框是否显示安装按钮
    pub(crate) cli_install_modal_show_install_action: bool,
    /// CLI 安装对话框是否使用应用自身更新动作
    pub(crate) cli_install_modal_use_app_update_action: bool,
    /// CLI 更新检测是否进行中
    pub(crate) cli_install_modal_is_checking_update: bool,
    /// 应用自更新下载进度
    pub(crate) app_update_progress: Option<vw_shared::update::UpdateProgress>,
    /// 应用自更新下载进度接收器
    pub(crate) app_update_progress_rx:
        Option<std::sync::mpsc::Receiver<vw_shared::update::UpdateProgress>>,
    /// 问题对话框关联的请求 ID
    pub(crate) question_modal_request_id: Option<String>,
    /// 当前问题对话框承载的请求数据
    pub(crate) question_modal_request: Option<vw_shared::question::Request>,
    /// 问题对话框的答案选项列表（每个问题多个选项）
    pub(crate) question_modal_answers: Vec<Vec<String>>,
    /// 问题对话框的自定义答案输入
    pub(crate) question_modal_custom: Vec<String>,
    /// 权限对话框关联的请求 ID
    pub(crate) permission_modal_request_id: Option<String>,
    /// 当前权限对话框承载的请求数据
    pub(crate) permission_modal_request: Option<vw_gateway_client::PendingPermissionRequestDto>,
    /// 当前轮询到的全部待审批权限请求
    pub(crate) permission_modal_requests: Vec<vw_gateway_client::PendingPermissionRequestDto>,
    /// 当前激活的菜单类型
    pub(crate) active_menu: Option<message::view::MenuType>,
    /// 默认使用的外部打开应用
    pub(crate) open_external_app: ExternalOpenApp,
    /// 外部应用检测结果对应的运行时平台
    pub(crate) open_external_platform: Option<RuntimePlatform>,
    /// 外部应用存在性检查结果缓存
    pub(crate) open_external_exists: HashMap<ExternalOpenApp, bool>,
    /// 设置面板宽度
    pub(crate) settings_panel_width: f32,
    /// 是否正在拖动设置面板
    pub(crate) dragging_settings: bool,
    /// 设置面板拖动起始 X 坐标
    pub(crate) settings_drag_anchor_x: Option<f32>,
    /// 设置面板拖动起始宽度
    pub(crate) settings_drag_start_width: f32,
    /// 文件管理器面板宽度
    pub(crate) file_manager_width: f32,
    /// 是否正在拖动文件管理器面板
    pub(crate) dragging_file_manager: bool,
    /// 文件管理器面板拖动起始 X 坐标
    pub(crate) file_manager_drag_anchor_x: Option<f32>,
    /// 文件管理器面板拖动起始宽度
    pub(crate) file_manager_start_width: f32,
    /// 文件管理器是否显示变更
    pub(crate) file_manager_show_changes: bool,
    /// 文件管理器刷新动画帧
    pub(crate) file_manager_refresh_frame: usize,
    /// 通用状态动画帧
    pub(crate) status_animation_frame: usize,
    /// 文件管理器 Git 更改列表是否正在手动刷新
    pub(crate) file_manager_changes_refreshing: bool,
    /// 文件管理器文件树是否正在手动刷新
    pub(crate) file_manager_file_tree_refreshing: bool,
    /// 是否显示文件管理器
    pub(crate) show_file_manager: bool,
    /// 是否显示模型选择弹出框
    pub(crate) show_model_popover: bool,
    /// 是否显示模式选择弹出框
    pub(crate) show_mode_popover: bool,
    /// 是否显示代码/工作界面模式弹出框
    pub(crate) show_workspace_mode_popover: bool,
    /// 是否显示文件选择弹出框
    pub(crate) show_send_mode_popover: bool,
    pub(crate) show_file_popover: bool,
    /// 是否显示 ACP 选择弹出框
    pub(crate) show_acp_popover: bool,
    /// 是否显示推理强度选择弹出框
    pub(crate) show_reasoning_effort_popover: bool,
    /// 是否显示使用量弹出框
    pub(crate) show_usage_popover: bool,
    /// 是否显示会话工具选择弹出框
    pub(crate) show_session_tool_selector_popover: bool,
    /// 是否显示当前父会话的 AgentTool 子会话弹层（不持久化）
    pub(crate) show_agent_sessions_popover: bool,
    /// 是否显示会话操作弹出框
    pub(crate) show_session_actions_popover: bool,
    /// 是否显示执行器选择弹出框
    pub(crate) show_executor_popover: bool,
    /// 会话标题上次点击时间（用于双击检测）
    pub(crate) session_title_last_click: Option<web_time::Instant>,
    /// 模型选择弹出框中的悬停项
    pub(crate) model_popover_hover: Option<ModelPopoverHover>,
    /// 是否启用自动最大化模式
    pub(crate) auto_max_mode: bool,
    /// 最近一次调用的日志文件路径
    pub(crate) last_call_log_path: Option<String>,
    /// 最近一次会话快照文件路径
    pub(crate) last_session_snapshot_path: Option<String>,
    /// 搜索文本输入
    pub(crate) search_text: String,
    /// 是否显示搜索覆盖层
    pub(crate) show_search_overlay: bool,
    /// 文件索引缓存（路径 -> 子文件列表）
    pub(crate) file_index_cache: HashMap<String, Vec<String>>,
    /// 文件索引版本号，用于搜索缓存失效判断
    pub(crate) file_index_revision: u64,
    /// 文件树缓存（路径 -> 目录树模型）
    pub(crate) file_tree_model_cache:
        HashMap<String, crate::app::components::file_tree::model::FileTreeNode>,
    /// 全局搜索面板文件结果缓存查询
    pub(crate) search_panel_file_cache_query: String,
    /// 全局搜索面板文件结果缓存项目路径
    pub(crate) search_panel_file_cache_project_path: Option<String>,
    /// 全局搜索面板文件结果缓存索引版本
    pub(crate) search_panel_file_cache_revision: u64,
    /// 全局搜索面板文件结果缓存
    pub(crate) search_panel_file_cache_results: Vec<String>,
    /// 输入框文件搜索缓存查询
    pub(crate) file_search_cache_query: String,
    /// 输入框文件搜索缓存项目路径
    pub(crate) file_search_cache_project_path: Option<String>,
    /// 输入框文件搜索缓存索引版本
    pub(crate) file_search_cache_revision: u64,
    /// 输入框文件搜索缓存结果
    pub(crate) file_search_cache_entries: Vec<crate::app::message::chat::input::FileSearchResult>,
    /// 会话信息列表
    pub(crate) sessions: Vec<vw_shared::session::info::Info>,
    /// 会话预览映射（会话 ID -> 预览文本），缓存避免每帧查 DB
    pub(crate) session_previews: HashMap<String, String>,
    /// 当前活跃会话的唯一标识符
    pub(crate) active_session_id: Option<String>,
    /// 会话聊天消息缓存（会话 ID -> 消息列表）
    pub(crate) session_chat_cache: HashMap<String, crate::app::session::SharedChatMessages>,
    /// 会话消息 ID 缓存（会话 ID -> 消息 ID 列表）
    pub(crate) session_chat_message_id_cache: HashMap<String, Vec<Option<String>>>,
    /// 会话历史重置边界（会话 ID -> 重置时保留的消息 ID 列表）
    pub(crate) session_history_reset_boundaries: HashMap<String, Vec<Option<String>>>,
    /// 会话运行时状态映射（会话 ID -> 运行时状态）
    pub(crate) session_runtime_states: HashMap<String, SessionRuntimeState>,
    /// 已归档的会话 ID 集合
    pub(crate) archived_session_ids: HashSet<String>,
    /// 项目会话映射（项目路径 -> 会话列表）
    pub(crate) project_sessions: HashMap<String, Vec<vw_shared::session::info::Info>>,
    /// 项目会话加载计数（用于分页）
    pub(crate) project_session_load_counts: HashMap<String, usize>,
    /// 左侧普通会话树的展开会话 ID 集合（不持久化）
    pub(crate) session_tree_expanded: HashSet<String>,
    /// 正在加载会话的项目路径集合
    pub(crate) project_sessions_loading: HashSet<String>,
    /// 各项目会话列表是否显示纵向滚动条
    pub(crate) project_session_has_vertical_scrollbar: HashMap<String, bool>,
    /// 项目会话上次刷新时间
    pub(crate) project_sessions_last_refresh_at: HashMap<String, web_time::Instant>,
    /// 会话菜单的目标会话 ID
    pub(crate) session_menu_id: Option<String>,
    /// 会话菜单的锚点位置
    pub(crate) session_menu_anchor: Option<Point>,
    /// 项目工具菜单的文件路径
    pub(crate) project_tools_menu_path: Option<String>,
    /// 新建会话选择器的项目路径
    pub(crate) new_session_picker_project: Option<String>,
    /// 是否正在等待新会话创建完成
    pub(crate) new_session_creating: bool,
    /// 新建会话选择器的选项列表（显示名，值）
    pub(crate) new_session_picker_options: Vec<(String, String)>,
    /// 项目 worktree 启用状态映射
    pub(crate) project_worktree_enabled: HashMap<String, bool>,
    /// 新建会话的上次选择目录
    pub(crate) new_session_last_directory: Option<String>,
    /// 新建会话的 worktree 名称
    pub(crate) new_session_worktree_name: String,
    /// 新建会话确认删除的目录路径
    pub(crate) new_session_confirm_delete_directory: Option<String>,
    /// 新建会话强制删除的目录路径
    pub(crate) new_session_force_delete_directory: Option<String>,
    /// 新建会话删除操作的错误信息
    pub(crate) new_session_delete_error: Option<String>,
    /// 新建会话确认重置的目录路径
    pub(crate) new_session_confirm_reset_directory: Option<String>,
    /// 新建会话重置操作的错误信息
    pub(crate) new_session_reset_error: Option<String>,
    /// 正在重命名的会话 ID
    pub(crate) session_rename_id: Option<String>,
    /// 会话重命名输入值
    pub(crate) session_rename_value: String,
    /// 正在编辑的项目路径
    pub(crate) project_edit_path: Option<String>,
    /// 项目设置当前选中的标签页
    pub(crate) project_edit_tab: ProjectEditTab,
    /// 项目编辑名称输入
    pub(crate) project_edit_name: String,
    /// 项目图标标识符
    pub(crate) project_edit_icon: String,
    /// 项目图标是否被悬停
    pub(crate) project_edit_icon_hovered: bool,
    /// 项目图标颜色（十六进制）
    pub(crate) project_edit_icon_color: String,
    /// 项目图标颜色选择器是否打开
    pub(crate) project_edit_icon_color_picker_open: bool,
    /// 项目图标颜色格式
    pub(crate) project_edit_icon_color_format: ColorFormat,
    /// 项目启动脚本
    pub(crate) project_edit_start_script: String,
    /// 项目启动脚本编辑器内容
    pub(crate) project_edit_start_script_editor: text_editor::Content,
    /// 项目是否启用 worktree
    pub(crate) project_edit_worktree_enabled: bool,
    /// 项目任务看板设置
    pub(crate) project_edit_task_board_settings: crate::app::task::TaskBoardSettings,
    /// 项目最大并发数输入
    pub(crate) project_edit_max_concurrent_input: String,
    /// 项目任务看板自动刷新开关
    pub(crate) project_edit_task_board_auto_refresh: bool,
    /// 项目会话自动刷新开关
    pub(crate) project_edit_session_auto_refresh: bool,
    /// 项目会话自动刷新间隔秒数输入
    pub(crate) project_edit_session_refresh_interval_seconds_input: String,
    /// 项目任务看板自动刷新间隔秒数输入
    pub(crate) project_edit_task_board_refresh_interval_seconds_input: String,
    /// 项目任务调度 tick 间隔秒数输入
    pub(crate) project_edit_task_board_scheduler_tick_interval_seconds_input: String,
    /// 项目任务池自动执行 tick 间隔秒数输入
    pub(crate) project_edit_task_board_auto_promote_tick_interval_seconds_input: String,
    /// 项目失败重试分钟数输入
    pub(crate) project_edit_failed_retry_minutes_input: String,
    /// 项目失败最大重试次数输入
    pub(crate) project_edit_failed_retry_max_attempts_input: String,
    /// 项目运行超时分钟数输入
    pub(crate) project_edit_running_timeout_minutes_input: String,
    /// 项目 PR 提交停滞超时秒数输入
    pub(crate) project_edit_pr_submitted_stall_timeout_seconds_input: String,
    /// 是否显示差异视图
    pub(crate) show_diff: bool,
    /// 是否显示 Git 差异摘要
    pub(crate) show_git_diff_summary: bool,
    /// 是否显示 Git 差异高亮
    pub(crate) show_git_diff_highlight: bool,
    /// 终端状态（主终端）
    pub terminal: TerminalState,
    /// 按项目分组的终端状态映射
    pub(crate) terminals_by_project: HashMap<String, TerminalState>,
    /// 差异视图主题
    pub(crate) diff_theme: DiffTheme,
    /// 应用主题
    pub(crate) app_theme: Theme,
    /// 用于模型聊天时间上下文的 IANA 时区
    pub(crate) chat_timezone: String,
    /// 当前应用的自定义主题 id（配置键 `custom:<id>`）；内置主题时为 `None`
    pub(crate) active_custom_theme_id: Option<String>,
    /// 活动自定义主题的皮肤快照（背景路径与 chrome）；内置主题时为 `None`
    pub(crate) active_theme_skin: Option<crate::app::theme_catalog::ActiveThemeSkin>,
    /// 编辑器是否跟随系统主题
    pub(crate) editor_follow_system_theme: bool,
    /// 编辑器主题
    pub(crate) editor_theme: Theme,
    /// 展开的差异块列表（文件路径，块索引）
    pub(crate) expanded_hunks: Vec<(String, usize)>,
    /// 上下文扩展映射（文件路径，块索引） -> （前扩展行数，后扩展行数）
    pub(crate) context_expansions: HashMap<(String, usize), (usize, usize)>,
    /// Git 提交消息输入
    pub(crate) git_commit_message: String,
    /// Git 提交类型（约定式提交）
    pub(crate) git_commit_type: Option<ConventionalCommitType>,
    /// Git 提交作用域输入
    pub(crate) git_commit_scope: String,
    /// Git 提交描述输入
    pub(crate) git_commit_description: String,
    /// Git 提交描述编辑器内容
    pub(crate) git_commit_description_editor: text_editor::Content,
    /// Git 提交是否进行中
    pub(crate) git_commit_in_progress: bool,
    /// 是否显示约定式提交帮助弹窗
    pub(crate) show_git_commit_help_modal: bool,
    /// 是否显示 Git 过滤帮助弹窗
    pub(crate) show_git_filter_help_modal: bool,
    /// 已暂存的文件列表
    pub(crate) staged_files_selected: Vec<String>,
    /// 已暂存的差异块列表（文件路径，块索引）
    pub(crate) staged_hunks_selected: Vec<(String, usize)>,
    /// 已暂存的行列表（文件路径，行号）
    pub(crate) staged_lines_selected: Vec<(String, usize)>,
    /// 已暂存的旧行列表（文件路径，行号）
    pub(crate) staged_old_lines_selected: Vec<(String, usize)>,
    /// Git 差异中选中的行列表
    pub(crate) git_diff_selected_lines: Vec<GitDiffSelectedLine>,
    /// Git 差异拖动选择的行范围
    pub(crate) git_diff_drag_range: Option<GitDiffLineRange>,
    /// Git 差异已选中的连续行范围
    pub(crate) git_diff_selected_range: Option<GitDiffLineRange>,
    /// 是否正在拖动选择 Git 差异行
    pub(crate) git_diff_dragging: bool,
    /// Git 差异拖动起始文本
    pub(crate) git_diff_drag_start_text: Option<String>,
    /// Git 差异上次点击信息（文件，行号，是否旧版本，时间）
    pub(crate) git_diff_last_click: Option<(String, usize, bool, web_time::Instant)>,
    /// Git 差异悬停行信息（文件，行号，是否旧版本）
    pub(crate) git_diff_hovered_line: Option<(String, usize, bool)>,
    /// Git 差异评论草稿
    pub(crate) git_diff_comment_draft: Option<GitDiffCommentDraft>,
    /// Git 差异右键菜单状态
    pub(crate) git_diff_context_menu: Option<GitDiffContextMenuState>,
    /// Git 差异文件操作菜单状态
    pub(crate) git_diff_file_menu: Option<GitDiffFileMenuState>,
    /// 是否显示 Git 复制对话框
    pub(crate) show_git_copy_modal: bool,
    /// Git 复制对话框编辑器内容
    pub(crate) git_copy_modal_editor: text_editor::Content,
    /// Git 复制对话框是否使用颜色
    pub(crate) git_copy_modal_use_color: bool,
    /// Git 复制对话框代码编辑器
    pub(crate) git_copy_modal_code_editor: iced_code_editor::CodeEditor,
    /// 是否显示自定义差异对话框
    pub(crate) show_git_custom_diff_modal: bool,
    /// 自定义差异对话框是否隐藏输入框
    pub(crate) git_custom_diff_hide_inputs: bool,
    /// 自定义差异标题
    pub(crate) git_custom_diff_title: String,
    /// 自定义差异前内容编辑器
    pub(crate) git_custom_diff_before_editor: text_editor::Content,
    /// 自定义差异后内容编辑器
    pub(crate) git_custom_diff_after_editor: text_editor::Content,
    /// 聊天中显示的文本差异
    pub(crate) chat_text_diff: Option<ChatTextDiff>,
    /// 是否显示 Git 过滤选项
    pub(crate) show_git_filter_options: bool,
    /// Git 过滤查询输入
    pub(crate) git_filter_query: String,
    /// Git 过滤是否包含指定路径
    pub(crate) git_filter_included: bool,
    /// Git 过滤是否排除指定路径
    pub(crate) git_filter_excluded: bool,
    /// Git 过滤是否包含新文件
    pub(crate) git_filter_new: bool,
    /// Git 过滤是否包含修改文件
    pub(crate) git_filter_modified: bool,
    /// Git 过滤是否包含删除文件
    pub(crate) git_filter_deleted: bool,
    /// Git 聚焦的文件路径
    pub(crate) git_focused_file: Option<String>,
    /// Git 悬停的文件头部路径
    pub(crate) git_hovered_file_header: Option<String>,
    /// Git 面板头部是否被悬停
    pub(crate) git_panel_header_hovered: bool,
    /// Git 变更文件列表
    pub(crate) git_changed_files: Vec<String>,
    /// Git 变更文件是否正在加载
    pub(crate) git_changed_files_loading: bool,
    /// Git 变更文件列表当前对应的仓库路径
    pub(crate) git_changed_files_repo_path: Option<String>,
    /// Git diff 文件元数据缓存
    pub(crate) git_diff_file_metas: Vec<crate::app::components::git_panel::DiffFileMeta>,
    /// Git diff 文件元数据是否正在加载
    pub(crate) git_diff_file_metas_loading: bool,
    /// Git diff 文件元数据当前对应的仓库路径
    pub(crate) git_diff_file_metas_repo_path: Option<String>,
    pub(crate) git_diff_contents: HashMap<String, (String, String)>,
    pub(crate) git_diff_contents_loading: HashSet<String>,
    pub(crate) git_diff_scroll_offset_y: f32,
    pub(crate) git_diff_scroll_viewport_h: f32,
    /// 最近复制的代码哈希
    pub(crate) last_copied_code_hash: Option<u64>,
    /// 最近复制时间
    pub(crate) last_copy_time: Option<web_time::SystemTime>,
    /// 聊天是否自动滚动
    pub(crate) chat_auto_scroll: bool,
    /// 聊天滚动容器 ID
    pub(crate) chat_scroll_id: Id,
    /// 聊天滚动相对偏移（0.0-1.0）
    pub(crate) chat_scroll_offset_y: f32,
    /// 聊天滚动视口高度
    pub(crate) chat_scroll_viewport_h: f32,
    /// 小宠物任务列表滚动容器 ID
    pub(crate) task_pet_scroll_id: Id,
    /// 小宠物面板位置
    pub(crate) task_pet_position: Point,
    /// 小宠物面板是否收起
    pub(crate) task_pet_collapsed: bool,
    /// 小宠物展开动画进度（0.0 收起，1.0 展开）
    pub(crate) task_pet_expand_progress: f32,
    /// 小宠物展开动画目标
    pub(crate) task_pet_expand_target: Option<f32>,
    /// 小宠物面板是否正在拖拽
    pub(crate) task_pet_dragging: bool,
    /// 小宠物拖拽锚点
    pub(crate) task_pet_drag_anchor: Option<Point>,
    /// 小宠物拖拽起始位置
    pub(crate) task_pet_drag_start: Point,
    /// 小宠物拖动朝向（-1 左，1 右）
    pub(crate) task_pet_drag_direction: i8,
    /// 小宠物脚步动画保持到的时间戳
    pub(crate) task_pet_walk_until_ms: u64,
    /// 小宠物当前形态
    pub(crate) task_pet_avatar_kind: TaskPetAvatarKind,
    /// 鼠标是否悬停在小宠物本体上
    pub(crate) task_pet_robot_hovered: bool,
    /// 小宠物展示的任务条目
    pub(crate) task_pet_items: Vec<TaskPetItem>,
    /// 当前鼠标悬停的小宠物任务请求 ID
    pub(crate) task_pet_hovered_request_id: Option<u64>,
    /// 当前正在回复的小宠物任务请求 ID
    pub(crate) task_pet_reply_request_id: Option<u64>,
    /// 小宠物回复输入内容
    pub(crate) task_pet_reply_input: String,
    /// 已主动隐藏的请求 ID
    pub(crate) task_pet_dismissed_request_ids: HashSet<u64>,
    /// 流式场景下上次自动贴底触发时间（毫秒）
    pub(crate) chat_stream_autoscroll_last_ms: u64,
    /// 程序化贴底后的保护窗口截止时间（毫秒）
    pub(crate) chat_autoscroll_hold_until_ms: u64,
    /// 聊天面板是否全屏占据右侧区域
    pub(crate) chat_panel_fullscreen: bool,
    /// 聊天面板是否半全屏占据主内容区域但保留文件树和终端
    pub(crate) chat_panel_half_fullscreen: bool,
    /// 是否显示聊天右上角全屏浮动控件
    pub(crate) show_chat_fullscreen_overlay: bool,
    /// 输入编辑器 ID
    pub(crate) input_editor_id: Id,
    /// 文件搜索滚动容器 ID
    pub(crate) file_search_scroll_id: Id,
    /// Slash 面板滚动容器 ID
    pub(crate) slash_palette_scroll_id: Id,
    /// 预览滚动容器 ID
    pub(crate) preview_scroll_id: Id,
    /// 预览标签页滚动容器 ID
    pub(crate) preview_tabs_scroll_id: Id,
    /// Git 差异滚动容器 ID
    pub(crate) git_diff_scroll_id: Id,
    /// Git Diff 是否全屏占据右侧区域
    pub(crate) git_diff_fullscreen: bool,
    /// Git Diff 是否半全屏占据主内容区域但保留文件树和终端
    pub(crate) git_diff_half_fullscreen: bool,
    /// 主页应用栏滚动 X 偏移
    pub(crate) home_apps_bar_scroll_x: f32,
    /// 主页应用栏滚动容器 ID
    pub(crate) home_apps_bar_scroll_id: Id,
    /// 设置标签页
    pub(crate) settings_tab: SettingsTab,
    /// 系统设置标签页
    pub(crate) system_settings_tab: crate::app::components::system_settings::SystemTab,
    /// 系统设置分类搜索词
    pub(crate) system_settings_query: String,
    /// 当前打开的系统设置帮助标签页
    pub(crate) system_settings_help_tab: Option<crate::app::components::system_settings::SystemTab>,
    /// 顶栏网关服务弹窗的当前标签页
    pub(crate) top_bar_gateway_tab: TopBarGatewayTab,
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
    /// 目标循环设置状态
    pub(crate) goal_loop_settings: GoalLoopSettingsState,
    /// 心跳设置状态
    pub(crate) heartbeat_settings: HeartbeatSettingsState,
    /// Cron 设置状态
    pub(crate) cron_settings: CronSettingsState,
    /// SOP 设置状态
    pub(crate) sop_settings: SopSettingsState,
    /// 调度器设置状态
    pub(crate) scheduler_settings: SchedulerSettingsState,
    /// Hooks 设置状态
    pub(crate) hooks_settings: HooksSettingsState,
    /// 运行时设置状态
    pub(crate) runtime_settings: RuntimeSettingsState,
    /// 技能设置状态
    pub(crate) skills_settings: SkillsSettingsState,
    /// 研究设置状态
    pub(crate) research_settings: ResearchSettingsState,
    /// Web 搜索设置状态
    pub(crate) web_search_settings: WebSearchSettingsState,
    /// 浏览器设置状态
    pub(crate) browser_settings: BrowserSettingsState,
    /// HTTP 请求设置状态
    pub(crate) http_request_settings: HttpRequestSettingsState,
    /// 网关设置状态
    pub(crate) gateway_settings: GatewaySettingsState,
    /// 客户端网关连接设置状态
    pub(crate) gateway_client_settings: GatewayClientSettingsState,
    /// Agent 间 IPC 设置状态
    pub(crate) agents_ipc_settings: AgentsIpcSettingsState,
    /// 委托代理设置状态
    pub(crate) agents_settings: AgentsSettingsState,
    /// 员工配置设置状态
    pub(crate) staff_settings: StaffSettingsState,
    /// 协调设置状态
    pub(crate) coordination_settings: CoordinationSettingsState,
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
    /// Composio 设置状态
    pub(crate) composio_settings: ComposioSettingsState,
    /// 转录设置状态
    pub(crate) transcription_settings: TranscriptionSettingsState,
    /// 最近项目编辑列表
    pub(crate) recent_projects_edits: Vec<String>,
    /// 确认删除的最近项目索引
    pub(crate) recent_project_delete_confirm_idx: Option<usize>,
    /// 确认从列表移除的最近项目路径
    pub(crate) recent_project_remove_confirm_path: Option<String>,
    /// 对话流权限编辑器内容
    pub(crate) dialogue_flow_permission_editor: text_editor::Content,
    pub(crate) dialogue_flow_show_reasoning_summary: bool,
    pub(crate) dialogue_flow_expand_shell_tool_section: bool,
    pub(crate) dialogue_flow_expand_edit_tool_section: bool,
    pub(crate) quick_messages: Vec<vw_config_types::ui::QuickMessageConfig>,
    pub(crate) quick_message_open_group: Option<vw_config_types::ui::QuickMessageGroup>,
    pub(crate) show_shiju_skill_menu: bool,
    pub(crate) quick_message_handoff: Option<QuickMessageHandoffState>,
    /// 对话流设置保存消息
    pub(crate) dialogue_flow_settings_save_message: Option<String>,
    /// 最近项目元数据列表
    pub(crate) recent_projects_meta: Vec<RecentProjectMeta>,
    /// 展开的文件树路径列表
    pub(crate) file_tree_expanded: Vec<String>,
    /// 展开的文件树路径集合（用于快速查询）
    pub(crate) file_tree_expanded_set: HashSet<String>,
    /// 文件树右键菜单的目标路径
    pub(crate) file_tree_menu_path: Option<String>,
    /// 文件树右键菜单的锚点位置
    pub(crate) file_tree_menu_anchor: Option<Point>,
    /// 文件树右键菜单的来源（如 "tree"、"tab" 等）
    pub(crate) file_tree_menu_source: Option<String>,
    /// 文件树剪贴板内容
    pub(crate) file_tree_clipboard: Option<FileTreeClipboard>,
    /// 正在拖动的文件路径列表
    pub(crate) dragging_file_paths: Vec<String>,
    /// 正在拖动文件的插入位置（兄弟索引，子索引）
    pub(crate) dragging_file_position: Option<(usize, usize)>,
    /// 待放置的文件路径列表
    pub(crate) pending_drop_file_paths: Vec<String>,
    /// 待放置文件的插入位置
    pub(crate) pending_drop_file_position: Option<(usize, usize)>,
    /// 输入框是否有文件悬停（用于拖放高亮）
    pub(crate) input_drop_hovered: bool,
    /// 正在重命名的文件路径
    pub(crate) file_tree_rename_path: Option<String>,
    /// 文件重命名输入值
    pub(crate) file_tree_rename_value: String,
    /// 文件搜索查询文本
    pub(crate) file_search_query: String,
    /// 是否显示文件搜索面板
    pub(crate) show_file_search: bool,
    /// 文件搜索面板的锚点位置（X，Y）
    pub(crate) file_search_anchor: Option<(f32, f32)>,
    /// 文件搜索结果中选中的索引
    pub(crate) file_search_selected_index: usize,
    /// Slash 面板查询文本（不含前导 `/`）
    pub(crate) slash_palette_query: String,
    /// 是否显示 Slash 快捷选择面板
    pub(crate) show_slash_palette: bool,
    /// Slash 面板可见条目中的选中索引
    pub(crate) slash_palette_selected_index: usize,
    /// Slash 面板已展开的类别
    pub(crate) slash_palette_expanded_kinds:
        std::collections::BTreeSet<crate::app::state::SlashPaletteKind>,
    /// 在文件夹中查找的结果标签页列表
    pub(crate) find_results_tabs: Vec<FindInFolderTab>,
    /// 当前活跃的查找结果标签页 ID
    pub(crate) active_find_results_tab_id: Option<String>,
    /// 工具文件过滤器文本
    pub(crate) tool_files_filter: String,
    /// 文件引用悬停的索引
    pub(crate) file_ref_hovered_index: Option<usize>,
    /// 预览标签页列表
    pub(crate) preview_tabs: Vec<PreviewTab>,
    /// 当前活跃的预览路径
    pub(crate) active_preview_path: Option<String>,
    /// 按项目分组的预览标签页映射
    pub(crate) project_preview_tabs: HashMap<String, Vec<PreviewTab>>,
    /// 按项目分组的活跃预览路径映射
    pub(crate) project_preview_active_path: HashMap<String, Option<String>>,
    /// 预览标签页右键菜单的目标路径
    pub(crate) preview_tab_menu_path: Option<String>,
    /// 预览标签页右键菜单位置
    pub(crate) preview_tab_menu_pos: Option<Point>,
    /// LSP 事件接收器（仅非 wasm32 平台）
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) lsp_events: Option<std::sync::mpsc::Receiver<LspEvent>>,
    /// LSP 事件发送器（仅非 wasm32 平台）
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) lsp_event_sender: Option<std::sync::mpsc::Sender<LspEvent>>,
    /// LSP 服务管理器（仅非 wasm32 平台）
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) lsp_manager: Option<LspServiceManager>,
    /// LSP 覆盖层状态（仅非 wasm32 平台）
    #[cfg(not(target_arch = "wasm32"))]
    pub lsp_overlay: iced_code_editor::LspOverlayState,
    /// 是否禁用 LSP（仅非 wasm32 平台）
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) lsp_disabled: bool,
    /// 是否正在应用 LSP 补全（仅非 wasm32 平台）
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) lsp_applying_completion: bool,
    /// LSP 悬停锚点（文件路径，位置）（仅非 wasm32 平台）
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) lsp_hover_anchor: Option<(String, iced_code_editor::LspPosition)>,
    /// LSP 覆盖层路径（仅非 wasm32 平台）
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) lsp_overlay_path: Option<String>,
    /// 待处理的 LSP 悬停信息（仅非 wasm32 平台）
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) lsp_hover_pending: Option<LspHoverPending>,
    /// LSP 悬停隐藏截止时间（仅非 wasm32 平台）
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) lsp_hover_hide_deadline: Option<std::time::Instant>,
    /// LSP 进度映射（文件路径 -> 进度 ID -> 进度信息）（仅非 wasm32 平台）
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) lsp_progress: HashMap<String, HashMap<String, LspProgress>>,
    /// LSP 状态信息（仅非 wasm32 平台）
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) lsp_status: Option<String>,
    /// 待跳转的预览位置（文件路径，行号，列号）
    pub(crate) pending_preview_goto: Option<(String, usize, usize)>,
    /// 预览导航回退栈
    pub(crate) preview_trace_back: Vec<(String, usize, usize)>,
    /// 预览导航前进栈
    pub(crate) preview_trace_forward: Vec<(String, usize, usize)>,
    /// 是否正在导航预览
    pub(crate) preview_trace_navigating: bool,
    /// 上一个预览路径
    pub(crate) previous_preview_path: Option<String>,
    /// 当前聚焦区域
    pub(crate) focus_area: FocusArea,
    /// 是否显示预览上下文菜单
    pub(crate) show_preview_context_menu: bool,
    /// 预览上下文菜单目标（文件路径，起始行，起始列，结束行，结束列）
    pub(crate) preview_context_target: Option<(String, usize, usize, usize, usize)>,
    /// 预览上下文菜单位置
    pub(crate) preview_context_menu_pos: Option<(f32, f32)>,
    /// 预览导航弹出框（文件路径，X，Y，选项列表）
    pub(crate) preview_nav_popup: Option<(String, f32, f32, Vec<(String, bool)>)>,
    /// 知识库工作台状态
    pub knowledge: KnowledgeUiState,
    /// Dify 工作流无限画布状态
    pub workflow_state: crate::apps::workflow::state::WorkflowState,
    /// 错误消息
    pub error_message: Option<String>,
    /// 鼠标光标位置
    pub cursor_position: Point,
    /// 是否显示预览设置
    pub(crate) show_preview_settings: bool,
    /// 是否显示预览右上角全屏浮动控件
    pub(crate) show_preview_fullscreen_overlay: bool,
    /// 当前字体大小
    pub(crate) current_font_size: f32,
    /// 当前行高
    pub(crate) current_line_height: f32,
    /// 是否自动调整行高
    pub(crate) auto_adjust_line_height: bool,
    /// 预览编辑器自动保存模式
    pub(crate) preview_auto_save_mode: crate::app::PreviewAutoSaveMode,
    /// 当前语言设置
    pub(crate) current_language: iced_code_editor::i18n::Language,
    /// 待丢弃的文件路径
    pub(crate) file_to_discard: Option<String>,
    /// 当前活跃的标签页 ID
    pub(crate) active_tab_id: Option<String>,
    /// 悬停的标签页 ID
    pub(crate) hovered_tab_id: Option<String>,
    /// 悬停的最近项目路径
    pub(crate) hovered_recent_project: Option<String>,
    /// 打开的标签页列表
    pub(crate) open_tabs: Vec<AppTab>,
    /// 应用搜索查询
    pub apps_search_query: String,
    /// 网页书签列表
    pub web_bookmarks: Vec<WebBookmark>,
    /// 是否显示网页链接菜单
    pub show_web_links_menu: bool,
    /// 网页书签标题输入
    pub web_bookmark_title_input: String,
    /// 网页书签 URL 输入
    pub web_bookmark_url_input: String,
    /// 网页书签宽度输入
    pub web_bookmark_width_input: String,
    /// 网页书签高度输入
    pub web_bookmark_height_input: String,
    /// 正在编辑的网页书签索引
    pub editing_web_bookmark: Option<usize>,
    /// 编辑网页书签标题输入
    pub edit_web_bookmark_title_input: String,
    /// 编辑网页书签 URL 输入
    pub edit_web_bookmark_url_input: String,
    /// 编辑网页书签宽度输入
    pub edit_web_bookmark_width_input: String,
    /// 编辑网页书签高度输入
    pub edit_web_bookmark_height_input: String,
    /// 编辑网页书签 Cookie 配置编辑器
    pub edit_web_bookmark_cookie_configs_editor: text_editor::Content,
    /// 独立 WebView 子进程列表（仅非 wasm32 平台）
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) independent_webview_children: Vec<std::process::Child>,
    /// 加载动画帧索引
    pub spinner_frame: usize,
    /// 通知列表
    pub(crate) notifications: Vec<Notification>,
    /// 通知面板是否展开
    pub(crate) notifications_expanded: bool,
    /// 下一个通知 ID
    pub(crate) next_notification_id: usize,
    /// 通知滚动容器 ID
    pub(crate) notifications_scroll_id: Id,
    /// 通知消息编辑器映射（通知 ID -> 编辑器内容）
    pub(crate) notification_editors: HashMap<usize, text_editor::Content>,
    /// 最近点击复制的通知 ID
    pub(crate) copied_notification_id: Option<usize>,
    /// 当前显示的轻量提示
    pub(crate) active_toast: Option<Toast>,
    /// 下一个轻量提示 ID
    pub(crate) next_toast_id: usize,
    /// Shiju 本地多账户与登录状态。
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) account: AccountState,
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) expert_hub: crate::app::skill_hub::ExpertHubState,
    /// 工作流管理面板状态
    pub workflow_manager: crate::app::views::workflow_manager::WorkflowManagerState,
    /// 是否显示任务看板
    pub show_task_board: bool,
    /// 任务看板是否正在加载
    pub task_board_loading: bool,
    /// 任务看板当前视图模式
    pub task_board_view_mode: TaskBoardViewMode,
    /// 任务看板任务列表
    pub task_board_tasks: Vec<crate::app::task::Task>,
    /// 任务看板员工配置
    pub task_board_staff_config: crate::app::task::TaskBoardStaffConfig,
    /// 任务看板员工配置所属 workflow
    pub task_board_staff_config_workflow_id: Option<String>,
    /// 任务看板员工投影
    pub task_board_staff_projection: crate::app::task::staff::TaskBoardStaffProjection,
    /// 任务看板员工配置加载错误
    pub task_board_staff_config_load_error: Option<String>,
    /// 任务看板员工能力目录
    pub task_board_staff_capability_catalog: Vec<crate::app::task::staff::TaskBoardStaffCapability>,
    /// 任务看板员工能力目录所属 workflow
    pub task_board_staff_capability_catalog_workflow_id: Option<String>,
    /// 任务看板员工能力目录编辑草稿
    pub task_board_staff_capability_draft: Vec<crate::app::task::staff::TaskBoardStaffCapability>,
    /// 任务看板员工能力目录是否正在加载
    pub task_board_staff_capability_catalog_loading: bool,
    /// 任务看板员工能力目录是否正在保存
    pub task_board_staff_capability_saving: bool,
    /// 任务看板员工能力目录加载或保存错误
    pub task_board_staff_capability_catalog_error: Option<String>,
    /// 任务看板员工命令队列
    pub task_board_staff_command_queue: crate::app::task::staff::TaskBoardStaffCommandQueue,
    /// 任务看板员工事件流
    pub task_board_staff_events: Vec<crate::app::task::staff::TaskBoardStaffEvent>,
    /// 任务看板员工任务流当前标签页
    pub task_board_staff_feed_tab: TaskBoardStaffFeedTab,
    /// 任务看板员工任务状态筛选
    pub task_board_staff_task_status_filter: Vec<crate::app::task::TaskStatus>,
    /// 任务看板员工工作流输入
    pub task_board_staff_command_input: String,
    /// 任务看板员工模式当前筛选的成员 ID
    pub task_board_staff_filter_member_id: Option<String>,
    /// 任务看板员工阶段分配面板是否打开
    pub task_board_staff_assignment_open: bool,
    /// 任务看板员工阶段分配编辑草稿
    pub task_board_staff_assignment_draft:
        Option<Vec<crate::app::task::staff::TaskBoardStageAssignment>>,
    /// 任务看板员工阶段 Workflow 绑定编辑草稿
    pub task_board_staff_workflow_binding_draft:
        Option<Vec<crate::app::task::staff::TaskBoardStageWorkflowBinding>>,
    /// 任务看板员工阶段能力动作编辑草稿
    pub task_board_staff_stage_action_draft:
        Option<Vec<crate::app::task::staff::models::TaskBoardStageAction>>,
    /// 任务看板员工阶段能力重置是否等待二次确认
    pub task_board_staff_stage_action_reset_confirm: bool,
    /// 任务看板员工阶段可绑定的已保存 Workflow
    pub task_board_staff_workflow_options: Vec<vw_gateway_client::WorkflowRecordSummary>,
    /// 任务看板员工阶段 Workflow 列表加载错误
    pub task_board_staff_workflow_options_error: Option<String>,
    /// 任务看板员工阶段分配各状态当前选择标签页
    pub task_board_staff_assignment_tabs:
        HashMap<crate::app::task::TaskStatus, TaskBoardStaffAssignmentTab>,
    /// 任务看板员工阶段责任编排当前选择标签页
    pub task_board_staff_orchestration_tab: TaskBoardStaffOrchestrationTab,
    /// 任务看板创建对话框是否打开
    pub task_board_create_modal_open: bool,
    /// 任务看板草稿
    pub task_board_draft: crate::app::task::TaskDraft,
    /// 任务看板上次使用的模型
    pub task_board_last_model: String,
    /// 任务看板上次使用的 ACP 智能体
    pub task_board_last_acp_agent: Option<String>,
    /// 任务看板批量选中的任务 ID 集合
    pub task_board_selected_tasks: HashSet<String>,
    /// 当前启用批量操作的任务状态列
    pub task_board_bulk_active_status: Option<crate::app::task::TaskStatus>,
    /// 任务看板批量设置优先级输入
    pub task_board_bulk_priority_input: String,
    /// 任务看板批量设置模型输入
    pub task_board_bulk_model_input: String,
    /// 任务看板批量设置代理
    pub task_board_bulk_agent: String,
    /// 任务看板批量设置 ACP 智能体
    pub task_board_bulk_acp_agent: Option<String>,
    /// 任务看板批量设置推理强度
    pub task_board_bulk_reasoning_effort:
        Option<vw_gateway_client::vw_api_types::ReasoningEffortDto>,
    /// 任务看板批量设置任务拆分模式
    pub task_board_bulk_task_split_mode: crate::app::task::TaskSplitMode,
    /// 任务看板手动批量合并队列
    pub task_board_manual_merge_queue: Vec<String>,
    /// 任务看板选中的任务 ID
    pub task_board_selected_task: Option<String>,
    /// 任务看板正在查看日志的任务
    pub task_board_viewing_logs: Option<crate::app::task::Task>,
    /// 任务看板详情面板当前标签页
    pub task_board_detail_tab: TaskBoardDetailTab,
    /// 任务看板按任务 ID 缓存的内存日志
    pub task_board_log_cache: HashMap<String, Vec<crate::app::task::TaskLogEntry>>,
    /// 任务看板正在编辑的任务 ID
    pub task_board_editing_task_id: Option<String>,
    /// 任务看板拖动中的任务（任务 ID，状态）
    pub task_board_dragging: Option<(String, crate::app::task::TaskStatus)>,
    /// 任务看板待放置的任务（任务 ID，目标状态，位置）
    pub task_board_drag_pending: Option<(String, crate::app::task::TaskStatus, iced::Point)>,
    /// 任务看板状态过滤器
    pub task_board_filter_status: Option<crate::app::task::TaskStatus>,
    /// 任务看板优先级过滤器（最小值，最大值）
    pub task_board_filter_priority: Option<(u32, u32)>,
    /// 任务看板是否按优先级排序
    pub task_board_sort_by_priority: bool,
    /// 任务看板是否升序排序
    pub task_board_sort_ascending: bool,
    /// 任务看板设置
    pub task_board_settings: crate::app::task::TaskBoardSettings,
    /// 任务看板执行器状态
    pub task_board_executor: crate::app::task::TaskExecutorState,
    /// 任务看板执行器是否正在运行
    pub task_board_executor_running: bool,
    /// 任务看板下次刷新时间（毫秒时间戳）
    pub task_board_next_refresh_at_ms: u64,
    /// 任务看板下次调度器触发时间（毫秒时间戳）
    pub task_board_next_scheduler_tick_at_ms: u64,
    /// 任务看板下次自动审核触发时间（毫秒时间戳）
    pub task_board_next_auto_review_tick_at_ms: u64,
    /// 任务看板下次自动晋升触发时间（毫秒时间戳）
    pub task_board_next_auto_promote_tick_at_ms: u64,
    /// 任务看板下次自动合并触发时间（毫秒时间戳）
    pub task_board_next_auto_merge_tick_at_ms: u64,
    /// gateway 任务池调度返回的 worktree 目标容量
    pub task_board_worktree_desired_capacity: Option<u32>,
    /// Matrix 任务池入站事件游标
    pub task_board_matrix_event_cursor: u64,
    /// Matrix 任务池已处理事件 ID 环形记录
    pub task_board_processed_matrix_event_ids: VecDeque<String>,
    /// 任务看板上次日志刷新时间（毫秒时间戳）
    pub task_board_last_log_flush_at_ms: u64,
    /// 任务看板日志扫描游标
    pub task_board_log_scan_cursor: usize,
    /// 任务看板超时扫描游标
    pub task_board_timeout_scan_cursor: usize,
    /// 任务看板调度扫描游标
    pub task_board_schedule_scan_cursor: usize,
    /// 任务看板执行器弹出框是否打开
    pub task_board_executor_popover: bool,
    /// 任务看板批量执行器弹出框是否打开
    pub task_board_bulk_executor_popover: bool,
    /// 任务看板批量模型弹出框是否打开
    pub task_board_bulk_model_popover: bool,
    /// 任务看板 worktree 维护是否进行中
    pub task_board_worktree_maintenance_in_flight: bool,
    /// 任务看板 worktree 快照是否正在加载
    pub task_board_worktree_snapshot_loading: bool,
    /// 任务看板手动 worktree 操作类型（cleanup / merge）
    pub task_board_worktree_manual_action_kind: Option<&'static str>,
    /// 任务看板手动 worktree 二次确认操作类型（cleanup / merge）
    pub task_board_worktree_manual_confirm_kind: Option<&'static str>,
    /// 任务看板手动 worktree 操作日志
    pub task_board_worktree_action_logs: Vec<String>,
    /// 任务看板手动 worktree 操作日志自动隐藏截止时间（毫秒时间戳）
    pub task_board_worktree_action_logs_visible_until_ms: Option<u64>,
    /// 任务看板手动 worktree 操作日志接收器
    pub task_board_worktree_action_log_rx:
        Option<std::sync::mpsc::Receiver<crate::app::task::executor::TaskLogStream>>,
    /// 任务看板 worktree 池快照
    pub task_board_worktree_snapshot: Option<crate::app::task::WorktreePoolSnapshot>,
    /// 任务看板上次 worktree 快照时间（毫秒时间戳）
    pub task_board_last_worktree_snapshot_at_ms: u64,
    /// 任务看板 worktree 面板是否展开
    pub task_board_worktree_panel_expanded: bool,
    /// 任务看板是否显示像素办公室
    pub task_board_worktree_pixel_office: bool,
    /// 任务看板上下文菜单（任务 ID，X，Y）
    pub task_board_context_menu: Option<(String, f32, f32)>,
    /// 任务看板新建子任务内容
    pub task_board_new_subtask_content: String,
    /// 任务看板描述编辑器
    pub task_board_desc_editor: text_editor::Content,
    /// 任务看板提示词编辑器
    pub task_board_prompt_editor: text_editor::Content,
    /// 任务看板日志查看器（只读，可选择复制）
    pub task_board_logs_editor: text_editor::Content,
    /// 任务看板日志编辑器 ID
    pub task_board_logs_editor_id: Id,
    /// 任务看板日志是否自动跟随到底部
    pub task_board_logs_auto_scroll: bool,
    /// 任务看板日志右键菜单是否打开
    pub task_board_logs_context_menu_open: bool,
    /// 任务看板日志右键菜单位置
    pub task_board_logs_context_menu_pos: Option<(f32, f32)>,
    /// 任务看板日志当前顶部滚动行号
    pub task_board_logs_scroll_top_line: f32,
    /// 任务看板日志滚轮累积的小数行偏移
    pub task_board_logs_scroll_remainder: f32,
    /// 任务看板日志编辑器视口高度
    pub task_board_logs_viewport_height: f32,
    /// 任务看板模型选择弹出框是否打开
    pub task_board_model_popover: bool,
    /// 任务看板创建后是否清除提示词
    pub task_board_clear_prompt_after_create: bool,
    /// 任务看板创建后是否关闭对话框
    pub task_board_close_after_create: bool,
    /// 任务看板编辑后是否关闭详情面板
    pub task_board_close_after_edit: bool,
    /// 任务看板创建是否成功
    pub task_board_create_submit_success: bool,
    /// 任务看板编辑是否成功
    pub task_board_edit_submit_success: bool,
    /// 任务看板设置对话框是否打开
    pub task_board_settings_modal_open: bool,
    /// 任务看板设置对话框当前标签页
    pub task_board_settings_modal_tab: TaskBoardSettingsModalTab,
    /// 任务看板是否为导入模式
    pub task_board_is_import_mode: bool,
    /// 任务看板导入编辑器
    pub task_board_import_editor: text_editor::Content,
    /// 任务看板导入提示词格式
    pub task_board_import_prompt_format: crate::app::task::TaskImportPromptFormat,
    /// 任务看板导入提示词面板是否折叠
    pub task_board_import_prompt_collapsed: bool,
    /// 任务看板每个状态列是否显示纵向滚动条
    pub task_board_column_has_vertical_scrollbar: HashMap<crate::app::task::TaskStatus, bool>,
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
