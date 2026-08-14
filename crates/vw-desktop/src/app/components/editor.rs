//! 编辑器组件模块
//!
//! 本模块提供了代码编辑器组件的封装，基于 `iced_code_editor` 库构建。
//! 该组件支持语法高亮、搜索替换、撤销/重做等功能，并在非 WebAssembly
//! 环境下支持语言服务器协议（LSP）集成，提供代码补全、悬停提示、
//! 跳转到定义等高级编辑器功能。
//!
//! # 主要功能
//!
//! - **基础编辑功能**：文本内容管理、撤销/重做操作
//! - **外观配置**：字体、字号、行高、主题设置
//! - **界面功能**：行号显示、搜索替换对话框
//! - **LSP 集成**（非 wasm32）：代码补全、悬停提示、定义跳转
//!
//! # 示例
//!
//! ```ignore
//! use crate::app::components::editor::Editor;
//! use iced_code_editor::i18n;
//!
//! // 创建一个新的编辑器实例
//! let mut editor = Editor::new("fn main() {}", "rust");
//!
//! // 配置编辑器外观
//! editor.set_font(iced::Font::MONOSPACE);
//! editor.set_font_size(14.0);
//! editor.set_line_numbers_enabled(true);
//! ```

use iced::{Element, Task};
use iced_code_editor::{CodeEditor, ContextMenuEntry, i18n, theme};

pub(crate) const APP_CONTEXT_MENU_INSERT_SELECTION: &str = "app.insert_selection";
pub(crate) const APP_CONTEXT_MENU_INSERT_SELECTION_POSITIONS: &str =
    "app.insert_selection_positions";

/// 代码编辑器组件
///
/// 该结构体是对 `iced_code_editor::CodeEditor` 的封装，提供了统一的编辑器接口。
/// 在非 WebAssembly 环境下，还包含 LSP 连接状态的管理。
///
/// # 字段
///
/// - `inner`：内部封装的代码编辑器实例
/// - `lsp_attached`（非 wasm32）：标记是否已附加 LSP 客户端
pub struct Editor {
    /// 内部代码编辑器实例
    pub inner: CodeEditor,

    /// LSP 客户端附加状态（仅在非 wasm32 架构下可用）
    /// 用于跟踪编辑器是否已连接到语言服务器
    #[cfg(not(target_arch = "wasm32"))]
    lsp_attached: bool,

    preview_context_menu_configured: bool,
}

impl Editor {
    /// 创建一个新的编辑器实例
    ///
    /// # 参数
    ///
    /// - `content`：编辑器的初始内容文本
    /// - `syntax`：语法高亮语言标识符（如 "rust"、"javascript"、"python" 等）
    ///
    /// # 返回值
    ///
    /// 返回一个新创建的 `Editor` 实例，LSP 连接状态初始化为 `false`（非 wasm32 环境）
    ///
    /// # 示例
    ///
    /// ```ignore
    /// // 创建一个包含 Rust 代码的编辑器
    /// let editor = Editor::new("fn main() { println!(\"Hello\"); }", "rust");
    ///
    /// // 创建一个空的 Python 编辑器
    /// let editor = Editor::new("", "python");
    /// ```
    pub fn new(content: &str, syntax: &str) -> Self {
        Self {
            inner: CodeEditor::new(content, syntax),
            #[cfg(not(target_arch = "wasm32"))]
            lsp_attached: false,
            preview_context_menu_configured: false,
        }
    }

    /// 获取编辑器当前内容
    ///
    /// # 返回值
    ///
    /// 返回编辑器中的完整文本内容
    ///
    /// # 示例
    ///
    /// ```ignore
    /// let content = editor.content();
    /// println!("当前内容: {}", content);
    /// ```
    pub fn content(&self) -> String {
        self.inner.content()
    }

    /// 设置编辑器字体
    ///
    /// # 参数
    ///
    /// - `font`：要使用的字体（通常使用等宽字体以获得最佳显示效果）
    ///
    /// # 示例
    ///
    /// ```ignore
    /// editor.set_font(iced::Font::MONOSPACE);
    /// ```
    pub fn set_font(&mut self, font: iced::Font) {
        self.inner.set_font(font);
    }

    /// 设置编辑器字体大小
    ///
    /// # 参数
    ///
    /// - `size`：字体大小（以像素为单位）
    ///
    /// # 备注
    ///
    /// 内部调用会将第二个参数（`proportional`）设为 `true`，
    /// 意味着行高也会按比例调整。
    ///
    /// # 示例
    ///
    /// ```ignore
    /// editor.set_font_size(16.0);
    /// ```
    pub fn set_font_size(&mut self, size: f32) {
        self.inner.set_font_size(size, true);
    }

    /// 设置编辑器行高
    ///
    /// # 参数
    ///
    /// - `height`：行高值（以像素为单位）
    ///
    /// # 示例
    ///
    /// ```ignore
    /// editor.set_line_height(20.0);
    /// ```
    pub fn set_line_height(&mut self, height: f32) {
        self.inner.set_line_height(height);
    }

    /// 设置编辑器主题
    ///
    /// 将 iced 主题转换为代码编辑器主题并应用。
    ///
    /// # 参数
    ///
    /// - `theme`：iced 框架的主题实例
    ///
    /// # 示例
    ///
    /// ```ignore
    /// editor.set_theme(iced::Theme::Dark);
    /// ```
    pub fn set_theme(&mut self, theme: iced::Theme) {
        self.inner.set_theme(theme::from_iced_theme(&theme));
    }

    /// 设置编辑器界面语言
    ///
    /// 用于本地化编辑器的用户界面元素（如菜单、提示等）。
    ///
    /// # 参数
    ///
    /// - `language`：界面语言枚举值
    ///
    /// # 示例
    ///
    /// ```ignore
    /// use iced_code_editor::i18n::Language;
    /// editor.set_ui_language(Language::ZhCn);
    /// ```
    pub fn set_ui_language(&mut self, language: i18n::Language) {
        self.inner.set_language(language);
        if self.preview_context_menu_configured {
            self.inner.set_custom_context_menu_entries(preview_context_menu_entries(language));
        }
    }

    /// 配置预览编辑器使用的应用菜单项和底层默认编辑菜单。
    pub fn configure_preview_context_menu(
        &mut self,
        language: i18n::Language,
        reveal_in_file_manager_enabled: bool,
    ) {
        self.preview_context_menu_configured = true;
        self.inner.set_custom_context_menu_entries(preview_context_menu_entries(language));
        self.inner.set_default_context_menu_enabled(true);
        self.inner.set_reveal_in_file_manager_enabled(reveal_in_file_manager_enabled);
    }

    /// 检查是否可以执行撤销操作
    ///
    /// # 返回值
    ///
    /// 如果存在可撤销的历史记录，返回 `true`；否则返回 `false`
    ///
    /// # 示例
    ///
    /// ```ignore
    /// if editor.can_undo() {
    ///     // 执行撤销操作
    /// }
    /// ```
    pub fn can_undo(&self) -> bool {
        self.inner.can_undo()
    }

    /// 检查是否可以执行重做操作
    ///
    /// # 返回值
    ///
    /// 如果存在可重做的历史记录，返回 `true`；否则返回 `false`
    ///
    /// # 示例
    ///
    /// ```ignore
    /// if editor.can_redo() {
    ///     // 执行重做操作
    /// }
    /// ```
    pub fn can_redo(&self) -> bool {
        self.inner.can_redo()
    }

    /// 设置是否启用行号显示
    ///
    /// # 参数
    ///
    /// - `enabled`：`true` 启用行号显示，`false` 禁用
    ///
    /// # 示例
    ///
    /// ```ignore
    /// editor.set_line_numbers_enabled(true);
    /// ```
    pub fn set_line_numbers_enabled(&mut self, enabled: bool) {
        self.inner.set_line_numbers_enabled(enabled);
    }

    /// 设置是否启用搜索替换功能
    ///
    /// # 参数
    ///
    /// - `enabled`：`true` 启用搜索替换，`false` 禁用
    ///
    /// # 示例
    ///
    /// ```ignore
    /// editor.set_search_replace_enabled(true);
    /// ```
    pub fn set_search_replace_enabled(&mut self, enabled: bool) {
        self.inner.set_search_replace_enabled(enabled);
    }

    /// 打开搜索对话框
    ///
    /// # 返回值
    ///
    /// 返回一个 `Task`，用于异步执行打开搜索对话框的操作
    ///
    /// # 示例
    ///
    /// ```ignore
    /// let task = editor.open_search_dialog();
    /// // 将 task 传递给 iced 运行时执行
    /// ```
    pub fn open_search_dialog(&mut self) -> Task<iced_code_editor::Message> {
        self.inner.open_search_dialog()
    }

    /// 打开搜索替换对话框
    ///
    /// 与 `open_search_dialog` 类似，但同时显示替换输入框。
    ///
    /// # 返回值
    ///
    /// 返回一个 `Task`，用于异步执行打开搜索替换对话框的操作
    ///
    /// # 示例
    ///
    /// ```ignore
    /// let task = editor.open_search_replace_dialog();
    /// ```
    pub fn open_search_replace_dialog(&mut self) -> Task<iced_code_editor::Message> {
        self.inner.open_search_replace_dialog()
    }

    /// 关闭搜索对话框
    ///
    /// # 返回值
    ///
    /// 返回一个 `Task`，用于异步执行关闭搜索对话框的操作
    ///
    /// # 示例
    ///
    /// ```ignore
    /// let task = editor.close_search_dialog();
    /// ```
    pub fn close_search_dialog(&mut self) -> Task<iced_code_editor::Message> {
        self.inner.close_search_dialog()
    }

    /// 附加 LSP 客户端到编辑器
    ///
    /// 将语言服务器协议客户端连接到编辑器，启用智能代码补全、
    /// 悬停提示、定义跳转等高级功能。此功能仅在非 WebAssembly 环境下可用。
    ///
    /// # 参数
    ///
    /// - `client`：LSP 客户端实例，实现 `iced_code_editor::LspClient` trait
    /// - `document`：LSP 文档信息，包含文件 URI、语言 ID 等
    ///
    /// # 备注
    ///
    /// 调用此方法后，`has_lsp_attached()` 将返回 `true`。
    ///
    /// # 示例
    ///
    /// ```ignore
    /// let client = create_lsp_client();
    /// let document = LspDocument::new("file:///path/to/file.rs", "rust");
    /// editor.attach_lsp(client, document);
    /// ```
    #[cfg(not(target_arch = "wasm32"))]
    pub fn attach_lsp(
        &mut self,
        client: Box<dyn iced_code_editor::LspClient>,
        document: iced_code_editor::LspDocument,
    ) {
        self.inner.attach_lsp(client, document);
        self.lsp_attached = true;
    }

    /// 向 LSP 服务器通知文档打开事件
    ///
    /// 在切换到新文档时调用，通知 LSP 服务器当前活动文档的变化。
    /// 此功能仅在非 WebAssembly 环境下可用。
    ///
    /// # 参数
    ///
    /// - `document`：新打开的 LSP 文档信息
    #[cfg(not(target_arch = "wasm32"))]
    pub fn lsp_open_document(&mut self, document: iced_code_editor::LspDocument) {
        self.inner.lsp_open_document(document);
    }

    /// 分离 LSP 客户端
    ///
    /// 断开与语言服务器的连接，清理相关资源。
    /// 此功能仅在非 WebAssembly 环境下可用。
    ///
    /// # 备注
    ///
    /// 调用此方法后，`has_lsp_attached()` 将返回 `false`。
    #[cfg(not(target_arch = "wasm32"))]
    pub fn detach_lsp(&mut self) {
        self.inner.detach_lsp();
        self.lsp_attached = false;
    }

    /// 检查是否已附加 LSP 客户端
    ///
    /// # 返回值
    ///
    /// 如果编辑器已连接到 LSP 服务器，返回 `true`；否则返回 `false`
    ///
    /// # 示例
    ///
    /// ```ignore
    /// if editor.has_lsp_attached() {
    ///     // 可以使用 LSP 相关功能
    /// }
    /// ```
    #[cfg(not(target_arch = "wasm32"))]
    pub fn has_lsp_attached(&self) -> bool {
        self.lsp_attached
    }

    /// 通知 LSP 服务器文档已保存
    ///
    /// 在文档保存操作完成后调用，使 LSP 服务器能够更新其内部状态。
    /// 此功能仅在非 WebAssembly 环境下可用。
    #[cfg(not(target_arch = "wasm32"))]
    pub fn lsp_did_save(&mut self) {
        self.inner.lsp_did_save();
    }

    /// 刷新待处理的 LSP 变更
    ///
    /// 将编辑器中累积的文档变更立即发送给 LSP 服务器，
    /// 而不是等待自动刷新。此功能仅在非 WebAssembly 环境下可用。
    #[cfg(not(target_arch = "wasm32"))]
    pub fn lsp_flush_pending_changes(&mut self) {
        self.inner.lsp_flush_pending_changes();
    }

    /// 请求 LSP 代码补全
    ///
    /// 在当前光标位置请求代码补全建议。
    /// 此功能仅在非 WebAssembly 环境下可用。
    #[cfg(not(target_arch = "wasm32"))]
    pub fn lsp_request_completion(&mut self) {
        self.inner.lsp_request_completion();
    }

    /// 在指定位置请求 LSP 悬停提示
    ///
    /// # 参数
    ///
    /// - `position`：要请求悬停提示的文档位置（行、列）
    ///
    /// # 返回值
    ///
    /// 如果成功发起请求，返回 `true`；否则返回 `false`
    #[cfg(not(target_arch = "wasm32"))]
    pub fn lsp_request_hover_at_position(
        &mut self,
        position: iced_code_editor::LspPosition,
    ) -> bool {
        self.inner.lsp_request_hover_at_position(position)
    }

    /// 获取指定屏幕坐标处的悬停锚点信息
    ///
    /// 用于确定鼠标悬停时应该请求哪个位置的 LSP 提示。
    /// 此功能仅在非 WebAssembly 环境下可用。
    ///
    /// # 参数
    ///
    /// - `point`：屏幕坐标点
    ///
    /// # 返回值
    ///
    /// 如果在该位置存在可悬停的内容，返回 `Some((position, anchor_point))`，
    /// 其中 `position` 是文档位置，`anchor_point` 是提示框的锚点位置；
    /// 否则返回 `None`。
    #[cfg(not(target_arch = "wasm32"))]
    pub fn lsp_hover_anchor_at_point(
        &self,
        point: iced::Point,
    ) -> Option<(iced_code_editor::LspPosition, iced::Point)> {
        self.inner.lsp_hover_anchor_at_point(point)
    }

    /// 在指定屏幕坐标处请求跳转到定义
    ///
    /// # 参数
    ///
    /// - `point`：屏幕坐标点
    ///
    /// # 返回值
    ///
    /// 如果成功发起请求，返回 `true`；否则返回 `false`
    #[cfg(not(target_arch = "wasm32"))]
    pub fn lsp_request_definition_at(&mut self, point: iced::Point) -> bool {
        self.inner.lsp_request_definition_at(point)
    }

    /// 设置 LSP 自动刷新模式
    ///
    /// 启用时，文档变更会自动发送给 LSP 服务器；
    /// 禁用时，需要手动调用 `lsp_flush_pending_changes()` 发送变更。
    /// 此功能仅在非 WebAssembly 环境下可用。
    ///
    /// # 参数
    ///
    /// - `enabled`：`true` 启用自动刷新，`false` 禁用
    #[cfg(not(target_arch = "wasm32"))]
    pub fn set_lsp_auto_flush(&mut self, enabled: bool) {
        self.inner.set_lsp_auto_flush(enabled);
    }

    /// 获取光标的屏幕位置
    ///
    /// # 返回值
    ///
    /// 如果光标可见，返回 `Some(point)` 包含光标的屏幕坐标；
    /// 否则返回 `None`。
    #[cfg(not(target_arch = "wasm32"))]
    pub fn cursor_screen_position(&self) -> Option<iced::Point> {
        self.inner.cursor_screen_position()
    }

    /// 获取光标的文档位置
    ///
    /// # 返回值
    ///
    /// 返回一个元组 `(line, column)`，表示光标在文档中的位置。
    /// 行和列都是从 0 开始索引。
    #[cfg(not(target_arch = "wasm32"))]
    pub fn cursor_position(&self) -> (usize, usize) {
        self.inner.cursor_position()
    }

    /// 创建编辑器的视图元素
    ///
    /// 生成包含底层原生右键菜单能力的编辑器 `Element`。
    ///
    /// # 参数
    ///
    /// - `on_event`：处理编辑器内部事件的回调函数，接收 `iced_code_editor::Message`
    ///
    /// # 返回值
    ///
    /// 返回一个 `Element`，可嵌入到 iced 界面层次结构中
    ///
    /// # 类型参数
    ///
    /// - `Message`：应用程序的消息类型，必须实现 `Clone` 和 `'a` 生命周期
    ///
    /// # 示例
    ///
    /// ```ignore
    /// fn view(&self) -> Element<Message> {
    ///     editor.view(
    ///         |msg| Message::EditorEvent(msg),
    ///         |point| Message::RightClick(point),
    ///     )
    /// }
    /// ```
    pub fn content_view<'a, Message>(
        &'a self,
        on_event: impl Fn(iced_code_editor::Message) -> Message + 'a,
    ) -> Element<'a, Message, iced::Theme, iced::Renderer>
    where
        Message: Clone + 'a,
    {
        self.inner.view().map(on_event)
    }
}

fn preview_context_menu_entries(language: i18n::Language) -> Vec<ContextMenuEntry> {
    let (insert_selection, insert_positions) = match language {
        i18n::Language::ChineseSimplified => ("插入选择到聊天", "插入选区位置信息"),
        _ => ("Insert selection into chat", "Insert selection positions"),
    };

    vec![
        ContextMenuEntry::item(APP_CONTEXT_MENU_INSERT_SELECTION, insert_selection),
        ContextMenuEntry::item(APP_CONTEXT_MENU_INSERT_SELECTION_POSITIONS, insert_positions),
    ]
}
