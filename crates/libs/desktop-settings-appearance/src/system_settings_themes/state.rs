/// 主题设置页视图模式。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ThemesSettingsMode {
    #[default]
    List,
    Edit,
}

/// 主题编辑器内部分段 Tab（降低单页高度）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ThemeEditorTab {
    #[default]
    Basics,
    Palette,
    Assets,
    Chrome,
    Preview,
}

/// 主题编辑器草稿：命名 + 六色 hex + 资源路径 + chrome。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ThemeEditorDraft {
    pub id: String,
    /// 编辑已有主题时锁定 id。
    pub id_readonly: bool,
    /// 非空时表示正在编辑内置主题覆盖层（值为内置显示名）。
    pub builtin_base: Option<String>,
    pub name: String,
    pub tagline: String,
    pub background: String,
    pub text: String,
    pub primary: String,
    pub success: String,
    pub warning: String,
    pub danger: String,
    /// 已入库的封面相对文件名（如 `cover.png`）。
    pub cover_relative: Option<String>,
    /// 已入库的背景相对文件名（如 `background.jpg`）。
    pub background_relative: Option<String>,
    /// 已入库的聊头分区相对文件名。
    pub chat_header_relative: Option<String>,
    /// 已入库的消息区分区相对文件名。
    pub chat_body_relative: Option<String>,
    /// 已入库的输入区分区相对文件名。
    pub chat_input_relative: Option<String>,
    /// 待保存时复制的封面外部源路径。
    pub cover_source: Option<String>,
    /// 待保存时复制的背景外部源路径。
    pub background_source: Option<String>,
    /// 待保存时复制的聊头分区外部源路径。
    pub chat_header_source: Option<String>,
    /// 待保存时复制的消息区分区外部源路径。
    pub chat_body_source: Option<String>,
    /// 待保存时复制的输入区分区外部源路径。
    pub chat_input_source: Option<String>,
    pub chrome_input_bg: String,
    pub chrome_input_border: String,
    pub chrome_input_text: String,
    pub chrome_panel_bg: String,
    pub chrome_panel_border: String,
    pub chrome_button_bg: String,
    pub chrome_button_text: String,
    pub chrome_radius_px: String,
    pub chrome_panel_alpha: String,
    pub chrome_chat_header_alpha: String,
    pub chrome_chat_body_alpha: String,
    pub chrome_chat_input_alpha: String,
    /// 聊头背景图缩放：cover/contain/fill/scale_down/none；空表示默认 cover。
    pub chrome_chat_header_image_fit: String,
    /// 消息区背景图缩放。
    pub chrome_chat_body_image_fit: String,
    /// 输入区背景图缩放。
    pub chrome_chat_input_image_fit: String,
}

impl Default for ThemeEditorDraft {
    fn default() -> Self {
        Self {
            id: "theme".to_string(),
            id_readonly: false,
            builtin_base: None,
            name: String::new(),
            tagline: String::new(),
            background: "#0f172a".to_string(),
            text: "#e2e8f0".to_string(),
            primary: "#38bdf8".to_string(),
            success: "#22c55e".to_string(),
            warning: "#f59e0b".to_string(),
            danger: "#ef4444".to_string(),
            cover_relative: None,
            background_relative: None,
            chat_header_relative: None,
            chat_body_relative: None,
            chat_input_relative: None,
            cover_source: None,
            background_source: None,
            chat_header_source: None,
            chat_body_source: None,
            chat_input_source: None,
            chrome_input_bg: String::new(),
            chrome_input_border: String::new(),
            chrome_input_text: String::new(),
            chrome_panel_bg: String::new(),
            chrome_panel_border: String::new(),
            chrome_button_bg: String::new(),
            chrome_button_text: String::new(),
            chrome_radius_px: String::new(),
            chrome_panel_alpha: String::new(),
            chrome_chat_header_alpha: String::new(),
            chrome_chat_body_alpha: String::new(),
            chrome_chat_input_alpha: String::new(),
            chrome_chat_header_image_fit: String::new(),
            chrome_chat_body_image_fit: String::new(),
            chrome_chat_input_image_fit: String::new(),
        }
    }
}

/// 主题设置面板状态。
#[derive(Debug, Clone)]
pub struct ThemesSettingsState {
    /// 画廊条目（内置 + 自定义）。
    pub entries:
        Vec<super::gallery::ThemeGalleryEntry>,
    /// 画廊当前页码（从 0 起）。
    pub page_index: usize,
    /// 最近一次刷新/操作/校验错误。
    pub error: Option<String>,
    pub error_translation: Option<desktop_i18n::format::Message>,
    /// 列表或编辑模式。
    pub mode: ThemesSettingsMode,
    /// 编辑器当前分段 Tab（仅 Edit 模式使用）。
    pub editor_tab: ThemeEditorTab,
    /// 编辑器草稿（仅 Edit 模式使用）。
    pub draft: Option<ThemeEditorDraft>,
    /// 当前打开的颜色选择器目标；`None` 表示关闭。
    pub color_picker_target: Option<super::messages::ThemeColorPickerTarget>,
    /// 颜色选择器显示格式。
    pub color_picker_format: desktop_ui_controls::color_picker::ColorFormat,
}

impl Default for ThemesSettingsState {
    fn default() -> Self {
        Self {
            entries: Vec::new(),
            page_index: 0,
            error: None,
            error_translation: None,
            mode: ThemesSettingsMode::List,
            editor_tab: ThemeEditorTab::Basics,
            draft: None,
            color_picker_target: None,
            color_picker_format: desktop_ui_controls::color_picker::ColorFormat::Hex,
        }
    }
}
