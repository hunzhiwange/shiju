//! 常规设置的行标签和定位标识，由表单与搜索目录共用。

use crate::GeneralSettingsTab;
use desktop_editor::components::editor_settings_fields::fields as editor_fields;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GeneralSettingField {
    pub label: &'static str,
    pub tab: GeneralSettingsTab,
    pub id: &'static str,
    pub keywords: &'static [&'static str],
}

pub const LAUNCH_ON_LOGIN: GeneralSettingField = GeneralSettingField {
    label: "开机启动",
    tab: GeneralSettingsTab::Interface,
    id: "general-settings-launch-on-login",
    keywords: &["startup", "launch at login"],
};
pub const CLOSE_TO_TRAY: GeneralSettingField = GeneralSettingField {
    label: "关闭到托盘",
    tab: GeneralSettingsTab::Interface,
    id: "general-settings-close-to-tray",
    keywords: &["后台运行", "tray"],
};
pub const LANGUAGE: GeneralSettingField = GeneralSettingField {
    label: "语言",
    tab: GeneralSettingsTab::Interface,
    id: "general-settings-language",
    keywords: &["locale", "界面语言"],
};
pub const APP_THEME: GeneralSettingField = GeneralSettingField {
    label: "应用主题",
    tab: GeneralSettingsTab::Interface,
    id: "general-settings-app-theme",
    keywords: &[],
};
pub const AUTO_SAVE: GeneralSettingField = GeneralSettingField {
    label: "自动保存",
    tab: GeneralSettingsTab::Interface,
    id: "general-settings-auto-save",
    keywords: &["preview auto save"],
};
pub const TASK_NOTIFICATION: GeneralSettingField = GeneralSettingField {
    label: "任务完成通知",
    tab: GeneralSettingsTab::Interface,
    id: "general-settings-task-notification",
    keywords: &["notification"],
};
pub const CHAT_TIMEZONE: GeneralSettingField = GeneralSettingField {
    label: "聊天时区",
    tab: GeneralSettingsTab::Interface,
    id: "general-settings-chat-timezone",
    keywords: &["timezone"],
};
pub const TERMINAL_SHELL: GeneralSettingField = GeneralSettingField {
    label: "终端 Shell",
    tab: GeneralSettingsTab::Terminal,
    id: "general-settings-terminal-shell",
    keywords: &[],
};
pub const TERMINAL_THEME: GeneralSettingField = GeneralSettingField {
    label: "终端主题",
    tab: GeneralSettingsTab::Terminal,
    id: "general-settings-terminal-theme",
    keywords: &[],
};
pub const TERMINAL_FONT: GeneralSettingField = GeneralSettingField {
    label: "终端字体",
    tab: GeneralSettingsTab::Terminal,
    id: "general-settings-terminal-font",
    keywords: &[],
};
pub const TERMINAL_FONT_SIZE: GeneralSettingField =
    GeneralSettingField {
        label: "字体大小",
        tab: GeneralSettingsTab::Terminal,
        id: "general-settings-terminal-font-size",
        keywords: &[],
    };
pub const EDITOR_FOLLOW_THEME: GeneralSettingField =
    GeneralSettingField {
        label: editor_fields::EDITOR_FOLLOW_THEME.label,
        tab: GeneralSettingsTab::Editor,
        id: editor_fields::EDITOR_FOLLOW_THEME.id,
        keywords: &[],
    };
pub const EDITOR_THEME: GeneralSettingField = GeneralSettingField {
    label: editor_fields::EDITOR_THEME.label,
    tab: GeneralSettingsTab::Editor,
    id: editor_fields::EDITOR_THEME.id,
    keywords: &[],
};
pub const EDITOR_FONT_SIZE: GeneralSettingField = GeneralSettingField {
    label: editor_fields::EDITOR_FONT_SIZE.label,
    tab: GeneralSettingsTab::Editor,
    id: editor_fields::EDITOR_FONT_SIZE.id,
    keywords: &[],
};
pub const EDITOR_LINE_HEIGHT: GeneralSettingField =
    GeneralSettingField {
        label: editor_fields::EDITOR_LINE_HEIGHT.label,
        tab: GeneralSettingsTab::Editor,
        id: editor_fields::EDITOR_LINE_HEIGHT.id,
        keywords: &[],
    };
pub const EDITOR_AUTO_LINE_HEIGHT: GeneralSettingField =
    GeneralSettingField {
        label: editor_fields::EDITOR_AUTO_LINE_HEIGHT.label,
        tab: GeneralSettingsTab::Editor,
        id: editor_fields::EDITOR_AUTO_LINE_HEIGHT.id,
        keywords: &[],
    };
#[cfg(not(target_arch = "wasm32"))]
pub const EDITOR_LSP: GeneralSettingField = GeneralSettingField {
    label: editor_fields::EDITOR_LSP.label,
    tab: GeneralSettingsTab::Editor,
    id: editor_fields::EDITOR_LSP.id,
    keywords: &["language server"],
};

pub const ALL: &[&GeneralSettingField] = &[
    &LAUNCH_ON_LOGIN,
    &CLOSE_TO_TRAY,
    &LANGUAGE,
    &APP_THEME,
    &AUTO_SAVE,
    &TASK_NOTIFICATION,
    &CHAT_TIMEZONE,
    &TERMINAL_SHELL,
    &TERMINAL_THEME,
    &TERMINAL_FONT,
    &TERMINAL_FONT_SIZE,
    &EDITOR_FOLLOW_THEME,
    &EDITOR_THEME,
    &EDITOR_FONT_SIZE,
    &EDITOR_LINE_HEIGHT,
    &EDITOR_AUTO_LINE_HEIGHT,
    #[cfg(not(target_arch = "wasm32"))]
    &EDITOR_LSP,
];
