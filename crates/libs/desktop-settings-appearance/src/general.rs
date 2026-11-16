use crate::general_fields::{self, GeneralSettingField};
use desktop_ui_controls::settings_navigation::GeneralSettingsTab;
use desktop_ui_controls::system_settings_common::*;
use crate::headings::{HelpRenderer, settings_flexible_row_with_help, settings_group_heading_with_help, settings_page_heading_with_help};
use desktop_i18n::I18n;
use config_ui::PreviewAutoSaveMode;
use desktop_theme::{theme_store_from_home, to_iced_theme};
use iced::{Element, Theme, Length, Alignment};
use iced::widget::{button, column, row, slider, text};
use std::borrow::Cow;
use desktop_settings_security::SettingsTabDescriptor as TabDescriptor;
pub struct View<'a, S> {
 pub desktop_locale: config_ui::DesktopLocale, pub app_theme: &'a Theme,
 pub shell: S, pub shell_options: Vec<S>, pub font_family: &'a str, pub displayed_font_size: f32,
 pub preview_auto_save_mode: PreviewAutoSaveMode, pub chat_timezone: &'a str,
 pub launch_at_login: bool, pub launch_at_login_busy: bool, pub close_to_tray: bool,
 pub task_completion_notification: bool, pub notification_supported: bool,
 pub general_settings_tab: GeneralSettingsTab,
}
#[derive(Debug, Clone)]
pub enum Action<S> {
 AppThemeSelected(Theme), ShellSelected(S), TerminalSystemTheme, FontSelected(String),
 FontSizePreview(f32), FontSizePreviewCommit, AutoSaveModeChanged(PreviewAutoSaveMode),
 LocaleSelected(config_ui::DesktopLocale), TimezoneSelected(String),
 LaunchAtLoginToggled(bool), CloseToTrayToggled(bool), TaskCompletionNotificationToggled(bool),
 TestTaskCompletionNotification, TabSelected(GeneralSettingsTab),
}
pub fn app_theme_options(app_theme: &Theme) -> Vec<Theme> {
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

    if !themes.contains(app_theme) {
        themes.push(app_theme.clone());
    }
    themes
}

const GENERAL_CONTROL_WIDTH: f32 = 280.0;

pub const GENERAL_TABS: &[TabDescriptor<GeneralSettingsTab>] = &[
    TabDescriptor::new(GeneralSettingsTab::Interface, "界面", &[]),
    TabDescriptor::new(GeneralSettingsTab::Terminal, "终端", &[]),
    TabDescriptor::new(GeneralSettingsTab::Editor, "编辑器", &["editor", "code"]),
    TabDescriptor::new(
        GeneralSettingsTab::Themes,
        "主题",
        &["themes", "gallery", "palette", "custom", "builtin", "画廊", "配色", "色板"],
    ),
    TabDescriptor::new(
        GeneralSettingsTab::Companion,
        "迷你界面和智能伙伴",
        &["mini", "companion", "pet", "伙伴", "头像"],
    ),
];

fn general_field_with_help<'a, S, Message: Clone + 'static>(
    app: &View<'_, S>,
    help: HelpRenderer<Message>,
    field: &GeneralSettingField,
    description: impl Into<Cow<'a, str>>,
    control: impl Into<Element<'a, Message>>,
) -> iced::widget::Container<'a, Message> {
    let i18n = I18n::new(app.desktop_locale);
    settings_flexible_row_with_help(
        help,
        i18n,
        app.app_theme,
        i18n.__(field.label),
        description,
        GENERAL_CONTROL_WIDTH,
        control,
    )
    .id(field.id)
}

fn interface_field_row<'a, S, Message: Clone + 'static>(
    app: &View<'_, S>,
    help: HelpRenderer<Message>,
    field: &GeneralSettingField,
    description: impl Into<Cow<'a, str>>,
    control: impl Into<Element<'a, Message>>,
) -> Element<'a, Message> {
    general_field_with_help(app, help, field, description, control).padding([12, 0]).into()
}

/// Render the active general tab using explicit borrowed settings and a host subpage.
pub fn view<'a, S: Clone + Copy + PartialEq + std::fmt::Display + 'static, Message: Clone + 'static>(app: &View<'a, S>, event: fn(Action<S>) -> Message, help: HelpRenderer<Message>, subpage: Element<'a, Message>) -> Element<'a, Message> {
    let i18n = I18n::new(app.desktop_locale);
    let locales = ["简体中文", "繁體中文", "English"];
    let locale_index = match app.desktop_locale {
        config_ui::DesktopLocale::SimplifiedChinese => 0,
        config_ui::DesktopLocale::TraditionalChinese => 1,
        config_ui::DesktopLocale::English => 2,
    };
    let locale_pick = settings_select(locales, Some(locales[locale_index]), move |label| {
        let locale = match label {
            "繁體中文" => config_ui::DesktopLocale::TraditionalChinese,
            "English" => config_ui::DesktopLocale::English,
            _ => config_ui::DesktopLocale::SimplifiedChinese,
        };
        event(Action::
            LocaleSelected(locale),
        )
    })
    .width(GENERAL_CONTROL_WIDTH);

    let shell_pick =
        settings_select(app.shell_options.clone(), Some(app.shell), move |v| event(Action::ShellSelected(v)))
            .width(GENERAL_CONTROL_WIDTH);

    let term_theme_pick =
        settings_select([i18n.__("跟随系统")], Some(i18n.__("跟随系统")), move |_| {
            event(Action::TerminalSystemTheme)
        })
        .width(GENERAL_CONTROL_WIDTH);

    let font_pick = settings_select(
        [
            "JetBrains Mono".to_string(),
            "Fira Code".to_string(),
            "Menlo".to_string(),
            "Monaco".to_string(),
        ],
        Some(app.font_family.to_string()),
        move |v| event(Action::FontSelected(v)),
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
            event(Action::AutoSaveModeChanged(mode))
        },
    )
    .width(GENERAL_CONTROL_WIDTH);

    let timezone_pick = settings_select(
        config_ui::timezone_options().into_iter().map(str::to_string).collect::<Vec<_>>(),
        Some(app.chat_timezone.to_string()),
        move |v| event(Action::TimezoneSelected(v)),
    )
    .width(GENERAL_CONTROL_WIDTH);

    let launch_toggle = settings_toggle(app.launch_at_login).on_toggle_maybe(
        (!app.launch_at_login_busy && !cfg!(target_arch = "wasm32")).then_some(move |enabled| {
            event(Action::
                LaunchAtLoginToggled(enabled),
            )
        }),
    );
    let tray_toggle = settings_toggle(app.close_to_tray).on_toggle_maybe(
        (!cfg!(target_arch = "wasm32")).then_some(move |enabled| {
            event(Action::
                CloseToTrayToggled(enabled),
            )
        }),
    );

    let displayed_font_size =
        app.displayed_font_size;
    let size_slider = slider(10.0..=18.0, displayed_font_size, move |v| event(Action::FontSizePreview(v)))
        .on_release(event(Action::FontSizePreviewCommit))
        .width(Length::Fill);

    let active_tab = app.general_settings_tab;
    let tabs = settings_tab_row(GENERAL_TABS.iter().map(|descriptor| {
        settings_tab_button(
            i18n.__(descriptor.label),
            descriptor.tab == active_tab,
            Some(event(Action::
                TabSelected(descriptor.tab),
            )),
        )
        .into()
    }))
    .wrap();

    let content: Element<'_, Message> = match active_tab {
        GeneralSettingsTab::Interface => column![
            settings_group_heading_with_help(
        help,
                i18n,
                app.app_theme,
                i18n.__("界面"),
                i18n.__("应用外观与预览行为。"),
            ),
            settings_group_card(
                column![
                    interface_field_row(
                        app, help,
                        &general_fields::LAUNCH_ON_LOGIN,
                        i18n.__("随系统自动启动识矩，仅对当前设备生效。"),
                        launch_toggle,
                    ),
                    settings_divider(),
                    interface_field_row(
                        app, help,
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
                        app, help,
                        &general_fields::LANGUAGE,
                        i18n.__("当前桌面端使用的界面语言。"),
                        locale_pick,
                    ),
                    settings_divider(),
                    interface_field_row(
                        app, help,
                        &general_fields::APP_THEME,
                        i18n.__("快速切换内置、预设和自定义主题；完整管理见「主题」标签。"),
                        settings_select(
                            app_theme_options(app.app_theme),
                            Some(app.app_theme.clone()),
                            move |v| event(Action::AppThemeSelected(v)),
                        )
                        .width(GENERAL_CONTROL_WIDTH),
                    ),
                    settings_divider(),
                    interface_field_row(
                        app, help,
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
                        app, help,
                        &general_fields::TASK_NOTIFICATION,
                        i18n.__(if app.notification_supported {
                            "本轮对话成功完成后发送系统通知，需在系统中允许通知。"
                        } else {
                            "当前仅支持 macOS 和 Windows 桌面端。"
                        }),
                        row![
                            button(text(i18n.__("发送测试通知")).size(13))
                                .on_press_maybe(app.notification_supported.then_some(
                                    event(Action::
                                        TestTaskCompletionNotification,
                                    ),
                                ))
                                .padding([8, 14])
                                .style(rounded_action_btn_style),
                            settings_toggle(app.task_completion_notification)
                                .on_toggle_maybe(app.notification_supported.then_some(
                                    move |enabled| event(Action::
                                        TaskCompletionNotificationToggled(enabled),
                                    ),
                                )),
                        ]
                        .spacing(12)
                        .align_y(Alignment::Center)
                        .wrap(),
                    ),
                    settings_divider(),
                    interface_field_row(
                        app, help,
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
        help,
                i18n, app.app_theme, i18n.__("终端"), i18n.__("默认 Shell、主题与字体。")
            ),
            settings_group_card(
                column![
                    general_field_with_help(
                        app, help,
                        &general_fields::TERMINAL_SHELL,
                        i18n.__("新终端默认使用的 Shell。"),
                        shell_pick
                    ),
                    settings_divider(),
                    general_field_with_help(
                        app, help,
                        &general_fields::TERMINAL_THEME,
                        i18n.__("当前仅跟随系统主题。"),
                        term_theme_pick
                    ),
                    settings_divider(),
                    general_field_with_help(
                        app, help,
                        &general_fields::TERMINAL_FONT,
                        i18n.__("选择终端字体家族。"),
                        font_pick,
                    ),
                    settings_divider(),
                    general_field_with_help(
                        app, help,
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
        GeneralSettingsTab::Editor => subpage,
        GeneralSettingsTab::Companion => subpage,
        GeneralSettingsTab::Themes => subpage,
    };

    column![
        settings_page_heading_with_help(
        help,
            i18n,
            app.app_theme,
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
