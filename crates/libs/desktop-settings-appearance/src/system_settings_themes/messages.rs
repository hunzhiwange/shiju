use super::state::ThemeEditorTab;
/// 主题编辑器调色板字段。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThemePaletteField {
    Background,
    Text,
    Primary,
    Success,
    Warning,
    Danger,
}

/// 主题编辑器控件皮肤字段。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThemeChromeField {
    InputBg,
    InputBorder,
    InputText,
    PanelBg,
    PanelBorder,
    ButtonBg,
    ButtonText,
    RadiusPx,
    PanelAlpha,
    ChatHeaderAlpha,
    ChatBodyAlpha,
    ChatInputAlpha,
    /// 聊头背景图缩放策略。
    ChatHeaderImageFit,
    /// 消息区背景图缩放策略。
    ChatBodyImageFit,
    /// 输入区背景图缩放策略。
    ChatInputImageFit,
}

/// 主题编辑器当前打开的颜色选择器目标（同时只开一个）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThemeColorPickerTarget {
    Palette(ThemePaletteField),
    Chrome(ThemeChromeField),
}

/// 主题设置页消息。
#[derive(Debug, Clone)]
pub enum ThemesMessage {
    Refresh,
    /// 画廊上一页（已在首页时由 UI 禁用）。
    GalleryPrevPage,
    /// 画廊下一页（已在末页时由 UI 禁用）。
    GalleryNextPage,
    ImportPack,
    ImportPackPicked(Option<String>),
    ExportPack(String),
    ExportPackPicked {
        theme_id: String,
        path: Option<String>,
    },
    ApplyBuiltin(String),
    ApplyCustom(String),
    DeleteCustom(String),
    NewThemeRequested,
    EditCustom(String),
    EditBuiltin(String),
    RestoreBuiltinDefault(String),
    RestorePresetDefault(String),
    CancelEdit,
    /// 切换主题编辑器分段 Tab。
    EditorTabSelected(ThemeEditorTab),
    DraftNameChanged(String),
    DraftTaglineChanged(String),
    DraftPaletteChanged(ThemePaletteField, String),
    PickCover,
    CoverPicked(Option<String>),
    ClearCover,
    PickBackground,
    BackgroundPicked(Option<String>),
    ClearBackground,
    PickChatHeader,
    ChatHeaderPicked(Option<String>),
    ClearChatHeader,
    PickChatBody,
    ChatBodyPicked(Option<String>),
    ClearChatBody,
    PickChatInput,
    ChatInputPicked(Option<String>),
    ClearChatInput,
    DraftChromeChanged(ThemeChromeField, String),
    /// 打开或切换到指定色字段的颜色对话框；同一目标再次触发则关闭。
    ColorPickerToggled(ThemeColorPickerTarget),
    ColorPickerClosed,
    /// 选择器提交的颜色字符串（由 UI 规范化为 `#rrggbb`）。
    ColorPickerColorChanged(String),
    ColorPickerFormatChanged(desktop_ui_controls::color_picker::ColorFormat),
    SaveDraft,
}
