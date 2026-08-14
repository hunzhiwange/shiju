//! 系统设置通用页面视图模块
//!
//! 本模块提供系统设置界面中通用配置项的视图渲染功能。
//! 主要包括以下设置项：
//! - 语言选择（当前固定为简体中文）
//! - 应用主题选择
//! - 终端 Shell 选择
//! - 终端主题选择
//! - 终端字体及字号配置

use crate::app::components::system_settings_common::{
    SETTINGS_LABEL_WIDTH, settings_divider, settings_muted_text_style, settings_page_intro,
    settings_panel, settings_pick_list_menu_style, settings_pick_list_style, settings_section_card,
    settings_value_badge,
};
use crate::app::theme_catalog::{theme_store_from_home, to_iced_theme};
use crate::app::{App, Message, PreviewAutoSaveMode, Shell, TerminalTheme, message};
use iced::widget::{column, container, pick_list, row, slider, text};
use iced::{Alignment, Element, Length, Theme};

fn setting_row<'a>(
    label: &'static str,
    description: &'static str,
    control: impl Into<Element<'a, Message>>,
) -> Element<'a, Message> {
    container(
        row![
            column![
                text(label).size(13),
                text(description).size(11).style(settings_muted_text_style),
            ]
            .spacing(4)
            .width(Length::Fixed(SETTINGS_LABEL_WIDTH)),
            container(control.into()).width(Length::Fill),
        ]
        .spacing(22)
        .align_y(Alignment::Center),
    )
    .padding([14, 0])
    .width(Length::Fill)
    .into()
}

fn app_theme_selected(theme: iced::Theme) -> Message {
    Message::View(message::ViewMessage::AppThemeSelected(theme))
}

/// 返回常规设置可选择的全部应用主题：Iced 内置主题与本地预设/自定义主题。
fn app_theme_options(app: &App) -> Vec<Theme> {
    let mut themes: Vec<Theme> = Theme::ALL.iter().cloned().collect();
    let Some(store) = theme_store_from_home() else {
        return themes;
    };

    match store.list() {
        Ok(definitions) => {
            for definition in definitions {
                match to_iced_theme(&definition) {
                    Ok(theme) => themes.push(theme),
                    Err(err) => tracing::warn!(
                        target: "vw_desktop",
                        theme_id = %definition.id,
                        error = %err,
                        "skipping invalid local theme in general settings picker"
                    ),
                }
            }
        }
        Err(err) => tracing::warn!(
            target: "vw_desktop",
            error = %err,
            "failed to load local themes for general settings picker"
        ),
    }

    if !themes.contains(&app.app_theme) {
        themes.push(app.app_theme.clone());
    }
    themes
}

fn terminal_shell_selected(shell: Shell) -> Message {
    Message::Terminal(message::TerminalMessage::ShellSelected(shell))
}

fn terminal_theme_selected(theme: TerminalTheme) -> Message {
    Message::Terminal(message::TerminalMessage::ThemeSelected(theme))
}

fn terminal_font_selected(font_family: String) -> Message {
    Message::Terminal(message::TerminalMessage::FontSelected(font_family))
}

fn preview_auto_save_mode_changed(mode: PreviewAutoSaveMode) -> Message {
    Message::Preview(message::PreviewMessage::AutoSaveModeChanged(mode))
}

fn timezone_selected(timezone: String) -> Message {
    Message::Settings(message::SettingsMessage::General(
        message::GeneralSettingsMessage::TimezoneSelected(timezone),
    ))
}

fn terminal_font_size_changed(font_size: f32) -> Message {
    Message::Terminal(message::TerminalMessage::FontSizeChanged(font_size))
}

/// 渲染系统设置通用页面的视图
///
/// 该函数构建并返回包含所有通用系统设置选项的 UI 元素。
/// 用户可以通过此界面配置应用和终端的各项设置。
///
/// # 参数
///
/// * `app` - 应用状态的可变引用，包含当前所有配置值
///
/// # 返回值
///
/// 返回一个 `Element`，包含完整的通用设置页面 UI 组件
///
/// # 示例
///
/// ```ignore
/// let settings_view = view(&app);
/// // 返回的 Element 可直接用于 iced 应用的视图渲染
/// ```
pub fn view(app: &App) -> Element<'_, Message> {
    let theme_pick =
        pick_list(app_theme_options(app), Some(app.app_theme.clone()), app_theme_selected)
            .padding([10, 14])
            .text_size(13)
            .style(settings_pick_list_style)
            .menu_style(settings_pick_list_menu_style)
            .width(Length::Fixed(280.0));

    let shell_pick = pick_list(Shell::all(), Some(app.terminal.shell), terminal_shell_selected)
        .padding([10, 14])
        .text_size(13)
        .style(settings_pick_list_style)
        .menu_style(settings_pick_list_menu_style)
        .width(Length::Fixed(280.0));

    let term_theme_pick =
        pick_list([TerminalTheme::System], Some(app.terminal.theme), terminal_theme_selected)
            .padding([10, 14])
            .text_size(13)
            .style(settings_pick_list_style)
            .menu_style(settings_pick_list_menu_style)
            .width(Length::Fixed(280.0));

    let font_pick = pick_list(
        [
            "JetBrains Mono".to_string(),
            "Fira Code".to_string(),
            "Menlo".to_string(),
            "Monaco".to_string(),
        ],
        Some(app.terminal.font_family.clone()),
        terminal_font_selected,
    )
    .padding([10, 14])
    .text_size(13)
    .style(settings_pick_list_style)
    .menu_style(settings_pick_list_menu_style)
    .width(Length::Fixed(280.0));

    let auto_save_pick = pick_list(
        PreviewAutoSaveMode::ALL,
        Some(app.preview_auto_save_mode),
        preview_auto_save_mode_changed,
    )
    .padding([10, 14])
    .text_size(13)
    .style(settings_pick_list_style)
    .menu_style(settings_pick_list_menu_style)
    .width(Length::Fixed(280.0));

    let timezone_pick = pick_list(
        vw_config_types::ui::timezone_options().into_iter().map(str::to_string).collect::<Vec<_>>(),
        Some(app.chat_timezone.clone()),
        timezone_selected,
    )
    .padding([10, 14])
    .text_size(13)
    .style(settings_pick_list_style)
    .menu_style(settings_pick_list_menu_style)
    .width(Length::Fixed(280.0));

    let size_slider =
        slider(10.0..=18.0, app.terminal.font_size, terminal_font_size_changed).width(Length::Fill);

    column![
        settings_page_intro("常规", "统一管理界面语言、主题与终端基础体验。"),
        settings_section_card("界面", "应用外观与预览行为。"),
        settings_panel(
            column![
                setting_row(
                    "语言",
                    "当前桌面端使用的界面语言。",
                    settings_value_badge("简体中文"),
                ),
                settings_divider(),
                setting_row(
                    "应用主题",
                    "快速切换内置、预设和自定义主题；完整管理见「主题」页。",
                    theme_pick,
                ),
                settings_divider(),
                setting_row(
                    "自动保存",
                    "控制预览编辑器自动保存的时机。",
                    column![
                        auto_save_pick,
                        text(app.preview_auto_save_mode.description())
                            .size(11)
                            .style(settings_muted_text_style),
                    ]
                    .spacing(8),
                ),
                settings_divider(),
                setting_row(
                    "聊天时区",
                    "用于模型时间上下文，新会话生效。",
                    timezone_pick,
                ),
            ]
            .spacing(0)
        ),
        settings_section_card("终端", "默认 Shell、主题与字体。"),
        settings_panel(
            column![
                setting_row("终端 Shell", "新终端默认使用的 Shell。", shell_pick),
                settings_divider(),
                setting_row("终端主题", "当前仅跟随系统主题。", term_theme_pick),
                settings_divider(),
                setting_row("终端字体", "选择终端字体家族。", font_pick),
                settings_divider(),
                setting_row(
                    "字体大小",
                    "在 10pt 到 18pt 之间调整终端字号。",
                    row![size_slider, settings_value_badge(format!("{} pt", app.terminal.font_size as u32))]
                        .spacing(12)
                        .align_y(Alignment::Center),
                ),
            ]
            .spacing(0)
        ),
    ]
    .spacing(16)
    .into()
}
#[cfg(test)]
#[path = "system_settings_general_tests.rs"]
mod system_settings_general_tests;
