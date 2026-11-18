//! 系统设置与文件编辑器弹窗共用字段及消息。
use self::fields::{self as general_fields, Field};
use super::preview_panel::tabs::{DisplayLanguage, all_languages};
use desktop_i18n::I18n;
#[cfg(not(target_arch = "wasm32"))]
use desktop_lsp::install::Server;
use desktop_ui_controls::system_settings_common::{
    help::settings_hover_help_label, panels::settings_flexible_row_with_label,
};
use desktop_ui_controls::system_settings_common::{
    settings_checkbox_style, settings_divider, settings_field_label, settings_group_card,
    settings_select, settings_value_badge,
};
use iced::widget::{checkbox, column, row, text};
use iced::{Alignment, Element, Length};
use std::borrow::Cow;

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
    #[cfg(not(target_arch = "wasm32"))]
    pub copied_lsp_command: Option<Server>,
    #[cfg(not(target_arch = "wasm32"))]
    pub installing_lsp: Option<Server>,
    #[cfg(not(target_arch = "wasm32"))]
    pub installed_lsp: Vec<Server>,
    #[cfg(not(target_arch = "wasm32"))]
    pub activity_frame: usize,
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
    range: std::ops::RangeInclusive<f32>,
    value: f32,
    on_preview: impl Fn(f32) -> Message<Help> + 'static,
) -> iced::widget::Slider<'static, f32, Message<Help>> {
    iced::widget::slider(range, value, on_preview).on_release(Message::CommitMetrics)
}

/// 帮助正文由调用方提供，保留跨域文本选择与复制交互。
pub fn view<Help: Clone + 'static>(
    state: SettingsView,
    control_width: f32,
    render_help: impl Fn(&'static str, Cow<'static, str>) -> Element<'static, Help>,
) -> Element<'static, Message<Help>> {
    let i18n = I18n::new(state.locale);
    let field_row =
        |field: &Field, description: &'static str, control: Element<'static, Message<Help>>| {
            settings_row_with_help(
                i18n.__(field.label),
                description,
                control_width,
                control,
                &render_help,
            )
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
        metrics_slider(10.0..=30.0, displayed.font_size, |v| { Message::FontSizePreview(v) })
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
        metrics_slider(10.0..=60.0, displayed.line_height, |v| { Message::LineHeightPreview(v) })
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
    let display_fields = column![
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

    let group_heading = |title: &'static str, description: Cow<'static, str>| {
        settings_hover_help_label(
            text(title).size(17),
            render_help(title, description).map(Message::Help),
        )
        .wrap()
    };
    let groups = column![
        group_heading(i18n.__("主题与显示"), i18n.__("统一编辑器主题来源及排版参数。").into(),),
        settings_group_card(display_fields),
    ]
    .spacing(16)
    .width(Length::Fill);

    #[cfg(not(target_arch = "wasm32"))]
    let groups = {
        use desktop_ui_controls::system_settings_common::settings_toggle;

        let lsp_enabled = settings_toggle(!state.lsp_disabled)
            .on_toggle(|enabled| Message::LspToggleDisabled(!enabled));
        let lsp_row = settings_flexible_row_with_label(
            settings_field_label(i18n.__("启用 LSP")).width(Length::Fill),
            control_width,
            lsp_enabled,
        )
        .id(general_fields::EDITOR_LSP.id);
        let lsp_help = format!(
            "{} {}",
            i18n.__("启用代码补全、悬停提示与跳转定义。"),
            i18n.__("需要对应语言服务器可用。"),
        );
        let install_help = format!(
            "{}\n\n{}",
            i18n.__("快捷安装需要联网，不会自动提权。也可复制各语言的命令，手动在终端运行。"),
            i18n.__("安装后确保语言服务器所在目录在 PATH 中，重启 ShiJu 并重新打开文件。"),
        );
        groups
            .push(group_heading(i18n.__(general_fields::EDITOR_LSP.label), lsp_help.into()))
            .push(settings_group_card(lsp_row))
            .push(group_heading(i18n.__("语言服务器安装"), install_help.into()))
            .push(lsp_install_guide(&state, i18n, control_width, &render_help))
    };

    groups.into()
}

#[cfg(not(target_arch = "wasm32"))]
fn lsp_install_guide<Help: Clone + 'static>(
    state: &SettingsView,
    i18n: I18n,
    control_width: f32,
    render_help: &impl Fn(&'static str, Cow<'static, str>) -> Element<'static, Help>,
) -> Element<'static, Message<Help>> {
    use desktop_ui_controls::status_animation::activity_spinner;
    use desktop_ui_controls::system_settings_common::{
        SETTINGS_CONTROL_TEXT_SIZE, primary_action_btn_style, rounded_action_btn_style,
        settings_success_banner_style,
    };
    use iced::widget::{button, container, svg, text::Wrapping};

    let mut servers = column![].width(Length::Fill);
    for (index, server) in Server::ALL.into_iter().enumerate() {
        if index > 0 {
            servers = servers.push(settings_divider());
        }
        let copy_label = if state.copied_lsp_command == Some(server) {
            i18n.__("已复制")
        } else {
            i18n.__("复制命令")
        };
        let installing = state.installing_lsp == Some(server);
        let install_label =
            if installing { i18n.__("安装中…") } else { i18n.__("快捷安装") };
        let can_install = state.installing_lsp.is_none()
            && (!matches!(server, Server::Lua) || cfg!(target_os = "macos"));
        let actions = row![
            button(text(copy_label).size(SETTINGS_CONTROL_TEXT_SIZE))
                .on_press(Message::CopyLspCommand(server))
                .style(rounded_action_btn_style)
                .padding([6, 12]),
            button(text(install_label).size(SETTINGS_CONTROL_TEXT_SIZE))
                .on_press_maybe(can_install.then_some(Message::InstallLsp(server)))
                .style(primary_action_btn_style)
                .padding([6, 12]),
        ]
        .spacing(8)
        .wrap();
        let requirement = match server {
            Server::Rust => i18n.__("需要 rustup。https://rustup.rs"),
            Server::Python | Server::TypeScript => i18n.__(
                "需要 Node.js（含 npm）。https://nodejs.org\nWindows 手工安装可使用 npm.cmd。",
            ),
            Server::Go => i18n.__("需要 Go。https://go.dev/dl\n语言服务器通常安装到 ~/go/bin。"),
            Server::Lua => i18n.__(
                "快捷安装需要 macOS Homebrew。https://brew.sh\nWindows 和 Linux 安装说明见 https://luals.github.io/#install。",
            ),
        };
        let command = server.manual_command();
        let name = server.label();
        let mut label = settings_hover_help_label(
            settings_field_label(name),
            render_help(name, format!("{command}\n\n{requirement}").into()).map(Message::Help),
        )
        .spacing(6)
        .align_y(Alignment::Center)
        .width(Length::Fill);
        if state.installed_lsp.contains(&server) {
            label = label.push(
                container(text(i18n.__("已安装")).size(12))
                    .padding([3, 8])
                    .style(settings_success_banner_style),
            );
        }
        servers =
            servers.push(settings_flexible_row_with_label(label.wrap(), control_width, actions));
        if installing {
            let spinner =
                activity_spinner(state.activity_frame, 16.0).style(|theme: &iced::Theme, _| {
                    svg::Style { color: Some(theme.extended_palette().warning.base.color) }
                });
            let status = container(
                row![
                    spinner,
                    column![
                        text(i18n.__("正在执行的命令")).size(14),
                        text(command)
                            .size(13)
                            .font(iced::Font::MONOSPACE)
                            .wrapping(Wrapping::WordOrGlyph),
                    ]
                    .spacing(3)
                    .width(Length::Fill),
                ]
                .spacing(8)
                .align_y(Alignment::Center),
            )
            .padding([9, 12])
            .width(Length::Fill)
            .style(|theme: &iced::Theme| {
                let color = theme.extended_palette().warning.base.color;
                container::Style {
                    text_color: Some(theme.palette().text),
                    background: Some(color.scale_alpha(0.10).into()),
                    border: iced::Border {
                        color: color.scale_alpha(0.55),
                        width: 1.0,
                        radius: 8.0.into(),
                    },
                    ..Default::default()
                }
            });
            servers = servers.push(
                container(status).padding(iced::Padding { bottom: 12.0, ..iced::Padding::ZERO }),
            );
        }
    }
    settings_group_card(servers).into()
}

/// 设置表单与桌面搜索目录共用的标签和定位标识。
pub mod fields {
    pub struct Field {
        pub label: &'static str,
        pub id: &'static str,
    }
    pub const EDITOR_FOLLOW_THEME: Field =
        Field { label: "跟随系统主题", id: "general-settings-editor-follow-theme" };
    pub const EDITOR_THEME: Field =
        Field { label: "编辑器主题", id: "general-settings-editor-theme" };
    pub const EDITOR_FONT_SIZE: Field =
        Field { label: "字体大小", id: "general-settings-editor-font-size" };
    pub const EDITOR_LINE_HEIGHT: Field =
        Field { label: "行高", id: "general-settings-editor-line-height" };
    pub const EDITOR_AUTO_LINE_HEIGHT: Field =
        Field { label: "自动行高", id: "general-settings-editor-auto-line-height" };
    #[cfg(not(target_arch = "wasm32"))]
    pub const EDITOR_LSP: Field =
        Field { label: "LSP 语言服务", id: "general-settings-editor-lsp" };
}

fn settings_row_with_help<Help: Clone + 'static>(
    label: &'static str,
    description: impl Into<Cow<'static, str>>,
    control_width: f32,
    control: impl Into<Element<'static, Message<Help>>>,
    render_help: &impl Fn(&'static str, Cow<'static, str>) -> Element<'static, Help>,
) -> iced::widget::Container<'static, Message<Help>> {
    let label = settings_hover_help_label(
        settings_field_label(label),
        render_help(label, description.into()).map(Message::Help),
    )
    .width(Length::Fill)
    .wrap();
    settings_flexible_row_with_label(label, control_width, control)
}

#[cfg(test)]
#[path = "editor_settings_fields_tests.rs"]
mod tests;
