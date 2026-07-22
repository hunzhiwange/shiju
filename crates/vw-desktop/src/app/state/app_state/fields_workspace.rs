    /// 项目任务池自动执行 tick 间隔秒数输入
    pub(crate) project_edit_task_board_auto_promote_tick_interval_seconds_input: String,
    /// 项目失败重试分钟数输入
    pub(crate) project_edit_failed_retry_minutes_input: String,
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
    pub(crate) json_tool_editor_id: Id,
    pub(crate) sql_tool_editor_id: Id,
    pub(crate) html_tool_editor_id: Id,
    pub(crate) json_diff_left_editor_id: Id,
    pub(crate) json_diff_right_editor_id: Id,
    pub(crate) json_yaml_left_editor_id: Id,
    pub(crate) json_yaml_right_editor_id: Id,
    pub(crate) markdown_tool_editor_id: Id,
    pub(crate) pwd_editor_id: Id,
    /// 文件搜索滚动容器 ID
    pub(crate) file_search_scroll_id: Id,
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
    /// Composio 设置状态
    pub(crate) composio_settings: ComposioSettingsState,
    /// 转录设置状态
    pub(crate) transcription_settings: TranscriptionSettingsState,
    /// 最近项目编辑列表
    pub(crate) recent_projects_edits: Vec<String>,
    /// 确认删除的最近项目索引
    pub(crate) recent_project_delete_confirm_idx: Option<usize>,
    /// 对话流权限编辑器内容
    pub(crate) dialogue_flow_permission_editor: text_editor::Content,
    pub(crate) dialogue_flow_show_reasoning_summary: bool,
    pub(crate) dialogue_flow_expand_shell_tool_section: bool,
    pub(crate) dialogue_flow_expand_edit_tool_section: bool,
    pub(crate) chat_send_behavior: ChatSendBehavior,
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
    /// 正在拖动的文件路径
    pub(crate) dragging_file_path: Option<String>,
    /// 正在拖动文件的插入位置（兄弟索引，子索引）
    pub(crate) dragging_file_position: Option<(usize, usize)>,
    /// 待放置的文件路径
    pub(crate) pending_drop_file_path: Option<String>,
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
