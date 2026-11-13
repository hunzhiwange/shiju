//! 系统设置与文件编辑器弹窗共用字段及消息。
use super::preview_panel::tabs::{DisplayLanguage, all_languages};
use super::system_settings_editor::metrics_slider;
use crate::app::components::system_settings::general_fields::{self, GeneralSettingField};
#[cfg(not(target_arch = "wasm32"))]
use crate::app::components::system_settings_common::settings_control_with_hint;
use crate::app::components::system_settings_common::{
    settings_checkbox_style, settings_divider, settings_flexible_row_with_help,
    settings_group_card, settings_select, settings_value_badge,
};
use crate::app::i18n::I18n;
use crate::app::message::editor::{EditorMessage, metrics};
use crate::app::{App, Message, message};
#[cfg(not(target_arch = "wasm32"))]
use iced::widget::container;
use iced::widget::{checkbox, column, row};
use iced::{Alignment, Element, Length};

pub(crate) fn view(app: &App, control_width: f32) -> Element<'_, Message> {
    let i18n = I18n::new(app.desktop_locale);
    let field_row =
        |field: &GeneralSettingField, description, control: Element<'static, Message>| {
            settings_flexible_row_with_help(
                i18n,
                &app.app_theme,
                i18n.__(field.label),
                description,
                control_width,
                control,
            )
            .id(field.id)
        };
    let theme_pick = settings_select(
        iced::Theme::ALL.iter().cloned(),
        Some(app.effective_editor_theme()),
        |theme| Message::Editor(EditorMessage::ThemeChanged(theme)),
    )
    .disabled(app.editor_follow_system_theme)
    .width(Length::Fill);

    let follow_system_theme = field_row(
        &general_fields::EDITOR_FOLLOW_THEME,
        i18n.__("启用后沿用应用当前主题。"),
        checkbox(app.editor_follow_system_theme)
            .label(i18n.__("启用"))
            .on_toggle(|v| Message::Editor(message::EditorMessage::ToggleFollowSystemTheme(v)))
            .style(settings_checkbox_style)
            .into(),
    );

    let editor_theme_row = field_row(
        &general_fields::EDITOR_THEME,
        i18n.__("关闭跟随系统主题时使用的编辑器配色。"),
        theme_pick.into(),
    );

    let displayed = metrics::displayed(app);
    let size_slider = row![
        metrics_slider(10.0..=30.0, displayed.font_size, |v| {
            Message::Editor(message::EditorMessage::FontSizePreview(v))
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
            Message::Editor(message::EditorMessage::LineHeightPreview(v))
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
            .on_toggle(|v| Message::Editor(message::EditorMessage::ToggleAutoLineHeight(v)))
            .style(settings_checkbox_style)
            .into(),
    );

    #[cfg(not(target_arch = "wasm32"))]
    let lsp_enabled = field_row(
        &general_fields::EDITOR_LSP,
        i18n.__("启用代码补全、悬停提示与跳转定义。"),
        settings_control_with_hint(
            container(
                checkbox(!app.lsp_disabled)
                    .label(i18n.__("启用"))
                    .on_toggle(|enabled| {
                        Message::Preview(message::PreviewMessage::LspToggleDisabled(!enabled))
                    })
                    .style(settings_checkbox_style),
            )
            .align_right(Length::Fill),
            i18n.__("需要对应语言服务器可用。"),
        ),
    );

    let language = settings_flexible_row_with_help(
        i18n,
        &app.app_theme,
        i18n.__("编辑器界面语言"),
        i18n.__("仅调整编辑器内置控件的翻译，本次运行有效。"),
        control_width,
        settings_select(
            all_languages().iter().copied(),
            Some(DisplayLanguage(app.current_language)),
            |language| Message::Editor(EditorMessage::LanguageChanged(language.0)),
        )
        .width(Length::Fill),
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
        fields = fields.push(settings_divider()).push(lsp_enabled).push(lsp_install_guide(app));
    }

    settings_group_card(fields).into()
}

/// Installation actions are shared by system settings and the editor settings panel.
#[cfg(not(target_arch = "wasm32"))]
fn lsp_install_guide(app: &App) -> Element<'_, Message> {
    use super::system_settings_common::{primary_action_btn_style, rounded_action_btn_style};
    use crate::app::lsp::install::Server;
    use iced::widget::{button, text};
    let i18n = I18n::new(app.desktop_locale);
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
                        .on_press(Message::Editor(EditorMessage::CopyLspCommand(server)))
                        .style(rounded_action_btn_style)
                        .padding([6, 12]),
                    button(text(i18n.__("快捷安装")))
                        .on_press_maybe(
                            (!matches!(server, Server::Lua) || cfg!(target_os = "macos"))
                                .then_some(Message::Editor(EditorMessage::InstallLsp(server)))
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
