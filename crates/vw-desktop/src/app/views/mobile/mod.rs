//! 移动外壳；全局通知、认证和审批浮层由 App::view 统一挂载。
use crate::app::assets::{Icon, get_icon};
use crate::app::message::ViewMessage;
#[cfg(not(target_arch = "wasm32"))]
use crate::app::state::LayoutMode;
use crate::app::{App, Message, Screen};
use iced::widget::{column, container, row, text};
use iced::{Element, Length};
pub mod chat;
pub mod dialogs;
mod mine;
pub mod navigation;
pub mod settings;

pub(super) fn action(label: &str, message: ViewMessage) -> Element<'_, Message> {
    iced::widget::button(text(label))
        .on_press(Message::View(message))
        .padding([10, 12])
        .height(44)
        .style(crate::app::components::system_settings_common::rounded_action_btn_style)
        .into()
}

pub fn view(app: &App) -> Element<'_, Message> {
    use crate::app::components::system_settings_skills;

    let i18n = desktop_i18n::I18n::new(app.general_settings.desktop_locale);
    let local_skills =
        cfg!(target_arch = "wasm32") && system_settings_skills::is_visible(app);
    let title = if app.mobile_layout.mine_open {
        "我的"
    } else if app.mobile_layout.drawer_open {
        "项目"
    } else if app.navigation_panels.show_system_settings {
        "设置"
    } else if local_skills {
        "本地技能"
    } else if matches!(app.navigation_tabs.screen, Screen::AgentManagement(_)) {
        "智能体"
    } else if app.navigation_tabs.screen == Screen::Cron {
        desktop_i18n::I18n::new(app.general_settings.desktop_locale).__("定时任务")
    } else if app.navigation_tabs.screen == Screen::Projects {
        desktop_i18n::I18n::new(app.general_settings.desktop_locale).__("项目管理")
    } else {
        "会话"
    };
    let header = header(app, i18n.__(title));
    let content = if app.mobile_layout.mine_open {
        mine::view(app)
    } else if app.mobile_layout.drawer_open {
        navigation::view(app)
    } else if app.navigation_panels.show_system_settings {
        settings::view(app)
    } else if local_skills {
        system_settings_skills::view(app)
    } else if let Screen::AgentManagement(page) = app.navigation_tabs.screen {
        crate::app::components::system_settings_agents::management_view(app, page)
    } else if app.navigation_tabs.screen == Screen::Cron {
        crate::app::components::system_settings_cron::page(app)
    } else if app.navigation_tabs.screen == Screen::Projects {
        crate::app::components::system_settings_projects::page(app)
    } else if matches!(app.navigation_tabs.screen, Screen::Home | Screen::Project) {
        chat::view(app)
    } else {
        column![
            text(i18n.__("此工具尚未适配手机布局。")),
            action(i18n.__("返回会话"), ViewMessage::MobileChat),
        ]
        .spacing(12)
        .into()
    };
    let base = column![
        header,
        container(
            container(content)
                .padding(iced::Padding { top: 0.0, right: 7.0, bottom: 12.0, left: 7.0 })
                .width(Length::Fill)
                .max_width(430)
                .height(Length::Fill)
        )
        .center_x(Length::Fill)
        .height(Length::Fill),
        bottom_navigation(app)
    ]
    .spacing(0)
    .width(Length::Fill)
    .height(Length::Fill)
    .into();
    dialogs::view(app, base)
}

/// 原生窗口控制区与页面导航分行，避免窄屏下挤占返回按钮。
fn header<'a>(app: &App, title: &'a str) -> Element<'a, Message> {
    let i18n = desktop_i18n::I18n::new(app.general_settings.desktop_locale);
    let controls = row![
        navigation_icon(Icon::ArrowLeft, i18n.__("返回"), ViewMessage::MobileBack),
        container(text(title)).width(Length::Fill).center_x(Length::Fill),
    ];
    #[cfg(not(target_arch = "wasm32"))]
    let controls =
        controls.push(phone_mode_button(LayoutMode::Mobile, app.general_settings.desktop_locale));
    let controls = controls
        .push(navigation_icon(
            Icon::FolderClosed,
            i18n.__("项目"),
            ViewMessage::MobileDrawerToggled,
        ))
        .spacing(8)
        .align_y(iced::Alignment::Center);
    let controls: Element<'_, Message> =
        container(container(controls).padding(7).width(Length::Fill).max_width(430))
            .center_x(Length::Fill)
            .into();
    // macOS 使用透明标题栏，原生红黄绿按钮仍占用顶部 28px。
    #[cfg(target_os = "macos")]
    {
        let titlebar = iced::widget::mouse_area(
            container(iced::widget::Space::new())
                .width(Length::Fill)
                .height(crate::app::components::top_bar::TOP_BAR_HEIGHT),
        )
        .on_press(Message::View(ViewMessage::WindowDragPressed));
        return column![titlebar, controls].width(Length::Fill).into();
    }
    #[cfg(not(target_os = "macos"))]
    controls
}

/// 通栏底部导航：图标在上、标签在下，背景延伸至页面底边。
fn bottom_navigation(app: &App) -> Element<'_, Message> {
    let i18n = desktop_i18n::I18n::new(app.general_settings.desktop_locale);
    let mine = app.mobile_layout.mine_open;
    let project = !mine && app.mobile_layout.drawer_open;
    let settings = !mine && !project && app.navigation_panels.show_system_settings;
    container(
        row![
            bottom_tab(
                get_icon(Icon::ChatTextFill),
                i18n.__("会话"),
                ViewMessage::MobileChat,
                !mine
                    && !project
                    && !settings
                    && !matches!(app.navigation_tabs.screen, Screen::Cron | Screen::Projects | Screen::AgentManagement(_))
            ),
            bottom_tab(
                get_icon(Icon::FolderClosed),
                i18n.__("项目"),
                ViewMessage::MobileDrawerToggled,
                project
            ),
            bottom_tab(
                get_icon(Icon::Gear),
                i18n.__("设置"),
                ViewMessage::MobileSettings,
                settings
            ),
            bottom_tab(mine::person_icon(), i18n.__("我的"), ViewMessage::MobileMine, mine),
        ]
        .width(Length::Fill)
        .align_y(iced::Alignment::Center),
    )
    .padding(iced::Padding { top: 8.0, right: 0.0, bottom: 12.0, left: 0.0 })
    .width(Length::Fill)
    .style(|theme: &iced::Theme| container::Style {
        background: Some(theme.extended_palette().background.weak.color.into()),
        ..Default::default()
    })
    .into()
}

fn bottom_tab(
    icon: iced::widget::svg::Handle,
    label: &'static str,
    message: ViewMessage,
    selected: bool,
) -> Element<'static, Message> {
    use iced::widget::{button, svg};
    let icon = svg(icon).width(24).height(24).style(move |theme: &iced::Theme, _| svg::Style {
        color: Some(if selected {
            theme.palette().text
        } else {
            theme.palette().text.scale_alpha(0.48)
        }),
    });
    let label = text(label).size(12).style(move |theme: &iced::Theme| iced::widget::text::Style {
        color: Some(if selected {
            theme.palette().text
        } else {
            theme.palette().text.scale_alpha(0.48)
        }),
    });
    let content =
        column![icon, label].spacing(5).align_x(iced::Alignment::Center).width(Length::Fill);
    let button =
        button(content).padding([6, 0]).width(Length::Fill).style(|theme: &iced::Theme, status| {
            iced::widget::button::Style {
                background: matches!(
                    status,
                    iced::widget::button::Status::Hovered | iced::widget::button::Status::Pressed
                )
                .then(|| theme.palette().text.scale_alpha(0.06).into()),
                text_color: theme.palette().text,
                border: iced::Border { radius: 8.0.into(), ..Default::default() },
                ..Default::default()
            }
        });
    // 当前项目页重复点击保持打开，避免底部标签像抽屉开关一样关闭页面。
    if selected { button.into() } else { button.on_press(Message::View(message)).into() }
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn phone_mode_button(
    mode: LayoutMode,
    locale: config_ui::DesktopLocale,
) -> Element<'static, Message> {
    use crate::app::state::LayoutPreference;
    use iced::widget::svg;
    let (next, label) = match mode {
        LayoutMode::Desktop => (LayoutPreference::Mobile, "进入手机模式"),
        LayoutMode::Mobile => (LayoutPreference::Desktop, "退出手机模式"),
    };
    let icon = svg::Handle::from_memory(
        br#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="white" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round"><rect x="6" y="2" width="12" height="20" rx="3"/><path d="M10 5h4m-3 14h2"/></svg>"#
            .as_slice(),
    );
    header_icon(icon, desktop_i18n::I18n::new(locale).__(label), ViewMessage::LayoutSelected(next))
}

fn navigation_icon(
    icon: Icon,
    label: &'static str,
    message: ViewMessage,
) -> Element<'static, Message> {
    header_icon(get_icon(icon), label, message)
}

/// 顶部图标共用尺寸、命中区域、交互反馈及下方提示。
fn header_icon(
    icon: iced::widget::svg::Handle,
    label: &'static str,
    message: ViewMessage,
) -> Element<'static, Message> {
    use iced::widget::{button, svg, tooltip};
    let icon = svg(icon)
        .width(14)
        .height(14)
        .style(|theme: &iced::Theme, _| svg::Style { color: Some(theme.palette().text) });
    let control = button(container(icon).center_x(24).center_y(24))
        .padding(0)
        .on_press(Message::View(message))
        .style(|theme, status| {
            desktop_navigation::components::top_bar::widgets::icon_toggle_button_style(
                theme, status, true, false,
            )
        });
    let tip = container(text(label).size(12))
        .padding([6, 10])
        .style(desktop_ui_controls::tooltip::tooltip_dark_style);
    tooltip(control, tip, tooltip::Position::Bottom).gap(6).into()
}

/// 页面铺满视口；限宽只应用于标题和内容，底栏始终通栏。
pub(crate) fn narrow_view(app: &App) -> Element<'_, Message> {
    view(app)
}

#[cfg(test)]
mod layout_tests;
