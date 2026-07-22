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
    /// 是否显示文件选择弹出框
    pub(crate) show_file_popover: bool,
    /// 是否显示 ACP 选择弹出框
    pub(crate) show_acp_popover: bool,
    /// 是否显示使用量弹出框
    pub(crate) show_usage_popover: bool,
    /// 是否显示会话工具选择弹出框
    pub(crate) show_session_tool_selector_popover: bool,
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
    /// 任务看板员工配置所属 workflow
    pub(crate) task_board_staff_config_workflow_id: Option<String>,
    /// 任务看板员工能力目录
    pub(crate) task_board_staff_capability_catalog:
        Vec<crate::app::task::staff::TaskBoardStaffCapability>,
    /// 任务看板员工能力目录所属 workflow
    pub(crate) task_board_staff_capability_catalog_workflow_id: Option<String>,
    /// 任务看板员工能力目录编辑草稿
    pub(crate) task_board_staff_capability_draft:
        Vec<crate::app::task::staff::TaskBoardStaffCapability>,
    /// 任务看板员工能力目录是否正在加载
    pub(crate) task_board_staff_capability_catalog_loading: bool,
    /// 任务看板员工能力目录是否正在保存
    pub(crate) task_board_staff_capability_saving: bool,
    /// 任务看板员工能力目录加载或保存错误
    pub(crate) task_board_staff_capability_catalog_error: Option<String>,
