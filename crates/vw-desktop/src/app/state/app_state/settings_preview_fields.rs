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
    /// HTTP 请求设置状态
    pub(crate) http_request_settings: HttpRequestSettingsState,
    /// 网关设置状态
    pub(crate) gateway_settings: GatewaySettingsState,
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
