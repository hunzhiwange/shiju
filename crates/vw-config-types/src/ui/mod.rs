mod gateway_client;
mod preferences;
mod quick_messages;

pub use gateway_client::{GatewayClientServerConfig, GatewayClientSystemSettingsConfig};
use preferences::deserialize_timezone;
pub use preferences::{
    DEFAULT_TIMEZONE, DesktopLocale, ModelRoute, PreviewAutoSaveMode, TuiKeybindingOverrideConfig,
    default_timezone, timezone_options,
};
use quick_messages::default_quick_messages;
pub use quick_messages::{
    QuickMessageConfig, QuickMessageGroup, QuickMessageKind,
    append_missing_builtin_general_quick_messages, append_missing_builtin_git_quick_messages,
    builtin_general_quick_messages, builtin_git_quick_messages,
    upgrade_legacy_quick_message_actions,
};

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use std::collections::HashMap;

/// Desktop app UI configuration (`[app_ui]` section).
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Default, PartialEq)]
pub struct AppUiConfig {
    /// Gateway service configuration, independent of local desktop device preferences.
    #[serde(default)]
    pub voice: config_voice::VoiceConfig,
    /// System settings values managed by the desktop UI.
    #[serde(default)]
    pub system_settings: AppSystemSettingsConfig,
}

/// Desktop system settings persisted in agent config (`[app_ui.system_settings]`).
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct AppSystemSettingsConfig {
    /// Language used by the desktop interface.
    #[serde(default)]
    pub desktop_locale: DesktopLocale,
    /// Desktop gateway client connection settings.
    #[serde(default)]
    pub gateway_client: GatewayClientSystemSettingsConfig,
    /// Iced theme name (`Theme::to_string()`).
    #[serde(default = "default_app_theme_name")]
    pub app_theme: String,
    /// IANA timezone used for chat date and time context.
    #[serde(default = "default_timezone", deserialize_with = "deserialize_timezone")]
    pub timezone: String,
    /// Send a native desktop notification after a successful conversation turn.
    #[serde(default)]
    pub task_completion_notification: bool,
    /// Terminal shell key (`bash` | `zsh`).
    #[serde(default = "default_terminal_shell")]
    pub terminal_shell: String,
    /// Terminal theme key (`system` | `ui` | `solarized_dark` | `monokai`).
    #[serde(default = "default_terminal_theme")]
    pub terminal_theme: String,
    /// Terminal font family.
    #[serde(default = "default_terminal_font_family")]
    pub terminal_font_family: String,
    /// Terminal font size.
    #[serde(default = "default_terminal_font_size")]
    pub terminal_font_size: f32,
    /// Whether editor theme follows system/app theme.
    #[serde(default = "default_true")]
    pub editor_follow_system_theme: bool,
    /// Iced editor theme name (`Theme::to_string()`).
    #[serde(default = "default_app_theme_name")]
    pub editor_theme: String,
    /// Per-project worktree toggle map.
    #[serde(default)]
    pub project_worktree_enabled: HashMap<String, bool>,
    /// Preview/editor font size.
    #[serde(default = "default_editor_font_size")]
    pub editor_font_size: f32,
    /// Preview/editor line height.
    #[serde(default = "default_editor_line_height")]
    pub editor_line_height: f32,
    /// Whether line height follows font size.
    #[serde(default = "default_false")]
    pub editor_auto_line_height: bool,
    /// Preview editor auto save mode.
    #[serde(default)]
    pub preview_auto_save: PreviewAutoSaveMode,
    /// Whether language-server features are enabled in the desktop preview editor.
    #[serde(default = "default_false")]
    pub lsp_enabled: bool,
    #[serde(default = "default_false")]
    pub dialogue_flow_show_reasoning_summary: bool,
    #[serde(default = "default_false")]
    pub dialogue_flow_expand_shell_tool_section: bool,
    #[serde(default = "default_false")]
    pub dialogue_flow_expand_edit_tool_section: bool,
    /// Configurable shortcuts shown above the desktop chat input.
    #[serde(default = "default_quick_messages")]
    pub quick_messages: Vec<QuickMessageConfig>,
    /// Desktop-managed model routing rules.
    #[serde(default)]
    pub model_routes: Vec<ModelRoute>,
    /// TUI logical action overrides (`context`, `action`, `keys`).
    #[serde(default)]
    pub tui_keybindings: Vec<TuiKeybindingOverrideConfig>,
}

fn default_app_theme_name() -> String {
    "Light".to_string()
}

fn default_terminal_shell() -> String {
    "zsh".to_string()
}

fn default_terminal_theme() -> String {
    "system".to_string()
}

fn default_terminal_font_family() -> String {
    "JetBrains Mono".to_string()
}

fn default_terminal_font_size() -> f32 {
    15.0
}

fn default_editor_font_size() -> f32 {
    16.0
}

fn default_editor_line_height() -> f32 {
    19.6
}

fn default_true() -> bool {
    true
}

fn default_false() -> bool {
    false
}

impl Default for AppSystemSettingsConfig {
    fn default() -> Self {
        Self {
            gateway_client: GatewayClientSystemSettingsConfig::default(),
            desktop_locale: DesktopLocale::default(),
            app_theme: default_app_theme_name(),
            timezone: default_timezone(),
            task_completion_notification: false,
            terminal_shell: default_terminal_shell(),
            terminal_theme: default_terminal_theme(),
            terminal_font_family: default_terminal_font_family(),
            terminal_font_size: default_terminal_font_size(),
            editor_follow_system_theme: default_true(),
            editor_theme: default_app_theme_name(),
            project_worktree_enabled: HashMap::new(),
            editor_font_size: default_editor_font_size(),
            editor_line_height: default_editor_line_height(),
            editor_auto_line_height: default_false(),
            preview_auto_save: PreviewAutoSaveMode::default(),
            lsp_enabled: default_false(),
            dialogue_flow_show_reasoning_summary: default_false(),
            dialogue_flow_expand_shell_tool_section: default_false(),
            dialogue_flow_expand_edit_tool_section: default_false(),
            quick_messages: default_quick_messages(),
            model_routes: Vec::new(),
            tui_keybindings: Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests;
