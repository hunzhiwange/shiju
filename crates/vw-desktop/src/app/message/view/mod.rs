//! 视图层消息处理模块
//!
//! 本模块负责处理与用户界面视图相关的所有消息事件。它定义了视图层的消息类型枚举
//! 以及对应的消息处理逻辑。模块采用路由分发模式，根据消息类型将处理委托给不同的
//! 子模块（panel、layout、theme）完成。
//!
//! ## 主要职责
//!
//! - 定义菜单类型枚举 [`MenuType`]，用于标识不同的菜单分类
//! - 定义视图消息枚举 [`ViewMessage`]，包含所有视图层可能产生的事件
//! - 提供 [`update`] 函数，作为视图消息的统一入口和分发器
//!
//! ## 模块结构
//!
//! - [`layout`] - 处理窗口布局、拖拽、调整大小等与界面布局相关的消息
//! - [`panel`] - 处理面板切换、弹窗、工具窗口等面板相关的消息
//! - [`theme`] - 处理应用主题选择和切换相关的消息

use crate::app::{
    App, Message,
    state::{
        ChatSendBehavior, ExternalOpenApp, ModelPopoverHover, TopBarGatewayTab, UsageModelInfo,
        WorkResourceMode, WorkSidebarFilterScope, WorkSidebarFilterStatus, WorkSidebarFilterTime,
        WorkSidebarTab, WorkspaceMode,
    },
};
use iced::{Task, Theme, Vector};

pub mod layout;
pub mod panel;
pub mod theme;

#[cfg(not(target_arch = "wasm32"))]
fn task_pet_window_settings(size: iced::Size, position: iced::Point) -> iced::window::Settings {
    iced::window::Settings {
        size,
        // 禁止用户缩放，但允许展开动画和伙伴大小设置调整原生窗口。
        min_size: None,
        max_size: None,
        position: iced::window::Position::Specific(position),
        resizable: false,
        decorations: false,
        transparent: true,
        level: iced::window::Level::AlwaysOnTop,
        exit_on_close_request: false,
        ..Default::default()
    }
}

/// 仅由加载和显式显示操作同步窗口；保存响应不改变窗口生命周期。
pub(crate) fn sync_task_pet_window(app: &mut App) -> Task<Message> {
    #[cfg(target_arch = "wasm32")]
    {
        let _ = app;
        Task::none()
    }
    #[cfg(not(target_arch = "wasm32"))]
    match (app.task_pet_visible, app.task_pet_window_id) {
        (true, None) => {
            let (id, open) = iced::window::open(task_pet_window_settings(
                app.task_pet_window_size(),
                app.task_pet_position,
            ));
            app.task_pet_window_id = Some(id);
            open.map(|_| Message::None)
        }
        (false, Some(id)) => iced::window::close(id),
        _ => Task::none(),
    }
}

fn open_task_pet_session(app: &mut App, session_id: String) -> Task<Message> {
    if app.active_session_id.as_deref() == Some(session_id.as_str()) {
        return Task::none();
    }

    let path = app.known_session_directory(&session_id).unwrap_or_default();
    Task::done(Message::Project(
        crate::app::message::project::ProjectMessage::OpenProjectSessionPressed(path, session_id),
    ))
}

/// 菜单类型枚举
///
/// 定义应用程序顶部菜单栏中的各个菜单分类。用于在菜单操作和悬停事件中
/// 标识具体的菜单项，以便进行相应的状态管理和界面响应。
#[derive(Debug, Clone, PartialEq, Eq, Copy)]
pub enum MenuType {
    /// 文件菜单 - 包含项目、会话的新建、打开、保存等操作
    File,
    /// 编辑菜单 - 包含撤销、重做、剪切、复制、粘贴等编辑操作
    Edit,
    /// 视图菜单 - 包含面板显示/隐藏、主题切换等视图相关操作
    View,
    /// 帮助菜单 - 包含关于、文档、反馈等帮助相关操作
    Help,
    /// 外部链接菜单 - 用于在外部浏览器或其他应用中打开链接
    OpenExternal,
    /// 网关服务菜单 - 显示服务端与客户端网关入口
    GatewayServices,
}

/// 视图层消息枚举
///
/// 定义了所有与用户界面视图交互相关的消息类型。这些消息涵盖了：
/// - 面板和弹窗的切换与控制
/// - 文件管理器操作
/// - Web 书签管理
/// - 使用量统计显示
/// - 窗口布局调整
/// - 主题切换
/// - 各种工具窗口的打开
///
/// 消息通过 [`update`] 函数处理，并根据类型分发给对应的子模块。
#[derive(Debug, Clone)]
pub enum ViewMessage {
    /// 切换设置面板的显示/隐藏状态
    ToggleSettingsPanel,
    /// 创建新的会话
    ProjectFileNewSession,
    /// 创建新的项目
    ProjectFileNewProject,
    /// 显示会话列表
    ProjectFileShowSessions,
    /// 显示项目列表
    ProjectFileShowProjects,
    /// 保存所有打开的文件
    ProjectFileSaveAll,
    /// 切换系统设置面板
    ToggleSystemSettings,
    /// 打开系统设置面板的指定标签页
    ///
    /// # 参数
    /// - 标签页标识，指定要打开的系统设置标签
    OpenAcpSettings,
    OpenSystemSettingsTab(crate::app::components::system_settings::SystemTab),
    /// 打开系统设置中的模型详情页
    ///
    /// # 参数
    /// - 第一个 String: 模型提供商标识
    /// - 第二个 String: 模型标识
    OpenSystemSettingsModelDetail(String, String),
    /// 切换"关于"弹窗的显示/隐藏状态
    ToggleAboutModal,
    /// 请求重启应用程序
    RestartApp,
    /// 应用程序重启完成
    ///
    /// # 参数
    /// - `Ok(())`: 重启成功
    /// - `Err(String)`: 重启失败，包含错误信息
    RestartAppFinished(Result<(), String>),
    /// 请求安装 CLI 工具
    InstallCliTool,
    #[cfg(not(target_arch = "wasm32"))]
    WebInstall(crate::app::web_install::Event),
    RunInstallCliTool,
    CheckCliToolUpdate,
    CliCommandsTabSelected(crate::app::components::cli_command_help::CliCommandsTab),
    CheckCliToolUpdateFinished(u64, vw_shared::update::UpdateSource, Result<String, String>),
    OpenAppUpdateModal,
    CheckAppUpdate,
    AutoCheckAppUpdate,
    AutoCheckAppUpdateFinished(
        u64,
        vw_shared::update::UpdateSource,
        Result<vw_shared::update::DesktopRelease, String>,
    ),
    SkipAppUpdate,
    SkipAppUpdateSaved(crate::app::desktop_update::SkippedRelease, Result<(), String>),
    CheckAppUpdateFinished(
        u64,
        vw_shared::update::UpdateSource,
        Result<vw_shared::update::DesktopRelease, String>,
    ),
    AppUpdateSourceSelected(vw_shared::update::UpdateSource),
    AppUpdateSourceSaved(Result<(), String>),
    #[cfg(not(target_arch = "wasm32"))]
    AppUpdatePrepared(Result<crate::app::installer_update::PreparedDesktopUpdate, String>),
    #[cfg(not(target_arch = "wasm32"))]
    AppUpdateDownloadPrepared(u64, Result<crate::app::installer_update::PreparedDesktopUpdate, String>),
    CancelAppUpdate,
    OpenAppInstaller,
    OpenAppInstallerDirectory,
    AppInstallerDirectoryOpened(Result<(), String>),
    RunAppUpdate,
    AppUpdateProgressTick,
    AppUpdateFinished(Result<String, String>),
    /// CLI 工具安装完成
    ///
    /// # 参数
    /// - `Ok(String)`: 安装成功，包含安装路径
    /// - `Err(String)`: 安装失败，包含错误信息
    InstallCliToolFinished(Result<String, String>),
    /// 关闭 CLI 安装弹窗
    CloseInstallCliModal,
    /// 打开使用量统计视图
    OpenUsage,
    /// 推进通用状态动画帧
    ActivityAnimationTick,
    /// 开启或关闭独立小宠物窗口
    TaskPetToggleWindow,
    /// 幂等显示或隐藏 Mini；关闭回调前保留窗口 ID。
    TaskPetShowWindow,
    TaskPetHideWindow,
    /// 聚焦主窗口当前会话输入，不创建会话或发送内容。
    TaskPetWriteMessage,
    /// 打开小宠物右键菜单
    TaskPetContextMenuOpened(f32, f32),
    /// 关闭小宠物右键菜单
    TaskPetContextMenuClosed,
    /// 切换小宠物任务面板收起状态
    TaskPetToggleCollapsed,
    /// 开始拖拽小宠物任务面板
    TaskPetDragStarted,
    /// 小宠物本体悬停状态改变
    TaskPetRobotHover(bool),
    /// 切换小宠物形态
    TaskPetAvatarCycle,
    /// 从 Mini 打开常规设置中的伙伴页面。
    TaskPetOpenSettings,
    /// 选择伙伴头像，不改变 Mini 显示偏好。
    TaskPetAvatarSelected(crate::app::state::TaskPetAvatarKind),
    /// 更新并保存伙伴比例。
    TaskPetScaleChanged(f32),
    TaskPetSizeOpen,
    TaskPetSizePreview(f32),
    TaskPetSizeReset,
    TaskPetSizeCancel,
    TaskPetSizeSave,
    TaskPetSizeSaved(crate::app::config::PreferencesScope, Result<(), String>),
    /// 点击小宠物任务条目
    TaskPetItemClicked(u64),
    /// 主动删除小宠物任务条目
    TaskPetRemove(u64),
    /// 小宠物任务条目悬停状态改变
    TaskPetHover(Option<u64>),
    /// 展开小宠物任务回复输入
    TaskPetReplyPressed(u64),
    /// 小宠物任务回复输入变化
    TaskPetReplyInputChanged(String),
    /// 提交小宠物任务回复
    TaskPetReplySubmit,
    /// 切换指定菜单的展开/折叠状态
    ///
    /// # 参数
    /// - `Some(MenuType)`: 展开指定菜单
    /// - `None`: 关闭所有菜单
    ToggleMenu(Option<MenuType>),
    /// 执行菜单项对应的操作
    ///
    /// # 参数
    /// - 内部消息，封装了具体要执行的操作
    MenuAction(Box<Message>),
    /// 切换模型选择弹窗
    ToggleModelPopover,
    /// 切换输入面板模型提供商分组的折叠状态。
    ToggleInputModelProvider(String),
    /// 切换模式选择弹窗
    ToggleModePopover,
    /// 切换代码/工作界面模式弹窗
    ToggleWorkspaceModePopover,
    /// 关闭代码/工作界面模式弹窗
    CloseWorkspaceModePopover,
    /// 选择代码/工作界面模式
    WorkspaceModeSelected(WorkspaceMode),
    /// 选择工作模式右侧资源区的呈现模式
    WorkResourceModeSelected(WorkResourceMode),
    /// 隐藏或展示工作模式右侧工作空间模块
    WorkResourcesToggled,
    /// 收起或展开工作模式左栏
    WorkSidebarToggled,
    /// 返回当前项目的工作模式起始页。
    WorkProjectHome,
    /// 打开或关闭工作模式左栏过滤弹层
    WorkSidebarFilterToggled,
    /// 关闭工作模式左栏过滤弹层
    WorkSidebarFilterClosed,
    /// 打开工作模式左栏“更多”应用菜单
    WorkSidebarMoreOpened,
    /// 关闭工作模式左栏“更多”应用菜单
    WorkSidebarMoreClosed,
    /// 悬停或离开工作模式左栏“项目”导航项
    WorkSidebarProjectNavHovered(bool),
    /// 选择工作模式左栏的项目范围过滤
    WorkSidebarFilterScopeSelected(WorkSidebarFilterScope),
    /// 选择工作模式左栏的会话运行状态过滤
    WorkSidebarFilterStatusSelected(WorkSidebarFilterStatus),
    /// 选择工作模式左栏的更新时间过滤
    WorkSidebarFilterTimeSelected(WorkSidebarFilterTime),
    /// 重置工作模式左栏过滤条件
    WorkSidebarFilterReset,
    /// 切换工作模式左栏的任务/空间标签
    WorkSidebarTabSelected(WorkSidebarTab),
    /// 为当前工作模式左栏标签加载更多项目
    WorkSidebarLoadMorePressed,
    /// 为指定空间加载更多任务
    WorkSidebarSpaceTasksLoadMorePressed(String),
    /// 打开或关闭输入区空间选择器
    WorkSpacePickerToggled,
    /// 关闭输入区空间选择器
    WorkSpacePickerClosed,
    /// 更新工作模式对话区占可用内容宽度的比例
    WorkSplitRatioChanged(f32),
    /// 开始拖动工作模式左栏宽度
    WorkSidebarDragStarted,
    /// 开始拖动工作模式聊天/资源分割条
    WorkSplitDragStarted,
    /// 切换工作模式资源区全屏
    WorkResourceFullscreenToggled,
    /// 切换工作模式资源区半屏
    WorkResourceHalfFullscreenToggled,
    /// 按真实项目路径切换工作模式空间展开状态
    WorkSpaceToggled(String),
    /// 更新停靠浏览器地址输入
    WorkBrowserAddressChanged(String),
    /// 导航停靠浏览器到当前地址
    WorkBrowserNavigate,
    /// 停靠浏览器后退
    WorkBrowserBack,
    /// 停靠浏览器前进
    WorkBrowserForward,
    /// 刷新停靠浏览器
    WorkBrowserReload,
    /// 关闭当前停靠浏览并进入空白态
    WorkBrowserClose,
    /// 接收停靠 WebView 回传的最新导航地址
    WorkBrowserRuntimeTick,
    /// 切换发送模式弹窗
    ToggleSendModePopover,
    /// 切换文件选择弹窗
    ToggleFilePopover,
    /// 切换 ACP 选择弹窗
    ToggleAcpPopover,
    /// 切换推理强度选择弹窗
    ToggleReasoningEffortPopover,
    /// 切换使用量详情弹窗
    ToggleUsagePopover,
    /// 切换会话工具选择弹窗
    ToggleSessionToolSelectorPopover,
    /// 切换会话操作弹窗
    ToggleSessionActionsPopover,
    /// 切换执行器选择弹窗
    ToggleExecutorPopover,
    /// 关闭所有弹窗
    ClosePopovers,
    /// 关闭模型选择弹窗
    CloseModelPopover,
    /// 关闭模式选择弹窗
    CloseModePopover,
    /// 关闭发送模式弹窗
    CloseSendModePopover,
    /// 关闭文件选择弹窗
    CloseFilePopover,
    /// 关闭 ACP 选择弹窗
    CloseAcpPopover,
    /// 关闭推理强度选择弹窗
    CloseReasoningEffortPopover,
    /// 关闭使用量详情弹窗
    CloseUsagePopover,
    /// 关闭执行器选择弹窗
    CloseExecutorPopover,
    /// 模型选择弹窗中的悬停状态改变
    ///
    /// # 参数
    /// - 当前悬停的模型项，None 表示无悬停
    ModelPopoverHoverChanged(Option<ModelPopoverHover>),
    /// 选择发送模式
    SelectChatSendBehavior(ChatSendBehavior),
    /// 切换差异对比面板的显示/隐藏
    ToggleDiffPanel,
    /// 切换 Git 差异摘要面板的显示/隐藏
    ToggleGitDiffSummary,
    /// 切换终端面板的显示/隐藏
    ToggleTerminalPanel,
    /// 文件管理器面板可见性改变
    ///
    /// # 参数
    /// - 新的可见性状态
    FileManagerPanelVisible(bool),
    /// 首页应用栏滚动位置改变
    ///
    /// # 参数
    /// - 新的滚动偏移量
    HomeAppsBarScrollChanged(f32),
    /// 首页应用栏向左滚动（上一页）
    HomeAppsBarPrev,
    /// 首页应用栏向右滚动（下一页）
    HomeAppsBarNext,
    /// 自动最大化设置切换
    ///
    /// # 参数
    /// - 新的自动最大化状态
    AutoMaxToggled(bool),
    /// 全局鼠标释放事件（用于结束拖拽操作）
    GlobalMouseReleased,
    /// 鼠标光标离开窗口区域
    GlobalCursorLeft,
    /// 设置面板拖拽开始
    SettingsDragStarted,
    /// 文件管理器面板拖拽开始
    FileManagerDragStarted,
    /// 分割条拖拽开始
    SplitDragStarted,
    /// 鼠标指针移动
    ///
    /// # 参数
    /// - x 坐标
    /// - y 坐标
    PointerMoved(f32, f32),
    /// 系统拖入的文件悬停在窗口内
    HoveredFilePath(String),
    /// 未由文件树消费的原生投递。
    DroppedFilePath(String),
    /// 系统拖入的文件离开窗口
    HoveredFilesLeft,
    /// 窗口大小改变
    ///
    /// # 参数
    /// - 新的宽度
    /// - 新的高度
    WindowResized(iced::window::Id, f32, f32),
    /// 窗口关闭完成
    WindowClosed(iced::window::Id),
    /// 窗口获得焦点
    WindowFocused(iced::window::Id),
    /// 窗口失去焦点
    WindowUnfocused(iced::window::Id),
    /// 全屏布局稳定完成
    FullscreenLayoutSettled,
    /// 窗口位置移动
    ///
    /// # 参数
    /// - 新的 x 坐标
    /// - 新的 y 坐标
    WindowMoved(iced::window::Id, f32, f32),
    /// 全局未捕获键盘按下事件
    GlobalKeyPressed(iced::keyboard::Key, iced::keyboard::Modifiers),
    TraverseFormFocus(iced::window::Id, bool),
    /// 返回首页
    GoHome,
    /// 选择应用主题
    ///
    /// # 参数
    /// - 选中的主题实例
    AppThemeSelected(Theme),
    /// 打开终端按钮按下
    OpenTerminalPressed,
    /// 窗口拖拽区域按下（用于自定义标题栏拖拽）
    WindowDragPressed,
    /// 选中指定标签页
    ///
    /// # 参数
    /// - 标签页标识
    TabSelected(String),
    /// 关闭指定标签页
    ///
    /// # 参数
    /// - 标签页标识
    TabClosed(String),
    /// 菜单项悬停
    ///
    /// # 参数
    /// - 悬停的菜单类型
    MenuHovered(MenuType),
    /// 标签页悬停状态改变
    ///
    /// # 参数
    /// - 悬停的标签页标识，None 表示无悬停
    TabHovered(Option<String>),
    /// 打开应用中心
    OpenApps,
    /// 打开知识库工作台
    OpenKnowledge,
    /// 打开 Dify 工作流无限画布工具
    OpenWorkflowTool,
    /// 打开任务工作流管理面板
    OpenWorkflowManager,
    /// 打开任务工作流管理面板并切换到阶段能力
    /// 打开最近使用的应用
    AppsOpenMostRecent,
    /// 应用中心搜索内容改变
    ///
    /// # 参数
    /// - 新的搜索关键词
    AppsSearchChanged(String),
    /// 在 Web 视图中打开指定 URL
    ///
    /// # 参数
    /// - 要打开的 URL
    OpenWebUrl(String),
    /// 在 Web 视图中打开指定 URL 并设置标题
    ///
    /// # 参数
    /// - URL
    /// - 页面标题
    OpenWebUrlWithTitle(String, String),
    /// 在外部浏览器中打开指定 URL
    ///
    /// # 参数
    /// - 要打开的 URL
    OpenUrlExternal(String),
    /// 切换 Web 链接菜单的显示/隐藏
    ToggleWebLinksMenu,
    /// 选择顶栏网关服务弹窗标签页
    GatewayServicesTabSelected(TopBarGatewayTab),
    /// Web 书签标题输入改变
    ///
    /// # 参数
    /// - 新的标题内容
    WebBookmarkTitleChanged(String),
    /// Web 书签 URL 输入改变
    ///
    /// # 参数
    /// - 新的 URL 内容
    WebBookmarkUrlChanged(String),
    /// Web 书签宽度输入改变
    ///
    /// # 参数
    /// - 新的宽度值（字符串形式）
    WebBookmarkWidthChanged(String),
    /// Web 书签高度输入改变
    ///
    /// # 参数
    /// - 新的高度值（字符串形式）
    WebBookmarkHeightChanged(String),
    /// 保存新增的 Web 书签
    WebBookmarkAddSave,
    /// 取消新增 Web 书签
    WebBookmarkAddCancel,
    /// 开始编辑指定索引的 Web 书签
    ///
    /// # 参数
    /// - 书签在列表中的索引
    WebBookmarkEditStart(usize),
    /// 编辑中的 Web 书签标题改变
    ///
    /// # 参数
    /// - 新的标题内容
    WebBookmarkEditTitleChanged(String),
    /// 编辑中的 Web 书签 URL 改变
    ///
    /// # 参数
    /// - 新的 URL 内容
    WebBookmarkEditUrlChanged(String),
    /// 编辑中的 Web 书签宽度改变
    ///
    /// # 参数
    /// - 新的宽度值（字符串形式）
    WebBookmarkEditWidthChanged(String),
    /// 编辑中的 Web 书签高度改变
    ///
    /// # 参数
    /// - 新的高度值（字符串形式）
    WebBookmarkEditHeightChanged(String),
    /// 编辑中的 Web 书签 Cookie 配置改变
    ///
    /// # 参数
    /// - 文本编辑器的操作动作
    WebBookmarkEditCookieConfigsChanged(iced::widget::text_editor::Action),
    /// 向编辑中的 Web 书签 Cookie 配置编辑器插入示例内容
    WebBookmarkEditCookieConfigsInsertExample,
    /// 保存编辑中的 Web 书签
    WebBookmarkEditSave,
    /// 取消编辑 Web 书签
    WebBookmarkEditCancel,
    /// 删除指定索引的 Web 书签
    ///
    /// # 参数
    /// - 书签在列表中的索引
    WebBookmarkRemove(usize),
    /// 在 Web 视图中打开 URL，并设置标题和窗口尺寸
    ///
    /// # 参数
    /// - URL
    /// - 页面标题
    /// - 可选的窗口宽度
    /// - 可选的窗口高度
    OpenWebUrlWithTitleAndSize(String, String, Option<i32>, Option<i32>),
    /// 窗口关闭请求（用户点击关闭按钮）
    CloseRequested(iced::window::Id),
    /// 有界关机清理已经完成或超时
    ShutdownFinished,
    /// 使用量统计中的模型信息加载完成
    ///
    /// # 参数
    /// - 加载的模型信息，None 表示加载失败或无数据
    UsageModelInfoLoaded(Option<UsageModelInfo>),
    /// 使用量步骤展开/折叠切换
    ///
    /// # 参数
    /// - 步骤索引
    UsageStepToggled(u32),
    /// 按稳定 turn 身份展开或折叠使用量步骤。
    UsageTurnStepToggled(String, u32),
    /// 使用量步骤原始 LLM 载荷加载完成
    ///
    /// # 参数
    /// - 会话 ID
    /// - 步骤索引
    /// - 原始载荷读取结果
    UsageStepRawLoaded(String, u32, Result<Option<serde_json::Value>, String>),
    /// 按稳定 turn 身份完成原始 LLM 载荷加载。
    UsageTurnStepRawLoaded(String, String, u32, Result<Option<serde_json::Value>, String>),
    /// 打开使用量步骤原始记录弹窗
    ///
    /// # 参数
    /// - 步骤索引
    UsageStepRawOpen(u32),
    /// 按稳定 turn 身份打开使用量步骤原始记录弹窗。
    UsageTurnStepRawOpen(String, u32),
    /// 关闭使用量步骤原始记录弹窗
    UsageStepRawClose,
    UsageCenter(crate::app::views::usage::center::Event),
    /// 打开聊天中点击的绝对路径。
    OpenChatPath(String),
    ChatPathKindResolved {
        path: String,
        result: Result<vw_gateway_client::vw_api_types::file::FileNodeKind, String>,
    },
    /// 在系统文件管理器中打开指定路径
    ///
    /// # 参数
    /// - 要打开的文件系统路径
    OpenPathInFinder(String),
    /// 数据网关完成文件管理器定位。
    PathRevealFinished {
        path: String,
        result: Result<(), String>,
    },
    /// 使用首选外部应用打开项目
    OpenProjectInExternalPreferred,
    /// 使用指定外部应用打开项目
    ///
    /// # 参数
    /// - 外部应用标识
    OpenProjectInExternalWith(ExternalOpenApp),
    /// 复制项目路径到剪贴板
    CopyProjectPath,
    /// 从快照重放会话
    ///
    /// # 参数
    /// - 快照文件路径
    ReplaySessionFromSnapshot(String),
}

/// 视图消息更新处理函数
///
/// 作为视图层消息的统一入口，根据消息类型将处理委托给相应的子模块。
/// 采用路由分发模式，将不同类型的消息分类到不同的处理逻辑中。
///
/// # 消息分发策略
///
/// 消息被分为三大类：
/// 1. **面板类消息** - 委托给 [`panel::update`] 处理
///    - 设置面板、系统设置、菜单、弹窗等界面组件的切换和操作
///    - 文件管理器、终端、差异对比等面板的控制
///    - 各种工具窗口的打开和关闭
///    - Web 书签的管理操作
///
/// 2. **布局类消息** - 委托给 [`layout::update`] 处理
///    - 窗口拖拽、调整大小、位置移动等布局操作
///    - 各种面板的拖拽调整
///    - 鼠标移动和释放事件
///
/// 3. **主题类消息** - 委托给 [`theme::update`] 处理
///    - 应用主题的选择和切换
///
/// # 参数
///
/// - `app`: 应用状态的可变引用，用于读取和修改应用状态
/// - `message`: 要处理的视图消息
///
/// # 返回值
///
/// 返回一个 [`Task<Message>`]，可能包含需要执行的异步命令或后续消息。
/// 如果不需要执行额外操作，返回 `Task::none()`。
///
/// # 示例
///
/// ```ignore
/// let task = update(&mut app, ViewMessage::ToggleSettingsPanel);
/// // task 可能包含需要在事件循环中执行的命令
/// ```
pub fn update(app: &mut App, message: ViewMessage) -> Task<Message> {
    if matches!(&message,
        ViewMessage::WorkspaceModeSelected(_) | ViewMessage::WorkProjectHome
        | ViewMessage::ToggleSystemSettings | ViewMessage::TaskPetOpenSettings
        | ViewMessage::OpenAcpSettings | ViewMessage::OpenSystemSettingsTab(_)
        | ViewMessage::OpenSystemSettingsModelDetail(_, _)
        | ViewMessage::GoHome | ViewMessage::TabSelected(_) | ViewMessage::TabClosed(_)) {
        app.usage_center.close();
        app.apps_catalog.external_sessions.cancel_reader();
    }

    if app.task_pet_size_draft.is_some()
        && matches!(
            &message,
            ViewMessage::TaskPetAvatarSelected(_)
                | ViewMessage::TaskPetAvatarCycle
                | ViewMessage::TaskPetScaleChanged(_)
                | ViewMessage::TaskPetDragStarted
                | ViewMessage::TaskPetShowWindow
                | ViewMessage::TaskPetHideWindow
                | ViewMessage::TaskPetToggleWindow
                | ViewMessage::TaskPetWriteMessage
                | ViewMessage::TaskPetToggleCollapsed
                | ViewMessage::TaskPetContextMenuOpened(_, _)
                | ViewMessage::TaskPetReplyPressed(_)
                | ViewMessage::TaskPetReplySubmit
                | ViewMessage::TaskPetItemClicked(_)
                | ViewMessage::TaskPetRemove(_)
        )
    {
        return Task::none();
    }

    let sync_work_browser = matches!(
        &message,
        ViewMessage::WorkspaceModeSelected(_)
            | ViewMessage::WorkResourceModeSelected(_)
            | ViewMessage::WorkResourcesToggled
            | ViewMessage::WorkSidebarToggled
            | ViewMessage::WorkProjectHome
            | ViewMessage::WorkSplitRatioChanged(_)
            | ViewMessage::WorkResourceFullscreenToggled
            | ViewMessage::WorkResourceHalfFullscreenToggled
            | ViewMessage::ToggleSystemSettings
            | ViewMessage::TaskPetOpenSettings
            | ViewMessage::OpenAcpSettings
            | ViewMessage::OpenSystemSettingsTab(_)
            | ViewMessage::OpenSystemSettingsModelDetail(_, _)
            | ViewMessage::GoHome
            | ViewMessage::TabSelected(_)
            | ViewMessage::TabClosed(_)
            | ViewMessage::WindowResized(_, _, _)
            | ViewMessage::WindowMoved(_, _, _)
            | ViewMessage::WindowFocused(_)
            | ViewMessage::WindowUnfocused(_)
            | ViewMessage::FullscreenLayoutSettled
            | ViewMessage::WorkBrowserRuntimeTick
    ) || matches!(
        &message,
        ViewMessage::ToggleTerminalPanel if app.workspace_mode == crate::app::state::WorkspaceMode::Work
    ) || matches!(
        &message,
        ViewMessage::PointerMoved(_, _)
            if app.dragging_work_sidebar || app.dragging_work_split
    ) || matches!(
        &message,
        ViewMessage::GlobalMouseReleased
            if app.dragging_work_sidebar || app.dragging_work_split
    );
    let task = match message {
        #[cfg(not(target_arch = "wasm32"))]
        msg @ ViewMessage::WebInstall(_) => panel::update(app, msg),
        #[cfg(not(target_arch = "wasm32"))]
        msg @ ViewMessage::AppUpdatePrepared(_) => panel::update(app, msg),
        #[cfg(not(target_arch = "wasm32"))]
        msg @ ViewMessage::AppUpdateDownloadPrepared(_, _) => panel::update(app, msg),
        ViewMessage::GlobalKeyPressed(key, modifiers) => handle_global_key(app, key, modifiers),
        ViewMessage::TraverseFormFocus(window, reverse) => {
            if app.task_pet_window_id == Some(window)
                || app.main_window_id.is_some_and(|id| id != window)
            {
                Task::none()
            } else if app.show_system_settings {
                crate::app::components::system_settings::traverse_select_focus(reverse)
            } else if app.file_manager_show_changes
                && app.show_git_diff_summary
                && !app.show_git_commit_help_modal
                && !app.show_git_filter_help_modal
                && !app.show_git_custom_diff_modal
                && !app.show_git_copy_modal
                && app.file_to_discard.is_none()
            {
                crate::app::components::git_panel::traverse_commit_focus(reverse)
            } else {
                Task::none()
            }
        }
        ViewMessage::ActivityAnimationTick => {
            app.advance_status_animation_frame();
            if app.task_pet_visible
                && let Some(window_id) = app.task_pet_window_id
                && let Some((size, position)) = app.advance_task_pet_expand_animation()
            {
                iced::window::resize(window_id, size)
                    .chain(iced::window::move_to(window_id, position))
            } else {
                Task::none()
            }
        }
        ViewMessage::TaskPetToggleWindow => {
            let intent = if app.task_pet_visible {
                ViewMessage::TaskPetHideWindow
            } else {
                ViewMessage::TaskPetShowWindow
            };
            update(app, intent)
        }
        ViewMessage::TaskPetShowWindow => {
            #[cfg(target_arch = "wasm32")]
            {
                return app.show_error_toast("当前平台不支持独立伙伴窗口");
            }
            #[cfg(not(target_arch = "wasm32"))]
            {
                app.task_pet_context_menu_pos = None;
                app.task_pet_visible = true;
                let open = sync_task_pet_window(app);
                let focus =
                    app.task_pet_window_id.map_or_else(Task::none, iced::window::gain_focus);
                Task::batch([open.chain(focus), app.save_task_pet_preferences()])
            }
        }
        ViewMessage::TaskPetHideWindow => {
            app.task_pet_context_menu_pos = None;
            app.task_pet_visible = false;
            app.task_pet_robot_hovered = false;
            Task::batch([sync_task_pet_window(app), app.save_task_pet_preferences()])
        }
        ViewMessage::CloseRequested(id) if app.task_pet_window_id == Some(id) => {
            update(app, ViewMessage::TaskPetHideWindow)
        }
        ViewMessage::WindowClosed(id) if app.task_pet_window_id == Some(id) => {
            app.task_pet_window_id = None;
            sync_task_pet_window(app)
        }
        ViewMessage::TaskPetWriteMessage => {
            app.task_pet_context_menu_pos = None;
            app.show_system_settings = false;
            app.show_settings = false;
            app.screen = crate::app::Screen::Project;
            #[cfg(not(target_arch = "wasm32"))]
            let focus = app.main_window_id.map_or_else(Task::none, |id| {
                iced::window::minimize(id, false).chain(iced::window::gain_focus(id))
            });
            #[cfg(target_arch = "wasm32")]
            let focus = Task::none();
            focus.chain(iced::widget::operation::focus(app.input_editor_id.clone()))
        }
        ViewMessage::TaskPetContextMenuOpened(x, y) => {
            app.task_pet_context_menu_pos = Some((x, y));
            Task::none()
        }
        ViewMessage::TaskPetContextMenuClosed => {
            app.task_pet_context_menu_pos = None;
            Task::none()
        }
        ViewMessage::TaskPetToggleCollapsed => {
            app.task_pet_context_menu_pos = None;
            app.toggle_task_pet_collapsed();
            Task::none()
        }
        ViewMessage::TaskPetDragStarted => {
            if let Some(window_id) = app.task_pet_window_id {
                app.pulse_task_pet_motion();
                iced::window::drag(window_id)
            } else {
                app.start_task_pet_drag();
                Task::none()
            }
        }
        ViewMessage::TaskPetRobotHover(hovered) => {
            app.set_task_pet_robot_hovered(hovered);
            Task::none()
        }
        ViewMessage::TaskPetOpenSettings => {
            app.task_pet_context_menu_pos = None;
            app.general_settings_tab = crate::app::state::GeneralSettingsTab::Companion;
            app.system_settings_tab = crate::app::components::system_settings::SystemTab::General;
            let open = update(
                app,
                ViewMessage::OpenSystemSettingsTab(
                    crate::app::components::system_settings::SystemTab::General,
                ),
            );
            let focus = app.main_window_id.map_or_else(Task::none, |id| {
                iced::window::set_mode(id, iced::window::Mode::Windowed)
                    .chain(iced::window::gain_focus(id))
            });
            Task::batch([open, focus])
        }
        ViewMessage::TaskPetSizeOpen => {
            if app.task_pet_size_draft.is_some() {
                return Task::none();
            }
            app.task_pet_size_draft = Some(app.task_pet_scale);
            app.task_pet_dragging = false;
            update(app, ViewMessage::TaskPetOpenSettings)
        }
        ViewMessage::TaskPetSizePreview(scale) => {
            if app.task_pet_size_draft.is_none()
                || app.task_pet_size_pending.is_some()
                || app.task_pet_avatar_kind == crate::app::state::TaskPetAvatarKind::None
            {
                return Task::none();
            }
            if !scale.is_finite() {
                return app.show_error_toast("伙伴大小必须是有限数值");
            }
            let previous_size = app.task_pet_window_size();
            app.task_pet_size_draft = Some(scale.clamp(0.5, 2.0));
            app.resize_task_pet_preview(previous_size)
        }
        ViewMessage::TaskPetSizeReset => update(app, ViewMessage::TaskPetSizePreview(1.0)),
        ViewMessage::TaskPetSizeCancel => {
            if app.task_pet_size_pending.is_some() {
                return Task::none();
            }
            let previous_size = app.task_pet_window_size();
            app.task_pet_size_draft = None;
            app.resize_task_pet_preview(previous_size)
        }
        ViewMessage::TaskPetSizeSave => {
            if app.task_pet_size_pending.is_some() {
                return Task::none();
            }
            let Some(scale) = app.task_pet_size_draft else {
                return Task::none();
            };
            if scale == app.task_pet_scale {
                return update(app, ViewMessage::TaskPetSizeCancel);
            }
            app.save_task_pet_size(scale)
        }
        ViewMessage::TaskPetSizeSaved(scope, result) => {
            if app.task_pet_size_pending.as_ref() == Some(&scope) {
                app.task_pet_size_pending = None;
                if result.is_ok()
                    && let Some(scale) = app.task_pet_size_draft.take()
                {
                    app.task_pet_scale = scale;
                }
            }
            app.desktop_preferences_saved(scope, result)
        }
        ViewMessage::TaskPetAvatarCycle => {
            app.cycle_task_pet_avatar();
            app.save_task_pet_preferences()
        }
        ViewMessage::TaskPetAvatarSelected(avatar) => {
            app.task_pet_avatar_kind = avatar;
            app.save_task_pet_preferences()
        }
        ViewMessage::TaskPetScaleChanged(scale) => match app.set_task_pet_scale(scale) {
            Ok(()) => app.save_task_pet_preferences(),
            Err(error) => app.show_error_toast(error),
        },
        ViewMessage::TaskPetItemClicked(request_id) => {
            if let Some(session_id) = app.task_pet_item_clicked(request_id) {
                open_task_pet_session(app, session_id)
            } else {
                Task::none()
            }
        }
        ViewMessage::TaskPetRemove(request_id) => {
            app.dismiss_task_pet_item(request_id);
            Task::none()
        }
        ViewMessage::TaskPetHover(request_id) => {
            app.set_task_pet_hovered(request_id);
            Task::none()
        }
        ViewMessage::TaskPetReplyPressed(request_id) => {
            app.open_task_pet_reply(request_id);
            Task::none()
        }
        ViewMessage::TaskPetReplyInputChanged(value) => {
            app.update_task_pet_reply_input(value);
            Task::none()
        }
        ViewMessage::TaskPetReplySubmit => {
            let Some((session_id, input)) = app.take_task_pet_reply() else {
                return Task::none();
            };
            Task::done(Message::Chat(crate::app::message::ChatMessage::SendToSession {
                session_id,
                input,
            }))
        }
        // 面板相关消息：包括设置面板、系统设置、菜单、弹窗、工具窗口等
        // 这些消息统一委托给 panel 子模块处理
        ViewMessage::ToggleSettingsPanel
        | ViewMessage::ProjectFileNewSession
        | ViewMessage::ProjectFileNewProject
        | ViewMessage::ProjectFileShowSessions
        | ViewMessage::ProjectFileShowProjects
        | ViewMessage::ProjectFileSaveAll
        | ViewMessage::ToggleSystemSettings
        | ViewMessage::OpenAcpSettings
        | ViewMessage::OpenSystemSettingsTab(_)
        | ViewMessage::OpenSystemSettingsModelDetail(_, _)
        | ViewMessage::ToggleAboutModal
        | ViewMessage::RestartApp
        | ViewMessage::RestartAppFinished(_)
        | ViewMessage::InstallCliTool
        | ViewMessage::RunInstallCliTool
        | ViewMessage::CheckCliToolUpdate
        | ViewMessage::CliCommandsTabSelected(_)
        | ViewMessage::CheckCliToolUpdateFinished(..)
        | ViewMessage::OpenAppUpdateModal
        | ViewMessage::CheckAppUpdate
        | ViewMessage::AutoCheckAppUpdate
        | ViewMessage::AutoCheckAppUpdateFinished(..)
        | ViewMessage::SkipAppUpdate
        | ViewMessage::SkipAppUpdateSaved(..)
        | ViewMessage::CheckAppUpdateFinished(..)
        | ViewMessage::AppUpdateSourceSelected(_)
        | ViewMessage::AppUpdateSourceSaved(_)
        | ViewMessage::OpenAppInstaller
        | ViewMessage::OpenAppInstallerDirectory
        | ViewMessage::AppInstallerDirectoryOpened(_)
        | ViewMessage::CancelAppUpdate
        | ViewMessage::RunAppUpdate
        | ViewMessage::AppUpdateProgressTick
        | ViewMessage::AppUpdateFinished(_)
        | ViewMessage::InstallCliToolFinished(_)
        | ViewMessage::CloseInstallCliModal
        | ViewMessage::OpenUsage
        | ViewMessage::ToggleMenu(_)
        | ViewMessage::MenuAction(_)
        | ViewMessage::ToggleModelPopover
        | ViewMessage::ToggleInputModelProvider(_)
        | ViewMessage::ToggleModePopover
        | ViewMessage::ToggleWorkspaceModePopover
        | ViewMessage::CloseWorkspaceModePopover
        | ViewMessage::WorkspaceModeSelected(_)
        | ViewMessage::ToggleFilePopover
        | ViewMessage::ToggleAcpPopover
        | ViewMessage::ToggleReasoningEffortPopover
        | ViewMessage::ToggleUsagePopover
        | ViewMessage::ToggleSessionToolSelectorPopover
        | ViewMessage::ToggleSendModePopover
        | ViewMessage::ToggleSessionActionsPopover
        | ViewMessage::ToggleExecutorPopover
        | ViewMessage::ClosePopovers
        | ViewMessage::CloseModelPopover
        | ViewMessage::CloseModePopover
        | ViewMessage::CloseFilePopover
        | ViewMessage::CloseAcpPopover
        | ViewMessage::CloseReasoningEffortPopover
        | ViewMessage::CloseUsagePopover
        | ViewMessage::CloseSendModePopover
        | ViewMessage::CloseExecutorPopover
        | ViewMessage::ModelPopoverHoverChanged(_)
        | ViewMessage::GoHome
        | ViewMessage::AutoMaxToggled(_)
        | ViewMessage::WorkResourceModeSelected(_)
        | ViewMessage::WorkResourcesToggled
        | ViewMessage::WorkSidebarToggled
        | ViewMessage::WorkProjectHome
        | ViewMessage::WorkSidebarFilterToggled
        | ViewMessage::WorkSidebarFilterClosed
        | ViewMessage::WorkSidebarMoreOpened
        | ViewMessage::WorkSidebarMoreClosed
        | ViewMessage::WorkSidebarProjectNavHovered(_)
        | ViewMessage::WorkSidebarFilterScopeSelected(_)
        | ViewMessage::WorkSidebarFilterStatusSelected(_)
        | ViewMessage::WorkSidebarFilterTimeSelected(_)
        | ViewMessage::WorkSidebarFilterReset
        | ViewMessage::WorkSidebarTabSelected(_)
        | ViewMessage::WorkSidebarLoadMorePressed
        | ViewMessage::WorkSidebarSpaceTasksLoadMorePressed(_)
        | ViewMessage::WorkSpacePickerToggled
        | ViewMessage::WorkSpacePickerClosed
        | ViewMessage::WorkSplitRatioChanged(_)
        | ViewMessage::WorkResourceFullscreenToggled
        | ViewMessage::WorkResourceHalfFullscreenToggled
        | ViewMessage::WorkSpaceToggled(_)
        | ViewMessage::WorkBrowserAddressChanged(_)
        | ViewMessage::WorkBrowserNavigate
        | ViewMessage::WorkBrowserBack
        | ViewMessage::WorkBrowserForward
        | ViewMessage::WorkBrowserReload
        | ViewMessage::WorkBrowserClose
        | ViewMessage::WorkBrowserRuntimeTick
        | ViewMessage::SelectChatSendBehavior(_) => panel::update(app, message),
        ViewMessage::ToggleDiffPanel
        | ViewMessage::ToggleGitDiffSummary
        | ViewMessage::ToggleTerminalPanel
        | ViewMessage::FileManagerPanelVisible(_)
        | ViewMessage::HomeAppsBarScrollChanged(_)
        | ViewMessage::HomeAppsBarPrev
        | ViewMessage::HomeAppsBarNext
        | ViewMessage::OpenTerminalPressed
        | ViewMessage::TabSelected(_)
        | ViewMessage::TabClosed(_)
        | ViewMessage::MenuHovered(_)
        | ViewMessage::TabHovered(_)
        | ViewMessage::OpenApps
        | ViewMessage::OpenKnowledge
        | ViewMessage::OpenWorkflowTool
        | ViewMessage::OpenWorkflowManager
        | ViewMessage::AppsOpenMostRecent
        | ViewMessage::AppsSearchChanged(_)
        | ViewMessage::OpenWebUrl(_)
        | ViewMessage::OpenWebUrlWithTitle(_, _)
        | ViewMessage::OpenWebUrlWithTitleAndSize(_, _, _, _)
        | ViewMessage::OpenUrlExternal(_)
        | ViewMessage::UsageModelInfoLoaded(_)
        | ViewMessage::UsageStepToggled(_)
        | ViewMessage::UsageTurnStepToggled(_, _)
        | ViewMessage::UsageStepRawLoaded(_, _, _)
        | ViewMessage::UsageTurnStepRawLoaded(_, _, _, _)
        | ViewMessage::UsageStepRawOpen(_)
        | ViewMessage::UsageTurnStepRawOpen(_, _)
        | ViewMessage::UsageStepRawClose
        | ViewMessage::UsageCenter(_)
        | ViewMessage::OpenChatPath(_)
        | ViewMessage::ChatPathKindResolved { .. }
        | ViewMessage::OpenPathInFinder(_)
        | ViewMessage::PathRevealFinished { .. }
        | ViewMessage::OpenProjectInExternalPreferred
        | ViewMessage::OpenProjectInExternalWith(_)
        | ViewMessage::CopyProjectPath
        | ViewMessage::ReplaySessionFromSnapshot(_)
        | ViewMessage::ToggleWebLinksMenu
        | ViewMessage::GatewayServicesTabSelected(_)
        | ViewMessage::WebBookmarkTitleChanged(_)
        | ViewMessage::WebBookmarkUrlChanged(_)
        | ViewMessage::WebBookmarkWidthChanged(_)
        | ViewMessage::WebBookmarkHeightChanged(_)
        | ViewMessage::WebBookmarkAddSave
        | ViewMessage::WebBookmarkAddCancel
        | ViewMessage::WebBookmarkEditStart(_)
        | ViewMessage::WebBookmarkEditTitleChanged(_)
        | ViewMessage::WebBookmarkEditUrlChanged(_)
        | ViewMessage::WebBookmarkEditWidthChanged(_)
        | ViewMessage::WebBookmarkEditHeightChanged(_)
        | ViewMessage::WebBookmarkEditCookieConfigsChanged(_)
        | ViewMessage::WebBookmarkEditCookieConfigsInsertExample
        | ViewMessage::WebBookmarkEditSave
        | ViewMessage::WebBookmarkEditCancel
        | ViewMessage::WebBookmarkRemove(_)
        | ViewMessage::CloseRequested(_)
        | ViewMessage::ShutdownFinished => panel::update(app, message),

        // 布局相关消息：包括窗口拖拽、调整大小、鼠标移动等
        // 这些消息统一委托给 layout 子模块处理
        ViewMessage::GlobalMouseReleased
        | ViewMessage::GlobalCursorLeft
        | ViewMessage::SettingsDragStarted
        | ViewMessage::FileManagerDragStarted
        | ViewMessage::SplitDragStarted
        | ViewMessage::WorkSidebarDragStarted
        | ViewMessage::WorkSplitDragStarted
        | ViewMessage::PointerMoved(_, _)
        | ViewMessage::DroppedFilePath(_)
        | ViewMessage::HoveredFilePath(_)
        | ViewMessage::HoveredFilesLeft
        | ViewMessage::WindowResized(_, _, _)
        | ViewMessage::WindowClosed(_)
        | ViewMessage::WindowFocused(_)
        | ViewMessage::WindowUnfocused(_)
        | ViewMessage::FullscreenLayoutSettled
        | ViewMessage::WindowMoved(_, _, _)
        | ViewMessage::WindowDragPressed => layout::update(app, message),

        // 主题相关消息：包括主题选择和切换
        // 这些消息统一委托给 theme 子模块处理
        ViewMessage::AppThemeSelected(_) => theme::update(app, message),
    };

    if sync_work_browser { Task::batch(vec![task, panel::sync_work_browser(app)]) } else { task }
}

fn handle_global_key(
    app: &mut App,
    key: iced::keyboard::Key,
    modifiers: iced::keyboard::Modifiers,
) -> Task<Message> {
    if app.task_pet_size_draft.is_some() {
        return if key == iced::keyboard::Key::Named(iced::keyboard::key::Named::Escape) {
            update(app, ViewMessage::TaskPetSizeCancel)
        } else {
            Task::none()
        };
    }
    if app.screen == crate::app::Screen::WorkflowTool {
        if let Some(workflow_message) =
            workflow_key_message(&key, modifiers, app.workflow_state.selected_node_id.is_some())
        {
            return crate::apps::workflow::update(app, workflow_message);
        }
    }

    if let Some(message) = global_command_message(&key, modifiers) {
        return Task::done(message);
    }

    Task::none()
}

fn workflow_key_message(
    key: &iced::keyboard::Key,
    modifiers: iced::keyboard::Modifiers,
    has_selected_node: bool,
) -> Option<crate::apps::workflow::WorkflowMessage> {
    if modifiers.command() {
        return match key {
            iced::keyboard::Key::Character(c) if c == "+" || c == "=" => {
                Some(crate::apps::workflow::WorkflowMessage::Zoom(1.1, None))
            }
            iced::keyboard::Key::Character(c) if c == "-" => {
                Some(crate::apps::workflow::WorkflowMessage::Zoom(1.0 / 1.1, None))
            }
            iced::keyboard::Key::Character(c) if c == "0" => {
                Some(crate::apps::workflow::WorkflowMessage::ZoomFit)
            }
            iced::keyboard::Key::Character(c) if c.eq_ignore_ascii_case("o") => {
                Some(crate::apps::workflow::WorkflowMessage::OpenFile)
            }
            iced::keyboard::Key::Character(c) if c.eq_ignore_ascii_case("r") => {
                Some(crate::apps::workflow::WorkflowMessage::Reload)
            }
            _ => None,
        };
    }

    match key {
        iced::keyboard::Key::Named(iced::keyboard::key::Named::ArrowLeft) => {
            Some(crate::apps::workflow::WorkflowMessage::PanBy(Vector::new(-48.0, 0.0)))
        }
        iced::keyboard::Key::Named(iced::keyboard::key::Named::ArrowRight) => {
            Some(crate::apps::workflow::WorkflowMessage::PanBy(Vector::new(48.0, 0.0)))
        }
        iced::keyboard::Key::Named(iced::keyboard::key::Named::ArrowUp) => {
            Some(crate::apps::workflow::WorkflowMessage::PanBy(Vector::new(0.0, -48.0)))
        }
        iced::keyboard::Key::Named(iced::keyboard::key::Named::ArrowDown) => {
            Some(crate::apps::workflow::WorkflowMessage::PanBy(Vector::new(0.0, 48.0)))
        }
        iced::keyboard::Key::Named(iced::keyboard::key::Named::Backspace)
        | iced::keyboard::Key::Named(iced::keyboard::key::Named::Delete) => {
            if has_selected_node {
                Some(crate::apps::workflow::WorkflowMessage::DeleteSelectedNode)
            } else {
                Some(crate::apps::workflow::WorkflowMessage::DeleteSelectedEdge)
            }
        }
        iced::keyboard::Key::Named(iced::keyboard::key::Named::Escape) => {
            Some(crate::apps::workflow::WorkflowMessage::CancelInteraction)
        }
        _ => None,
    }
}

fn global_command_message(
    key: &iced::keyboard::Key,
    modifiers: iced::keyboard::Modifiers,
) -> Option<Message> {
    if !modifiers.command() {
        return None;
    }

    match key {
        iced::keyboard::Key::Character(c) if c.eq_ignore_ascii_case("p") => {
            Some(Message::Search(crate::app::message::SearchMessage::Open))
        }
        iced::keyboard::Key::Character(c) if c.eq_ignore_ascii_case("c") => {
            Some(Message::CopyShortcut)
        }
        _ => None,
    }
}
#[cfg(test)]
mod tests;
