//! 常规设置的行标签和定位标识，由表单与搜索目录共用。

use crate::app::state::GeneralSettingsTab;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GeneralSettingField {
    pub(in crate::app::components) label: &'static str,
    pub(crate) tab: GeneralSettingsTab,
    pub(crate) id: &'static str,
    pub(in crate::app::components) keywords: &'static [&'static str],
}

pub(in crate::app::components) const LAUNCH_ON_LOGIN: GeneralSettingField = GeneralSettingField {
    label: "开机启动",
    tab: GeneralSettingsTab::Interface,
    id: "general-settings-launch-on-login",
    keywords: &["startup", "launch at login"],
};
pub(in crate::app::components) const CLOSE_TO_TRAY: GeneralSettingField = GeneralSettingField {
    label: "关闭到托盘",
    tab: GeneralSettingsTab::Interface,
    id: "general-settings-close-to-tray",
    keywords: &["后台运行", "tray"],
};
pub(in crate::app::components) const LANGUAGE: GeneralSettingField = GeneralSettingField {
    label: "语言",
    tab: GeneralSettingsTab::Interface,
    id: "general-settings-language",
    keywords: &["locale", "界面语言"],
};
pub(in crate::app::components) const APP_THEME: GeneralSettingField = GeneralSettingField {
    label: "应用主题",
    tab: GeneralSettingsTab::Interface,
    id: "general-settings-app-theme",
    keywords: &[],
};
pub(in crate::app::components) const AUTO_SAVE: GeneralSettingField = GeneralSettingField {
    label: "自动保存",
    tab: GeneralSettingsTab::Interface,
    id: "general-settings-auto-save",
    keywords: &["preview auto save"],
};
pub(in crate::app::components) const TASK_NOTIFICATION: GeneralSettingField = GeneralSettingField {
    label: "任务完成通知",
    tab: GeneralSettingsTab::Interface,
    id: "general-settings-task-notification",
    keywords: &["notification"],
};
pub(in crate::app::components) const CHAT_TIMEZONE: GeneralSettingField = GeneralSettingField {
    label: "聊天时区",
    tab: GeneralSettingsTab::Interface,
    id: "general-settings-chat-timezone",
    keywords: &["timezone"],
};
pub(in crate::app::components) const TERMINAL_SHELL: GeneralSettingField = GeneralSettingField {
    label: "终端 Shell",
    tab: GeneralSettingsTab::Terminal,
    id: "general-settings-terminal-shell",
    keywords: &[],
};
pub(in crate::app::components) const TERMINAL_THEME: GeneralSettingField = GeneralSettingField {
    label: "终端主题",
    tab: GeneralSettingsTab::Terminal,
    id: "general-settings-terminal-theme",
    keywords: &[],
};
pub(in crate::app::components) const TERMINAL_FONT: GeneralSettingField = GeneralSettingField {
    label: "终端字体",
    tab: GeneralSettingsTab::Terminal,
    id: "general-settings-terminal-font",
    keywords: &[],
};
pub(in crate::app::components) const TERMINAL_FONT_SIZE: GeneralSettingField =
    GeneralSettingField {
        label: "字体大小",
        tab: GeneralSettingsTab::Terminal,
        id: "general-settings-terminal-font-size",
        keywords: &[],
    };
pub(in crate::app::components) const EDITOR_FOLLOW_THEME: GeneralSettingField =
    GeneralSettingField {
        label: "跟随系统主题",
        tab: GeneralSettingsTab::Editor,
        id: "general-settings-editor-follow-theme",
        keywords: &[],
    };
pub(in crate::app::components) const EDITOR_THEME: GeneralSettingField = GeneralSettingField {
    label: "编辑器主题",
    tab: GeneralSettingsTab::Editor,
    id: "general-settings-editor-theme",
    keywords: &[],
};
pub(in crate::app::components) const EDITOR_FONT_SIZE: GeneralSettingField = GeneralSettingField {
    label: "字体大小",
    tab: GeneralSettingsTab::Editor,
    id: "general-settings-editor-font-size",
    keywords: &[],
};
pub(in crate::app::components) const EDITOR_LINE_HEIGHT: GeneralSettingField =
    GeneralSettingField {
        label: "行高",
        tab: GeneralSettingsTab::Editor,
        id: "general-settings-editor-line-height",
        keywords: &[],
    };
pub(in crate::app::components) const EDITOR_AUTO_LINE_HEIGHT: GeneralSettingField =
    GeneralSettingField {
        label: "自动行高",
        tab: GeneralSettingsTab::Editor,
        id: "general-settings-editor-auto-line-height",
        keywords: &[],
    };
#[cfg(not(target_arch = "wasm32"))]
pub(in crate::app::components) const EDITOR_LSP: GeneralSettingField = GeneralSettingField {
    label: "LSP 语言服务",
    tab: GeneralSettingsTab::Editor,
    id: "general-settings-editor-lsp",
    keywords: &["language server"],
};

pub(in crate::app::components) const ALL: &[&GeneralSettingField] = &[
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
