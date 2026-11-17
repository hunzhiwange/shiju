//! 移动外壳；全局通知、认证和审批浮层由 App::view 统一挂载。
use crate::app::assets::{Icon, get_icon};
use crate::app::message::ViewMessage;
use crate::app::{App, Message, Screen, state::LayoutPreference};
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
    let title = if app.mobile_layout.mine_open {
        "我的"
    } else if app.mobile_layout.drawer_open {
        "项目"
    } else if app.navigation_panels.show_system_settings {
        "设置"
    } else {
        "会话"
    };
    let header = header(app, title);
    let content = if app.mobile_layout.mine_open {
        mine::view(app)
    } else if app.mobile_layout.drawer_open {
        navigation::view(app)
    } else if app.navigation_panels.show_system_settings {
        settings::view(app)
    } else if matches!(app.navigation_tabs.screen, Screen::Home | Screen::Project) {
        chat::view(app)
    } else {
        column![
            text("此工具尚未适配手机，请切换桌面布局使用。"),
            action("返回会话", ViewMessage::MobileChat),
            action("桌面布局", ViewMessage::LayoutSelected(LayoutPreference::Desktop))
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
fn header<'a>(app: &'a App, title: &'a str) -> Element<'a, Message> {
    let controls = row![
        navigation_icon(Icon::ArrowLeft, "返回", ViewMessage::MobileBack),
        container(text(title)).width(Length::Fill).center_x(Length::Fill),
        layout_switch(app),
        navigation_icon(Icon::FolderClosed, "项目", ViewMessage::MobileDrawerToggled)
    ]
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
    let mine = app.mobile_layout.mine_open;
    let project = !mine && app.mobile_layout.drawer_open;
    let settings = !mine && !project && app.navigation_panels.show_system_settings;
    container(
        row![
            bottom_tab(
                get_icon(Icon::ChatTextFill),
                "会话",
                ViewMessage::MobileChat,
                !mine && !project && !settings
            ),
            bottom_tab(
                get_icon(Icon::FolderClosed),
                "项目",
                ViewMessage::MobileDrawerToggled,
                project
            ),
            bottom_tab(get_icon(Icon::Gear), "设置", ViewMessage::MobileSettings, settings),
            bottom_tab(mine::person_icon(), "我的", ViewMessage::MobileMine, mine),
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

/// 桌面工具栏与手机标题栏共用同一个紧凑三态按钮。
pub(crate) fn layout_switch(app: &App) -> Element<'_, Message> {
    use iced::widget::svg;
    let (next, label, path) = match app.mobile_layout.preference() {
        LayoutPreference::Auto => (
            LayoutPreference::Desktop,
            "自动 · 点击切换桌面",
            r#"<rect x="3" y="4" width="15" height="12" rx="2"/><rect x="15" y="10" width="6" height="11" rx="1"/><path d="M7 20h5m-2-4v4"/>"#,
        ),
        LayoutPreference::Desktop => (
            LayoutPreference::Mobile,
            "桌面 · 点击切换手机",
            r#"<rect x="2" y="3" width="20" height="14" rx="2"/><path d="M8 21h8m-4-4v4"/>"#,
        ),
        LayoutPreference::Mobile => (
            LayoutPreference::Auto,
            "手机 · 点击切换自动",
            r#"<rect x="6" y="2" width="12" height="20" rx="3"/><path d="M10 5h4m-3 14h2"/>"#,
        ),
    };
    let icon = svg::Handle::from_memory(format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="white" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round">{path}</svg>"#
    ).into_bytes());
    header_icon(icon, label, ViewMessage::LayoutSelected(next))
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
