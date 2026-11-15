use iced::widget::text_editor::Action;
use iced_code_editor::Message as EditorMessage;
/// 预览模块消息枚举
///
/// 定义了文件预览器支持的所有用户交互与系统事件。
/// 消息按功能领域分组，由 [`update`] 函数路由到对应处理器。
#[derive(Debug, Clone)]
pub enum PreviewMessage {
    /// 打开指定路径的文件
    ///
    /// # 参数
    /// - `String`: 文件绝对路径
    Open(String),

    /// 在同一编辑器中打开只读快照，不读取或写入工作区文件。
    OpenSnapshot {
        key: String,
        path: String,
        title: String,
        content: String,
    },

    /// 文件内容加载完成
    ///
    /// 由异步加载任务在完成后发送，携带文件内容。
    ///
    /// # 参数
    /// - `path`: 文件路径
    /// - `content`: 文件内容
    /// - `truncated`: 内容是否因过大而被截断
    OpenLoaded {
        path: String,
        content: String,
        truncated: bool,
    },
    OpenFailed {
        path: String,
        error: String,
    },

    /// 切换到指定路径的标签页（已打开则聚焦，否则打开）
    ///
    /// # 参数
    /// - `String`: 文件路径
    Select(String),

    /// 关闭指定路径的标签页
    ///
    /// # 参数
    /// - `String`: 文件路径
    Close(String),

    /// 搜索框内容变更
    ///
    /// # 参数
    /// - `String`: 当前搜索文本
    SearchChanged(String),

    /// 跳转到下一个搜索匹配
    SearchNext,

    /// 跳转到上一个搜索匹配
    SearchPrev,

    /// 跳转行号输入框内容变更
    ///
    /// # 参数
    /// - `String`: 用户输入的行号文本
    GotoLineChanged(String),

    /// 提交跳转行号请求
    GotoLineSubmit,

    /// 选择模式 - 起始行号变更
    ///
    /// # 参数
    /// - `String`: 起始行号文本
    SelectStartLineChanged(String),

    /// 选择模式 - 起始列号变更
    ///
    /// # 参数
    /// - `String`: 起始列号文本
    SelectStartColChanged(String),

    /// 选择模式 - 结束行号变更
    ///
    /// # 参数
    /// - `String`: 结束行号文本
    SelectEndLineChanged(String),

    /// 选择模式 - 结束列号变更
    ///
    /// # 参数
    /// - `String`: 结束列号文本
    SelectEndColChanged(String),

    /// 切换选择模式的开关状态
    ///
    /// # 参数
    /// - `bool`: 是否启用选择模式
    ToggleSelectMode(bool),

    /// 可编辑区域的编辑器动作
    ///
    /// 用于主编辑区的文本操作。
    ///
    /// # 参数
    /// - `Action`: iced 文本编辑器动作
    SelectionEditorAction(Action),

    /// 只读区域的编辑器动作
    ///
    /// 用于预览区的只读文本操作。
    ///
    /// # 参数
    /// - `Action`: iced 文本编辑器动作
    ReadOnlyEditorAction(Action),

    /// 代码编辑器内部事件
    ///
    /// 来自 iced_code_editor 的底层事件。
    ///
    /// # 参数
    /// - `EditorMessage`: 编辑器事件
    EditorEvent(EditorMessage),

    /// 切换搜索面板的显示/隐藏状态
    SearchToggle,

    /// 搜索输入框的编辑器动作
    ///
    /// # 参数
    /// - `Action`: iced 文本编辑器动作
    SearchEditorAction(Action),

    /// 复制所有搜索匹配的文本到剪贴板
    CopySearchMatches,

    /// 在指定位置打开上下文菜单
    ///
    /// # 参数
    /// - `String`: 目标文件路径
    /// - `usize`: 菜单关联的行号
    /// - `usize`: 菜单关联的列号
    /// - `f32`: 菜单 X 坐标（像素）
    /// - `f32`: 菜单 Y 坐标（像素）
    ContextMenuOpen(String, usize, usize, f32, f32),

    /// 为当前活动编辑器打开上下文菜单
    ///
    /// # 参数
    /// - `f32`: 菜单 X 坐标
    /// - `f32`: 菜单 Y 坐标
    ContextMenuOpenForActiveEditor(f32, f32),

    /// 执行上下文菜单的复制操作
    ContextMenuCopy,

    /// 执行上下文菜单的剪切操作
    ContextMenuCut,

    /// 执行上下文菜单的粘贴操作
    ContextMenuPaste,

    /// 执行上下文菜单的删除操作
    ContextMenuDelete,

    /// 关闭上下文菜单
    ContextMenuClose,

    /// 鼠标进入预览右上角全屏控件区域
    FullscreenOverlayEntered,

    /// 鼠标离开预览右上角全屏控件区域
    FullscreenOverlayExited,

    #[cfg(not(target_arch = "wasm32"))]
    EditorMouseEntered,

    #[cfg(not(target_arch = "wasm32"))]
    EditorMouseExited,

    /// LSP 悬停提示进入（仅非 WASM 平台）
    ///
    /// 当鼠标悬停在符号上时触发，请求 LSP 服务器提供类型信息。
    #[cfg(not(target_arch = "wasm32"))]
    LspHoverEntered,

    /// LSP 悬停提示退出（仅非 WASM 平台）
    ///
    /// 当鼠标离开符号区域时触发，用于关闭悬停提示。
    #[cfg(not(target_arch = "wasm32"))]
    LspHoverExited,

    /// 选择指定的 LSP 自动补全项（仅非 WASM 平台）
    ///
    /// # 参数
    /// - `usize`: 补全列表中的索引
    #[cfg(not(target_arch = "wasm32"))]
    LspCompletionSelected(usize),

    /// 关闭 LSP 自动补全面板（仅非 WASM 平台）
    #[cfg(not(target_arch = "wasm32"))]
    LspCompletionClosed,

    /// LSP 补全列表向上导航（仅非 WASM 平台）
    #[cfg(not(target_arch = "wasm32"))]
    LspCompletionNavigateUp,

    /// LSP 补全列表向下导航（仅非 WASM 平台）
    #[cfg(not(target_arch = "wasm32"))]
    LspCompletionNavigateDown,

    /// 确认当前选中的 LSP 补全项（仅非 WASM 平台）
    #[cfg(not(target_arch = "wasm32"))]
    LspCompletionConfirm,

    /// 切换 LSP 是否禁用（仅非 WASM 平台）
    #[cfg(not(target_arch = "wasm32"))]
    LspToggleDisabled(bool),

    /// 向后回溯导航历史
    ///
    /// 返回上一个访问过的位置（如跳转定义前的位置）。
    TraceBack,

    /// 向前前进导航历史
    ///
    /// 在执行过 [`TraceBack`] 后，前往下一个位置。
    TraceForward,

    /// 保存当前文件
    SaveFile,

    /// 保存指定路径的文件。
    SaveFilePath {
        path: String,
        notify: bool,
    },

    /// 指定文件保存完成。
    SaveFileFinished {
        path: String,
        content: String,
        notify: bool,
        result: Result<(), String>,
    },

    /// 自动保存延迟已到达。
    AutoSaveDelayElapsed {
        path: String,
        revision: u64,
    },

    /// 窗口失去焦点。
    WindowUnfocused,

    /// 修改预览编辑器自动保存模式。
    AutoSaveModeChanged(config_ui::PreviewAutoSaveMode),

    /// 路径段被点击
    ///
    /// 用于面包屑导航，显示该目录下的文件列表。
    ///
    /// # 参数
    /// - `String`: 被点击的路径段
    /// - `Option<(f32, f32)>`: 点击位置坐标（用于定位弹出菜单）
    PathSegmentClicked(String, Option<(f32, f32)>),

    /// 关闭导航弹出菜单
    CloseNavPopup,

    /// 选择导航弹出菜单中的项目
    ///
    /// # 参数
    /// - `String`: 选中的路径
    /// - `bool`: 是否在新标签页打开
    NavPopupSelect(String, bool),

    /// 标签页被右键点击
    ///
    /// # 参数
    /// - `String`: 标签页对应的文件路径
    /// - `f32`: 点击 X 坐标
    /// - `f32`: 点击 Y 坐标
    TabRightClicked(String, f32, f32),

    /// 关闭标签页右键菜单
    TabMenuClose,

    /// 关闭指定标签页左侧的所有标签页
    ///
    /// # 参数
    /// - `String`: 参考标签页的路径
    TabMenuCloseLeft(String),

    /// 关闭指定标签页右侧的所有标签页
    ///
    /// # 参数
    /// - `String`: 参考标签页的路径
    TabMenuCloseRight(String),

    /// 关闭所有标签页
    TabMenuCloseAll,
}
