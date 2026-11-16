//! Owned editor metrics, navigation, and LSP overlay state; native transport resources stay in the host.
use iced::widget::Id;
#[cfg(not(target_arch = "wasm32"))]
use std::collections::HashMap;
pub struct State {
    pub editor_metrics_preview: crate::metrics::MetricsPreview,
    pub editor_config_persistence: crate::persistence::ConfigPersistence,
    /// 预览滚动容器 ID
    pub preview_scroll_id: Id,
    /// 预览标签页滚动容器 ID
    pub preview_tabs_scroll_id: Id,
    /// LSP 覆盖层状态（仅非 wasm32 平台）
    #[cfg(not(target_arch = "wasm32"))]
    pub lsp_overlay: iced_code_editor::LspOverlayState,
    /// 是否禁用 LSP（仅非 wasm32 平台）
    #[cfg(not(target_arch = "wasm32"))]
    pub lsp_disabled: bool,
    /// 是否正在应用 LSP 补全（仅非 wasm32 平台）
    #[cfg(not(target_arch = "wasm32"))]
    pub lsp_applying_completion: bool,
    /// LSP 悬停锚点（文件路径，位置）（仅非 wasm32 平台）
    #[cfg(not(target_arch = "wasm32"))]
    pub lsp_hover_anchor: Option<(String, iced_code_editor::LspPosition)>,
    /// LSP 覆盖层路径（仅非 wasm32 平台）
    #[cfg(not(target_arch = "wasm32"))]
    pub lsp_overlay_path: Option<String>,
    /// 待处理的 LSP 悬停信息（仅非 wasm32 平台）
    #[cfg(not(target_arch = "wasm32"))]
    pub lsp_hover_pending: Option<super::LspHoverPending>,
    /// LSP 悬停隐藏截止时间（仅非 wasm32 平台）
    #[cfg(not(target_arch = "wasm32"))]
    pub lsp_hover_hide_deadline: Option<std::time::Instant>,
    /// LSP 进度映射（文件路径 -> 进度 ID -> 进度信息）（仅非 wasm32 平台）
    #[cfg(not(target_arch = "wasm32"))]
    pub lsp_progress: HashMap<String, HashMap<String, super::LspProgress>>,
    /// Semantic preview notices, separate from file contents.
    pub preview_messages: std::collections::HashMap<String, desktop_i18n::format::Message>,
    /// LSP 状态信息（仅非 wasm32 平台）
    #[cfg(not(target_arch = "wasm32"))]
    pub lsp_status: Option<String>,
    #[cfg(not(target_arch = "wasm32"))]
    pub lsp_status_message: Option<(String, desktop_i18n::format::Message)>,
    /// 待跳转的预览位置（文件路径，行号，列号）
    pub pending_preview_goto: Option<(String, usize, usize)>,
    /// 预览导航回退栈
    pub preview_trace_back: Vec<(String, usize, usize)>,
    /// 预览导航前进栈
    pub preview_trace_forward: Vec<(String, usize, usize)>,
    /// 是否正在导航预览
    pub preview_trace_navigating: bool,
    /// 上一个预览路径
    pub previous_preview_path: Option<String>,
    /// 是否显示预览上下文菜单
    pub show_preview_context_menu: bool,
    /// 预览上下文菜单目标（文件路径，起始行，起始列，结束行，结束列）
    pub preview_context_target: Option<(String, usize, usize, usize, usize)>,
    /// 预览上下文菜单位置
    pub preview_context_menu_pos: Option<(f32, f32)>,
    /// 预览导航弹出框（文件路径，X，Y，选项列表）
    pub preview_nav_popup: Option<(String, f32, f32, Vec<(String, bool)>)>,
    /// 是否显示预览设置
    pub show_preview_settings: bool,
    pub editor_popover: Option<crate::components::preview_panel::view::options::EditorPopover>,
    /// 是否显示预览右上角全屏浮动控件
    pub show_preview_fullscreen_overlay: bool,
    /// 当前字体大小
    pub current_font_size: f32,
    /// 当前行高
    pub current_line_height: f32,
    /// 是否自动调整行高
    pub auto_adjust_line_height: bool,
    /// 预览编辑器自动保存模式
    pub preview_auto_save_mode: config_ui::PreviewAutoSaveMode,
    /// 当前语言设置
    pub current_language: iced_code_editor::i18n::Language,
}
impl State {
    pub fn new(system: &config_ui::AppSystemSettingsConfig) -> Self {
        Self {
            editor_metrics_preview: Default::default(),
            editor_config_persistence: Default::default(),
            preview_scroll_id: iced::widget::Id::new("preview"),
            preview_tabs_scroll_id: iced::widget::Id::new("preview_tabs"),
            #[cfg(not(target_arch = "wasm32"))]
            lsp_overlay: iced_code_editor::LspOverlayState::new(),
            #[cfg(not(target_arch = "wasm32"))]
            lsp_disabled: !system.lsp_enabled,
            #[cfg(not(target_arch = "wasm32"))]
            lsp_applying_completion: false,
            #[cfg(not(target_arch = "wasm32"))]
            lsp_hover_anchor: None,
            #[cfg(not(target_arch = "wasm32"))]
            lsp_overlay_path: None,
            #[cfg(not(target_arch = "wasm32"))]
            lsp_hover_pending: None,
            #[cfg(not(target_arch = "wasm32"))]
            lsp_hover_hide_deadline: None,
            #[cfg(not(target_arch = "wasm32"))]
            lsp_progress: HashMap::new(),
            preview_messages: std::collections::HashMap::new(),
            #[cfg(not(target_arch = "wasm32"))]
            lsp_status: None,
            #[cfg(not(target_arch = "wasm32"))]
            lsp_status_message: None,
            pending_preview_goto: None,
            preview_trace_back: Vec::new(),
            preview_trace_forward: Vec::new(),
            preview_trace_navigating: false,
            previous_preview_path: None,
            show_preview_context_menu: false,
            preview_context_target: None,
            preview_context_menu_pos: None,
            preview_nav_popup: None,
            show_preview_settings: false,
            editor_popover: None,
            show_preview_fullscreen_overlay: false,
            current_font_size: system.editor_font_size.max(1.0),
            current_line_height: system.editor_line_height.max(1.0),
            auto_adjust_line_height: system.editor_auto_line_height,
            preview_auto_save_mode: system.preview_auto_save,
            current_language: desktop_i18n::editor::language(system.desktop_locale),
        }
    }
}
