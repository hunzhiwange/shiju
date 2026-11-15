//! 系统设置与文件编辑器弹窗共用字段及消息。
use super::preview_panel::tabs::{DisplayLanguage, all_languages};
use self::fields::{self as general_fields, Field};
#[cfg(not(target_arch = "wasm32"))]
use desktop_ui_controls::system_settings_common::settings_control_with_hint;
use desktop_ui_controls::system_settings_common::{
    settings_checkbox_style, settings_divider, settings_field_label,
    settings_group_card, settings_select, settings_value_badge,
};
use desktop_i18n::I18n;
#[cfg(not(target_arch = "wasm32"))]
use iced::widget::container;
use iced::widget::{checkbox, column, row};
use iced::{Alignment, Element, Length};

/// 仅包含设置视图实际读取的值，不借用完整应用状态。
pub struct SettingsView {
    pub locale: config_ui::DesktopLocale,
    pub editor_theme: iced::Theme,
    pub follow_system_theme: bool,
    pub font_size: f32,
    pub line_height: f32,
    pub auto_line_height: bool,
    pub language: iced_code_editor::i18n::Language,
    #[cfg(not(target_arch = "wasm32"))]
    pub lsp_disabled: bool,
}

#[derive(Debug, Clone)]
pub enum Message<Help = ()> {
    Help(Help),
    ThemeChanged(iced::Theme),
    ToggleFollowSystemTheme(bool),
    FontSizePreview(f32),
    LineHeightPreview(f32),
    ToggleAutoLineHeight(bool),
    CommitMetrics,
    LanguageChanged(iced_code_editor::i18n::Language),
    #[cfg(not(target_arch = "wasm32"))]
    LspToggleDisabled(bool),
    #[cfg(not(target_arch = "wasm32"))]
    CopyLspCommand(desktop_lsp::install::Server),
    #[cfg(not(target_arch = "wasm32"))]
    InstallLsp(desktop_lsp::install::Server),
}

pub fn metrics_slider<Help: Clone + 'static>(
    range: std::ops::RangeInclusive<f32>, value: f32,
    on_preview: impl Fn(f32) -> Message<Help> + 'static,
) -> iced::widget::Slider<'static, f32, Message<Help>> {
    iced::widget::slider(range, value, on_preview).on_release(Message::CommitMetrics)
}

/// 帮助正文由调用方提供，保留跨域文本选择与复制交互。
pub fn view<Help: Clone + 'static>(
    state: SettingsView, control_width: f32,
    render_help: impl Fn(&'static str, &'static str) -> Element<'static, Help>,
) -> Element<'static, Message<Help>> {
    let i18n = I18n::new(state.locale);
    let field_row =
        |field: &Field, description, control: Element<'static, Message<Help>>| {
            settings_row_with_help(i18n.__(field.label), description, control_width, control, &render_help)
            .id(field.id)
        };
    let theme_pick = settings_select(
        iced::Theme::ALL.iter().cloned(),
        Some(state.editor_theme.clone()),
        |theme| Message::ThemeChanged(theme),
    )
    .disabled(state.follow_system_theme)
    .width(Length::Fill);

    let follow_system_theme = field_row(
        &general_fields::EDITOR_FOLLOW_THEME,
        i18n.__("启用后沿用应用当前主题。"),
        checkbox(state.follow_system_theme)
            .label(i18n.__("启用"))
            .on_toggle(|v| Message::ToggleFollowSystemTheme(v))
            .style(settings_checkbox_style)
            .into(),
    );

    let editor_theme_row = field_row(
        &general_fields::EDITOR_THEME,
        i18n.__("关闭跟随系统主题时使用的编辑器配色。"),
        theme_pick.into(),
    );

    let displayed = &state;
    let size_slider = row![
        metrics_slider(10.0..=30.0, displayed.font_size, |v| {
            Message::FontSizePreview(v)
        })
        .width(Length::Fill),
        settings_value_badge(format!("{:.0}px", displayed.font_size))
    ]
    .spacing(10)
    .align_y(Alignment::Center);

    let size_row = field_row(
        &general_fields::EDITOR_FONT_SIZE,
        i18n.__("调整编辑器主文本字号。"),
        size_slider.into(),
    );

    let line_height_slider = row![
        metrics_slider(10.0..=60.0, displayed.line_height, |v| {
            Message::LineHeightPreview(v)
        })
        .width(Length::Fill),
        settings_value_badge(format!("{:.1}px", displayed.line_height))
    ]
    .spacing(10)
    .align_y(Alignment::Center);

    let line_height_row = field_row(
        &general_fields::EDITOR_LINE_HEIGHT,
        i18n.__("调整每一行的垂直高度。"),
        line_height_slider.into(),
    );

    let auto_line_height = field_row(
        &general_fields::EDITOR_AUTO_LINE_HEIGHT,
        i18n.__("根据字体大小自动推导更合适的行高。"),
        checkbox(displayed.auto_line_height)
            .label(i18n.__("根据字体大小自动调整"))
            .on_toggle(|v| Message::ToggleAutoLineHeight(v))
            .style(settings_checkbox_style)
            .into(),
    );

    #[cfg(not(target_arch = "wasm32"))]
    let lsp_enabled = field_row(
        &general_fields::EDITOR_LSP,
        i18n.__("启用代码补全、悬停提示与跳转定义。"),
        settings_control_with_hint(
            container(
                checkbox(!state.lsp_disabled)
                    .label(i18n.__("启用"))
                    .on_toggle(|enabled| {
                        Message::LspToggleDisabled(!enabled)
                    })
                    .style(settings_checkbox_style),
            )
            .align_right(Length::Fill),
            i18n.__("需要对应语言服务器可用。"),
        ),
    );

    let language = settings_row_with_help(
        i18n.__("编辑器界面语言"),
        i18n.__("仅调整编辑器内置控件的翻译，本次运行有效。"),
        control_width,
        settings_select(
            all_languages().iter().copied(),
            Some(DisplayLanguage(state.language)),
            |language| Message::LanguageChanged(language.0),
        )
        .width(Length::Fill),
        &render_help,
    );
    let mut fields = column![
        follow_system_theme,
        settings_divider(),
        editor_theme_row,
        settings_divider(),
        size_row,
        settings_divider(),
        line_height_row,
        settings_divider(),
        auto_line_height,
        settings_divider(),
        language,
    ]
    .spacing(0);

    #[cfg(not(target_arch = "wasm32"))]
    {
        fields = fields.push(settings_divider()).push(lsp_enabled).push(lsp_install_guide(state.locale));
    }

    settings_group_card(fields).into()
}

/// Installation actions are shared by system settings and the editor settings panel.
#[cfg(not(target_arch = "wasm32"))]
fn lsp_install_guide<Help: Clone + 'static>(locale: config_ui::DesktopLocale) -> Element<'static, Message<Help>> {
    use desktop_ui_controls::system_settings_common::{primary_action_btn_style, rounded_action_btn_style};
    use desktop_lsp::install::Server;
    use iced::widget::{button, text};
    let i18n = I18n::new(locale);
    let mut guide = column![
        text(i18n.__("语言服务器安装")),
        text(i18n.__("安装在本机。快捷安装会下载软件并运行下方命令；需要联网，不会自动提权。也可复制命令到终端手工安装。")),
        text(i18n.__("前置工具：Rust 需要 rustup；Python 和 JS/TS 需要 Node.js（含 npm）；Go 需要 Go；Lua 快捷安装需要 macOS Homebrew。")),
        text("https://rustup.rs / https://nodejs.org / https://go.dev/dl / https://brew.sh"),
    ].spacing(10).padding(12);
    for server in Server::ALL {
        let command = server.manual_command();
        guide = guide.push(
            column![
                text(server.label()),
                text(command),
                row![
                    button(text(i18n.__("复制命令")))
                        .on_press(Message::CopyLspCommand(server))
                        .style(rounded_action_btn_style)
                        .padding([6, 12]),
                    button(text(i18n.__("快捷安装")))
                        .on_press_maybe(
                            (!matches!(server, Server::Lua) || cfg!(target_os = "macos"))
                                .then_some(Message::InstallLsp(server))
                        )
                        .style(primary_action_btn_style)
                        .padding([6, 12]),
                ]
                .spacing(8),
            ]
            .spacing(6),
        );
    }
    guide.push(text(i18n.__("Windows 手工安装可使用 npm.cmd。Lua 的 Windows/Linux 安装说明：https://luals.github.io/#install 。安装后确保语言服务器所在目录在 PATH 中（Go 通常为 ~/go/bin），重启 ShiJu 并重新打开文件。"))).into()
}

/// 设置表单与桌面搜索目录共用的标签和定位标识。
pub mod fields {
    pub struct Field { pub label: &'static str, pub id: &'static str }
    pub const EDITOR_FOLLOW_THEME: Field = Field { label: "跟随系统主题", id: "general-settings-editor-follow-theme" };
    pub const EDITOR_THEME: Field = Field { label: "编辑器主题", id: "general-settings-editor-theme" };
    pub const EDITOR_FONT_SIZE: Field = Field { label: "字体大小", id: "general-settings-editor-font-size" };
    pub const EDITOR_LINE_HEIGHT: Field = Field { label: "行高", id: "general-settings-editor-line-height" };
    pub const EDITOR_AUTO_LINE_HEIGHT: Field = Field { label: "自动行高", id: "general-settings-editor-auto-line-height" };
    #[cfg(not(target_arch = "wasm32"))]
    pub const EDITOR_LSP: Field = Field { label: "LSP 语言服务", id: "general-settings-editor-lsp" };
}

fn settings_row_with_help<Help: Clone + 'static>(
    label: &'static str, description: &'static str, control_width: f32,
    control: impl Into<Element<'static, Message<Help>>>,
    render_help: &impl Fn(&'static str, &'static str) -> Element<'static, Help>,
) -> iced::widget::Container<'static, Message<Help>> {
    use desktop_ui_controls::system_settings_common::{help::settings_hover_help_label, panels::settings_flexible_row_with_label};
    let label = settings_hover_help_label(
        settings_field_label(label), render_help(label, description).map(Message::Help),
    ).width(Length::Fill).wrap();
    settings_flexible_row_with_label(label, control_width, control)
}
