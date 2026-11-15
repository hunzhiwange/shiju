use super::*;

pub(crate) use desktop_settings_network::app_init::state::{HttpRequestSettingsState};

pub(crate) use desktop_settings_agents::acp_state::AcpSettingsState;

pub use desktop_settings_mcp::state::{McpNewServerState, McpNewServerTransport, McpSettingsTab};

#[derive(Debug, Clone)]
pub(crate) struct McpSettingsState {
    pub(crate) tools: crate::app::message::settings::mcp_management_tools::ToolsState,
    pub(crate) diagnostics: crate::app::message::agent_apps_diagnostics::McpState,
    pub(crate) catalog: crate::app::message::settings::mcp_management_list::CatalogState,
    pub(crate) resync: crate::app::message::settings::mcp_management_resync::ResyncState,
    pub(crate) bindings: crate::app::message::settings::mcp_management_bindings::BindingsState,
    pub(crate) import: Option<crate::app::message::settings::mcp_management_import::ImportState>,
    pub(crate) connection_form: Option<crate::app::message::settings::mcp_management_form::Draft>,
    pub(crate) page: desktop_settings_mcp::state::State,
}

impl Default for McpSettingsState {
    fn default() -> Self {
        Self {
            diagnostics: Default::default(),
            tools: Default::default(),
            catalog: Default::default(),
            bindings: Default::default(),
            resync: Default::default(),
            import: None,
            connection_form: None,
            page: Default::default(),
        }
    }
}

// Preserve existing callers until the update slices migrate; state has one owner.
impl std::ops::Deref for McpSettingsState {
    type Target = desktop_settings_mcp::state::State;
    fn deref(&self) -> &Self::Target { &self.page }
}
impl std::ops::DerefMut for McpSettingsState {
    fn deref_mut(&mut self) -> &mut Self::Target { &mut self.page }
}

pub use desktop_settings_security::security_state::*;

pub(crate) use desktop_settings_security::app_init::state::{AutonomyListKind, AutonomyListEditorState, AutonomySettingsState, AutonomySettingsTab};

/// 存储设置面板状态
///
/// 管理持久化存储 provider、连接地址、schema、table 与 TLS 配置。
pub(crate) use desktop_settings_memory::state::StorageSettingsState;



/// 代理设置面板状态
///
/// 管理网络代理的配置，包括 HTTP/HTTPS 代理和作用域等。
pub(crate) use desktop_settings_network::app_init::state::{ProxySettingsState, TunnelSettingsState};

/// 主题设置页视图模式。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum ThemesSettingsMode {
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
pub(crate) struct ThemeEditorDraft {
    pub(crate) id: String,
    /// 编辑已有主题时锁定 id。
    pub(crate) id_readonly: bool,
    /// 非空时表示正在编辑内置主题覆盖层（值为内置显示名）。
    pub(crate) builtin_base: Option<String>,
    pub(crate) name: String,
    pub(crate) tagline: String,
    pub(crate) background: String,
    pub(crate) text: String,
    pub(crate) primary: String,
    pub(crate) success: String,
    pub(crate) warning: String,
    pub(crate) danger: String,
    /// 已入库的封面相对文件名（如 `cover.png`）。
    pub(crate) cover_relative: Option<String>,
    /// 已入库的背景相对文件名（如 `background.jpg`）。
    pub(crate) background_relative: Option<String>,
    /// 已入库的聊头分区相对文件名。
    pub(crate) chat_header_relative: Option<String>,
    /// 已入库的消息区分区相对文件名。
    pub(crate) chat_body_relative: Option<String>,
    /// 已入库的输入区分区相对文件名。
    pub(crate) chat_input_relative: Option<String>,
    /// 待保存时复制的封面外部源路径。
    pub(crate) cover_source: Option<String>,
    /// 待保存时复制的背景外部源路径。
    pub(crate) background_source: Option<String>,
    /// 待保存时复制的聊头分区外部源路径。
    pub(crate) chat_header_source: Option<String>,
    /// 待保存时复制的消息区分区外部源路径。
    pub(crate) chat_body_source: Option<String>,
    /// 待保存时复制的输入区分区外部源路径。
    pub(crate) chat_input_source: Option<String>,
    pub(crate) chrome_input_bg: String,
    pub(crate) chrome_input_border: String,
    pub(crate) chrome_input_text: String,
    pub(crate) chrome_panel_bg: String,
    pub(crate) chrome_panel_border: String,
    pub(crate) chrome_button_bg: String,
    pub(crate) chrome_button_text: String,
    pub(crate) chrome_radius_px: String,
    pub(crate) chrome_panel_alpha: String,
    pub(crate) chrome_chat_header_alpha: String,
    pub(crate) chrome_chat_body_alpha: String,
    pub(crate) chrome_chat_input_alpha: String,
    /// 聊头背景图缩放：cover/contain/fill/scale_down/none；空表示默认 cover。
    pub(crate) chrome_chat_header_image_fit: String,
    /// 消息区背景图缩放。
    pub(crate) chrome_chat_body_image_fit: String,
    /// 输入区背景图缩放。
    pub(crate) chrome_chat_input_image_fit: String,
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
pub(crate) struct ThemesSettingsState {
    /// 画廊条目（内置 + 自定义）。
    pub(crate) entries:
        Vec<crate::app::components::system_settings_themes::gallery::ThemeGalleryEntry>,
    /// 画廊当前页码（从 0 起）。
    pub(crate) page_index: usize,
    /// 最近一次刷新/操作/校验错误。
    pub(crate) error: Option<String>,
    pub(crate) error_translation: Option<crate::app::i18n::format::Message>,
    /// 列表或编辑模式。
    pub(crate) mode: ThemesSettingsMode,
    /// 编辑器当前分段 Tab（仅 Edit 模式使用）。
    pub(crate) editor_tab: ThemeEditorTab,
    /// 编辑器草稿（仅 Edit 模式使用）。
    pub(crate) draft: Option<ThemeEditorDraft>,
    /// 当前打开的颜色选择器目标；`None` 表示关闭。
    pub(crate) color_picker_target: Option<crate::app::message::settings::ThemeColorPickerTarget>,
    /// 颜色选择器显示格式。
    pub(crate) color_picker_format: crate::app::components::color_picker::ColorFormat,
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
            color_picker_format: crate::app::components::color_picker::ColorFormat::Hex,
        }
    }
}

#[cfg(test)]
#[path = "infrastructure_tests.rs"]
mod infrastructure_tests;

pub(crate) use desktop_settings_runtime::state::{MultimodalSettingsState, ObservabilitySettingsState};

#[cfg(test)]
pub(crate) const AUTONOMY_LIST_KINDS: [AutonomyListKind; 7] = [
    AutonomyListKind::AllowedCommands,
    AutonomyListKind::ForbiddenPaths,
    AutonomyListKind::ShellEnvPassthrough,
    AutonomyListKind::AutoApprove,
    AutonomyListKind::AlwaysAsk,
    AutonomyListKind::AllowedRoots,
    AutonomyListKind::NonCliExcludedTools,
];
