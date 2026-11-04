//! 系统设置通用页面视图模块
//!
//! 本模块提供系统设置界面中通用配置项的视图渲染功能。
//! 主要包括以下设置项：
//! - 桌面界面语言选择
//! - 应用主题选择
//! - 终端 Shell 选择
//! - 终端主题选择
//! - 终端字体及字号配置

use crate::app::components::system_settings_common::{
    SETTINGS_LABEL_WIDTH, rounded_action_btn_style, settings_checkbox_style, settings_divider,
    settings_muted_text_style, settings_page_intro, settings_pick_list_menu_style,
    settings_pick_list_style, settings_section_card, settings_section_content,
    settings_segment_button_style, settings_value_badge,
};
use crate::app::components::system_settings_search::SettingsTabDescriptor;
use crate::app::state::GeneralSettingsTab;
use crate::app::theme_catalog::{theme_store_from_home, to_iced_theme};
use crate::app::{App, Message, PreviewAutoSaveMode, Shell, TerminalTheme, message};
use iced::widget::{button, checkbox, column, container, pick_list, row, slider, text};
use iced::{Alignment, Element, Length, Theme};

pub(super) const GENERAL_TABS: &[SettingsTabDescriptor<GeneralSettingsTab>] = &[
    SettingsTabDescriptor::new(
        GeneralSettingsTab::Interface,
        "界面",
        &[
            "语言",
            "应用主题",
            "自动保存",
            "开机启动",
            "关闭到托盘",
            "后台运行",
            "startup",
            "tray",
            "preview auto save",
            "通知",
            "任务完成",
            "notification",
        ],
    ),
    SettingsTabDescriptor::new(
        GeneralSettingsTab::Terminal,
        "终端",
        &["Shell", "终端主题", "终端字体", "字体大小"],
    ),
    SettingsTabDescriptor::new(
        GeneralSettingsTab::Editor,
        "编辑器",
        &["editor", "code", "编辑器主题", "行高"],
    ),
    SettingsTabDescriptor::new(
        GeneralSettingsTab::Themes,
        "主题",
        &["themes", "gallery", "palette", "custom", "builtin", "画廊", "配色", "色板"],
    ),
];

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
    let i18n = crate::app::i18n::I18n::new(app.desktop_locale);
    let locales = ["简体中文", "繁體中文", "English"];
    let locale_index = match app.desktop_locale {
        vw_config_types::ui::DesktopLocale::SimplifiedChinese => 0,
        vw_config_types::ui::DesktopLocale::TraditionalChinese => 1,
        vw_config_types::ui::DesktopLocale::English => 2,
    };
    let locale_pick = pick_list(locales, Some(locales[locale_index]), |label| {
        let locale = match label {
            "繁體中文" => vw_config_types::ui::DesktopLocale::TraditionalChinese,
            "English" => vw_config_types::ui::DesktopLocale::English,
            _ => vw_config_types::ui::DesktopLocale::SimplifiedChinese,
        };
        Message::Settings(message::SettingsMessage::General(
            message::GeneralSettingsMessage::LocaleSelected(locale),
        ))
    })
    .padding([10, 14])
    .text_size(13)
    .style(settings_pick_list_style)
    .menu_style(settings_pick_list_menu_style)
    .width(Length::Fixed(280.0));
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
        pick_list([i18n.__("跟随系统")], Some(i18n.__("跟随系统")), |_| {
            terminal_theme_selected(TerminalTheme::System)
        })
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

    let auto_save_options: Vec<_> =
        PreviewAutoSaveMode::ALL.into_iter().map(|mode| i18n.__(mode.label())).collect();
    let auto_save_pick = pick_list(
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

    let launch_toggle = checkbox(app.launch_at_login)
        .label(i18n.__("开机启动"))
        .on_toggle_maybe((!app.launch_at_login_busy && !cfg!(target_arch = "wasm32")).then_some(
            |enabled| {
                Message::Settings(message::SettingsMessage::General(
                    message::GeneralSettingsMessage::LaunchAtLoginToggled(enabled),
                ))
            },
        ))
        .style(settings_checkbox_style);
    let tray_toggle = checkbox(app.close_to_tray)
        .label(i18n.__("关闭到托盘"))
        .on_toggle_maybe((!cfg!(target_arch = "wasm32")).then_some(|enabled| {
            Message::Settings(message::SettingsMessage::General(
                message::GeneralSettingsMessage::CloseToTrayToggled(enabled),
            ))
        }))
        .style(settings_checkbox_style);

    let size_slider =
        slider(10.0..=18.0, app.terminal.font_size, terminal_font_size_changed).width(Length::Fill);

    let active_tab = app.general_settings_tab;
    let tab_button = |tab, label| {
        let is_active = tab == active_tab;
        button(text(label).size(13))
            .on_press(Message::Settings(message::SettingsMessage::General(
                message::GeneralSettingsMessage::TabSelected(tab),
            )))
            .padding([8, 14])
            .style(move |theme: &Theme, status| {
                settings_segment_button_style(theme, status, is_active)
            })
    };
    let tabs = GENERAL_TABS
        .iter()
        .fold(row![].spacing(8).align_y(Alignment::Center), |row, descriptor| {
            row.push(tab_button(descriptor.tab, i18n.__(descriptor.label)))
        });

    let content: Element<'_, Message> = match active_tab {
        GeneralSettingsTab::Interface => column![
            settings_section_card(i18n.__("界面"), i18n.__("应用外观与预览行为。")),
            settings_section_content(
                column![
                    setting_row(
                        i18n.__("开机启动"),
                        i18n.__(if cfg!(target_os = "macos") {
                            "需从打包后的 .app 启用，可在系统设置「通用 → 登录项」查看。"
                        } else {
                            "登录系统后自动启动识矩，仅对当前设备生效。"
                        }),
                        launch_toggle,
                    ),
                    settings_divider(),
                    setting_row(
                        i18n.__("关闭到托盘"),
                        i18n.__(if cfg!(target_os = "macos") {
                            "点击红色关闭按钮后继续运行；从屏幕顶部菜单栏的识矩图标显示窗口或退出。"
                        } else {
                            "关闭主窗口后在后台运行，可从托盘显示窗口或退出。"
                        }),
                        tray_toggle,
                    ),
                    settings_divider(),
                    setting_row(
                        crate::app::i18n::I18n::new(app.desktop_locale).__("语言"),
                        crate::app::i18n::I18n::new(app.desktop_locale)
                            .__("当前桌面端使用的界面语言。"),
                        locale_pick,
                    ),
                    settings_divider(),
                    setting_row(
                        i18n.__("应用主题"),
                        i18n.__("快速切换内置、预设和自定义主题；完整管理见「主题」标签。"),
                        theme_pick,
                    ),
                    settings_divider(),
                    setting_row(
                        i18n.__("自动保存"),
                        i18n.__("控制预览编辑器自动保存的时机。"),
                        column![
                            auto_save_pick,
                            text(i18n.__(app.preview_auto_save_mode.description()))
                                .size(11)
                                .style(settings_muted_text_style),
                        ]
                        .spacing(8),
                    ),
                    settings_divider(),
                    setting_row(
                        i18n.__("任务完成通知"),
                        i18n.__(if crate::app::desktop_notification::SUPPORTED {
                            "本轮对话成功完成后发送系统通知，需在系统中允许通知。"
                        } else {
                            "当前仅支持 macOS 和 Windows 桌面端。"
                        }),
                        row![
                            checkbox(app.task_completion_notification)
                                .label(i18n.__("启用通知"))
                                .on_toggle_maybe(crate::app::desktop_notification::SUPPORTED.then_some(
                                    |enabled| Message::Settings(message::SettingsMessage::General(
                                        message::GeneralSettingsMessage::TaskCompletionNotificationToggled(enabled),
                                    )),
                                ))
                                .style(settings_checkbox_style),
                            button(text(i18n.__("发送测试通知")).size(13))
                                .on_press_maybe(crate::app::desktop_notification::SUPPORTED.then_some(
                                    Message::Settings(message::SettingsMessage::General(
                                        message::GeneralSettingsMessage::TestTaskCompletionNotification,
                                    )),
                                ))
                                .padding([8, 14])
                                .style(rounded_action_btn_style),
                        ]
                        .spacing(12)
                        .align_y(Alignment::Center),
                    ),
                    settings_divider(),
                    setting_row(
                        i18n.__("聊天时区"),
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
            settings_section_card(i18n.__("终端"), i18n.__("默认 Shell、主题与字体。")),
            settings_section_content(
                column![
                    setting_row(
                        i18n.__("终端 Shell"),
                        i18n.__("新终端默认使用的 Shell。"),
                        shell_pick
                    ),
                    settings_divider(),
                    setting_row(
                        i18n.__("终端主题"),
                        i18n.__("当前仅跟随系统主题。"),
                        term_theme_pick
                    ),
                    settings_divider(),
                    setting_row(i18n.__("终端字体"), i18n.__("选择终端字体家族。"), font_pick),
                    settings_divider(),
                    setting_row(
                        i18n.__("字体大小"),
                        i18n.__("在 10pt 到 18pt 之间调整终端字号。"),
                        row![
                            size_slider,
                            settings_value_badge(format!("{} pt", app.terminal.font_size as u32))
                        ]
                        .spacing(12)
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
        settings_page_intro(i18n.__("常规"), i18n.__("通过标签切换界面、终端、编辑器与主题配置。")),
        tabs,
        content,
    ]
    .spacing(16)
    .into()
}
#[cfg(test)]
#[path = "system_settings_general_tests.rs"]
mod system_settings_general_tests;
