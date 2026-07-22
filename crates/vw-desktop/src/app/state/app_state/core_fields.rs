    /// 当前显示的屏幕
    pub(crate) screen: Screen,
    /// 当前项目路径
    pub(crate) project_path: Option<String>,
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
    pub(crate) acp_agent: Option<String>,
    pub(crate) acp_history_mode: AcpHistoryReplayMode,
    pub(crate) acp_recent_count: usize,
    pub(crate) acp_agents: Vec<String>,
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
    /// 使用量面板当前会话对应的 SQLite 文件路径
    pub(crate) usage_session_file_path: Option<String>,
    /// 展开的使用量步骤索引集合
    pub(crate) usage_step_expanded: HashSet<u32>,
    /// 使用量步骤原始 LLM 载荷缓存
    pub(crate) usage_step_raw_payloads: HashMap<UsageStepRawKey, UsageStepRawPayloadResult>,
    /// 正在加载原始 LLM 载荷的使用量步骤
    pub(crate) usage_step_raw_loading: HashSet<UsageStepRawKey>,
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
