//! 系统设置通用页面视图模块
//!
//! 本模块提供系统设置界面中通用配置项的视图渲染功能。
//! 主要包括以下设置项：
//! - 桌面界面语言选择
//! - 应用主题选择
//! - 终端 Shell 选择
//! - 终端主题选择
//! - 终端字体及字号配置

use crate::app::components::system_settings::general_fields::{self, GeneralSettingField};
use crate::app::components::system_settings_common::{
    rounded_action_btn_style, settings_divider, settings_flexible_row,
    settings_flexible_row_with_help, settings_group_card, settings_group_heading_with_help,
    settings_page_heading_with_help, settings_select, settings_tab_button, settings_tab_row,
    settings_toggle, settings_value_badge,
};
use crate::app::components::system_settings_search::SettingsTabDescriptor;
use crate::app::i18n::I18n;
use crate::app::state::GeneralSettingsTab;
use crate::app::theme_catalog::{theme_store_from_home, to_iced_theme};
use crate::app::{App, Message, PreviewAutoSaveMode, Shell, TerminalTheme, message};
use iced::widget::{button, column, row, slider, text};
use iced::{Alignment, Element, Length, Theme};
use std::borrow::Cow;

const GENERAL_CONTROL_WIDTH: f32 = 280.0;

pub(super) const GENERAL_TABS: &[SettingsTabDescriptor<GeneralSettingsTab>] = &[
    SettingsTabDescriptor::new(GeneralSettingsTab::Interface, "界面", &[]),
    SettingsTabDescriptor::new(GeneralSettingsTab::Terminal, "终端", &[]),
    SettingsTabDescriptor::new(GeneralSettingsTab::Editor, "编辑器", &["editor", "code"]),
    SettingsTabDescriptor::new(
        GeneralSettingsTab::Themes,
        "主题",
        &["themes", "gallery", "palette", "custom", "builtin", "画廊", "配色", "色板"],
    ),
];

fn general_field_row<'a>(
    i18n: I18n,
    field: &GeneralSettingField,
    description: &'a str,
    control: impl Into<Element<'a, Message>>,
) -> Element<'a, Message> {
    settings_flexible_row(i18n.__(field.label), description, GENERAL_CONTROL_WIDTH, control)
        .id(field.id)
        .into()
}

fn general_field_with_help<'a>(
    app: &App,
    field: &GeneralSettingField,
    description: impl Into<Cow<'a, str>>,
    control: impl Into<Element<'a, Message>>,
) -> iced::widget::Container<'a, Message> {
    let i18n = I18n::new(app.desktop_locale);
    settings_flexible_row_with_help(
        i18n,
        &app.app_theme,
        i18n.__(field.label),
        description,
        GENERAL_CONTROL_WIDTH,
        control,
    )
    .id(field.id)
}

fn interface_field_row<'a>(
    app: &App,
    field: &GeneralSettingField,
    description: impl Into<Cow<'a, str>>,
    control: impl Into<Element<'a, Message>>,
) -> Element<'a, Message> {
    general_field_with_help(app, field, description, control).padding([12, 0]).into()
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
    Message::Terminal(message::TerminalMessage::FontSizePreview(font_size))
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
    let i18n = I18n::new(app.desktop_locale);
    let locales = ["简体中文", "繁體中文", "English"];
    let locale_index = match app.desktop_locale {
        vw_config_types::ui::DesktopLocale::SimplifiedChinese => 0,
        vw_config_types::ui::DesktopLocale::TraditionalChinese => 1,
        vw_config_types::ui::DesktopLocale::English => 2,
    };
    let locale_pick = settings_select(locales, Some(locales[locale_index]), |label| {
        let locale = match label {
            "繁體中文" => vw_config_types::ui::DesktopLocale::TraditionalChinese,
            "English" => vw_config_types::ui::DesktopLocale::English,
            _ => vw_config_types::ui::DesktopLocale::SimplifiedChinese,
        };
        Message::Settings(message::SettingsMessage::General(
            message::GeneralSettingsMessage::LocaleSelected(locale),
        ))
    })
    .width(GENERAL_CONTROL_WIDTH);

    let shell_pick =
        settings_select(Shell::all(), Some(app.terminal.shell), terminal_shell_selected)
            .width(GENERAL_CONTROL_WIDTH);

    let term_theme_pick =
        settings_select([i18n.__("跟随系统")], Some(i18n.__("跟随系统")), |_| {
            terminal_theme_selected(TerminalTheme::System)
        })
        .width(GENERAL_CONTROL_WIDTH);

    let font_pick = settings_select(
        [
            "JetBrains Mono".to_string(),
            "Fira Code".to_string(),
            "Menlo".to_string(),
            "Monaco".to_string(),
        ],
        Some(app.terminal.font_family.clone()),
        terminal_font_selected,
    )
    .width(GENERAL_CONTROL_WIDTH);

    let auto_save_options: Vec<_> =
        PreviewAutoSaveMode::ALL.into_iter().map(|mode| i18n.__(mode.label())).collect();
    let auto_save_pick = settings_select(
        auto_save_options,
        Some(i18n.__(app.preview_auto_save_mode.label())),
        move |label| {
            let mode = PreviewAutoSaveMode::ALL
                .into_iter()
                .find(|mode| i18n.__(mode.label()) == label)
                .expect("selected auto-save option belongs to the supplied list");
            preview_auto_save_mode_changed(mode)
        },
    )
    .width(GENERAL_CONTROL_WIDTH);

    let timezone_pick = settings_select(
        vw_config_types::ui::timezone_options().into_iter().map(str::to_string).collect::<Vec<_>>(),
        Some(app.chat_timezone.clone()),
        timezone_selected,
    )
    .width(GENERAL_CONTROL_WIDTH);

    let launch_toggle = settings_toggle(app.launch_at_login).on_toggle_maybe(
        (!app.launch_at_login_busy && !cfg!(target_arch = "wasm32")).then_some(|enabled| {
            Message::Settings(message::SettingsMessage::General(
                message::GeneralSettingsMessage::LaunchAtLoginToggled(enabled),
            ))
        }),
    );
    let tray_toggle = settings_toggle(app.close_to_tray).on_toggle_maybe(
        (!cfg!(target_arch = "wasm32")).then_some(|enabled| {
            Message::Settings(message::SettingsMessage::General(
                message::GeneralSettingsMessage::CloseToTrayToggled(enabled),
            ))
        }),
    );

    let displayed_font_size =
        app.terminal_config_persistence.displayed_font_size(app.terminal.font_size);
    let size_slider = slider(10.0..=18.0, displayed_font_size, terminal_font_size_changed)
        .on_release(Message::Terminal(message::TerminalMessage::FontSizePreviewCommit))
        .width(Length::Fill);

    let active_tab = app.general_settings_tab;
    let tabs = settings_tab_row(GENERAL_TABS.iter().map(|descriptor| {
        settings_tab_button(
            i18n.__(descriptor.label),
            descriptor.tab == active_tab,
            Some(Message::Settings(message::SettingsMessage::General(
                message::GeneralSettingsMessage::TabSelected(descriptor.tab),
            ))),
        )
        .into()
    }))
    .wrap();

    let content: Element<'_, Message> = match active_tab {
        GeneralSettingsTab::Interface => column![
            settings_group_heading_with_help(
                i18n,
                &app.app_theme,
                i18n.__("界面"),
                i18n.__("应用外观与预览行为。"),
            ),
            settings_group_card(
                column![
                    interface_field_row(
                        app,
                        &general_fields::LAUNCH_ON_LOGIN,
                        i18n.__("随系统自动启动识矩，仅对当前设备生效。"),
                        launch_toggle,
                    ),
                    settings_divider(),
                    interface_field_row(
                        app,
                        &general_fields::CLOSE_TO_TRAY,
                        i18n.__(if cfg!(target_os = "macos") {
                            "点击红色关闭按钮后继续运行；从屏幕顶部菜单栏的识矩图标显示窗口或退出。"
                        } else {
                            "关闭主窗口后在后台运行，可从托盘显示窗口或退出。"
                        }),
                        tray_toggle,
                    ),
                    settings_divider(),
                    interface_field_row(
                        app,
                        &general_fields::LANGUAGE,
                        i18n.__("当前桌面端使用的界面语言。"),
                        locale_pick,
                    ),
                    settings_divider(),
                    interface_field_row(
                        app,
                        &general_fields::APP_THEME,
                        i18n.__("快速切换内置、预设和自定义主题；完整管理见「主题」标签。"),
                        settings_select(
                            app_theme_options(app),
                            Some(app.app_theme.clone()),
                            app_theme_selected,
                        )
                        .width(GENERAL_CONTROL_WIDTH),
                    ),
                    settings_divider(),
                    interface_field_row(
                        app,
                        &general_fields::AUTO_SAVE,
                        format!(
                            "{}\n{}",
                            i18n.__("控制预览编辑器自动保存的时机。"),
                            i18n.__(app.preview_auto_save_mode.description()),
                        ),
                        auto_save_pick,
                    ),
                    settings_divider(),
                    interface_field_row(
                        app,
                        &general_fields::TASK_NOTIFICATION,
                        i18n.__(if crate::app::desktop_notification::SUPPORTED {
                            "本轮对话成功完成后发送系统通知，需在系统中允许通知。"
                        } else {
                            "当前仅支持 macOS 和 Windows 桌面端。"
                        }),
                        row![
                            button(text(i18n.__("发送测试通知")).size(13))
                                .on_press_maybe(crate::app::desktop_notification::SUPPORTED.then_some(
                                    Message::Settings(message::SettingsMessage::General(
                                        message::GeneralSettingsMessage::TestTaskCompletionNotification,
                                    )),
                                ))
                                .padding([8, 14])
                                .style(rounded_action_btn_style),
                            settings_toggle(app.task_completion_notification)
                                .on_toggle_maybe(crate::app::desktop_notification::SUPPORTED.then_some(
                                    |enabled| Message::Settings(message::SettingsMessage::General(
                                        message::GeneralSettingsMessage::TaskCompletionNotificationToggled(enabled),
                                    )),
                                )),
                        ]
                        .spacing(12)
                        .align_y(Alignment::Center)
                        .wrap(),
                    ),
                    settings_divider(),
                    interface_field_row(
                        app,
                        &general_fields::CHAT_TIMEZONE,
                        i18n.__("用于模型时间上下文，新会话生效。"),
                        timezone_pick,
                    ),
                ]
                .spacing(0)
            ),
        ]
        .spacing(16)
        .into(),
        GeneralSettingsTab::Terminal => column![
            settings_group_heading_with_help(
                i18n, &app.app_theme, i18n.__("终端"), i18n.__("默认 Shell、主题与字体。")
            ),
            settings_group_card(
                column![
                    general_field_with_help(
                        app,
                        &general_fields::TERMINAL_SHELL,
                        i18n.__("新终端默认使用的 Shell。"),
                        shell_pick
                    ),
                    settings_divider(),
                    general_field_row(
                        i18n,
                        &general_fields::TERMINAL_THEME,
                        i18n.__("当前仅跟随系统主题。"),
                        term_theme_pick
                    ),
                    settings_divider(),
                    general_field_with_help(
                        app,
                        &general_fields::TERMINAL_FONT,
                        i18n.__("选择终端字体家族。"),
                        font_pick,
                    ),
                    settings_divider(),
                    general_field_with_help(
                        app,
                        &general_fields::TERMINAL_FONT_SIZE,
                        i18n.__("在 10pt 到 18pt 之间调整终端字号。"),
                        row![
                            size_slider,
                            settings_value_badge(format!("{} pt", displayed_font_size as u32))
                        ]
                        .spacing(12)
                        .width(GENERAL_CONTROL_WIDTH)
                        .align_y(Alignment::Center),
                    ),
                ]
                .spacing(0)
            ),
        ]
        .spacing(16)
        .into(),
        GeneralSettingsTab::Editor => crate::app::components::system_settings_editor::content(app),
        GeneralSettingsTab::Themes => crate::app::components::system_settings_themes::view(app),
    };

    column![
        settings_page_heading_with_help(
            i18n,
            &app.app_theme,
            i18n.__("常规"),
            i18n.__("通过标签切换界面、终端、编辑器与主题配置。"),
        ),
        tabs,
        content,
    ]
    .spacing(28)
    .padding([8, 7])
    .max_width(1040)
    .into()
}
#[cfg(test)]
#[path = "system_settings_general_tests.rs"]
mod system_settings_general_tests;
