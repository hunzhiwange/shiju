//! 处理工作资源区停靠浏览器的导航、绘制与错误反馈。

use iced::widget::svg;
use iced::widget::tooltip::{Position as TooltipPosition, Tooltip};
use iced::widget::{Space, button, column, container, row, text, text_input};
use iced::{Background, Border, Element, Length, Task, Theme};

use super::ViewMessage;
use crate::app::assets::{self, Icon};
use crate::app::components::input_panel::styles::tooltip_dark_style;
use crate::app::{App, Message};

#[cfg(not(target_arch = "wasm32"))]
use crate::app::webview_runtime::{
    WorkBrowserLayout, WorkBrowserSurface, normalize_navigation_url, work_browser_bounds,
    work_browser_surface_visible,
};

const WORK_RESIZE_HANDLE_WIDTH: f32 = 6.0;
const RESOURCE_PANEL_HEADER_HEIGHT: f32 = 88.0;
const BROWSER_TOOLBAR_MAX_HEIGHT: f32 = 38.0;
const BROWSER_TOOLBAR_CONTENT_GAP: f32 = 8.0;
const BROWSER_CONTENT_TOP_INSET: f32 =
    RESOURCE_PANEL_HEADER_HEIGHT + BROWSER_TOOLBAR_MAX_HEIGHT + BROWSER_TOOLBAR_CONTENT_GAP;
const BROWSER_HORIZONTAL_INSET: f32 = 12.0;
const BROWSER_BOTTOM_INSET: f32 = crate::app::impls::STATUS_BAR_HEIGHT + 10.0;

fn toolbar_style(theme: &Theme) -> container::Style {
    let palette = theme.extended_palette();
    container::Style {
        background: Some(Background::Color(palette.background.weak.color.scale_alpha(0.46))),
        border: Border {
            width: 1.0,
            color: palette.background.strong.color.scale_alpha(0.56),
            radius: 8.0.into(),
        },
        ..Default::default()
    }
}

fn control_style(theme: &Theme, status: button::Status) -> button::Style {
    let palette = theme.extended_palette();
    let background = matches!(status, button::Status::Hovered | button::Status::Pressed)
        .then_some(palette.background.strong.color.scale_alpha(0.62));
    button::Style {
        background: background.map(Background::Color),
        text_color: palette.background.base.text,
        border: Border { radius: 6.0.into(), ..Default::default() },
        ..Default::default()
    }
}

fn address_style(theme: &Theme, status: text_input::Status) -> text_input::Style {
    let palette = theme.extended_palette();
    let focused = matches!(status, text_input::Status::Focused { .. });
    text_input::Style {
        background: Background::Color(palette.background.weak.color.scale_alpha(0.72)),
        border: Border {
            width: 1.0,
            color: if focused {
                theme.palette().primary.scale_alpha(0.82)
            } else {
                palette.background.strong.color.scale_alpha(0.72)
            },
            radius: 12.0.into(),
        },
        icon: theme.palette().text.scale_alpha(0.68),
        placeholder: theme.palette().text.scale_alpha(0.42),
        value: theme.palette().text,
        selection: theme.palette().primary.scale_alpha(0.28),
    }
}

fn browser_icon(icon: Icon) -> svg::Svg<'static> {
    svg::Svg::new(assets::get_icon(icon))
        .width(Length::Fixed(16.0))
        .height(Length::Fixed(16.0))
        .style(|theme: &Theme, _status| svg::Style {
            color: Some(theme.palette().text.scale_alpha(0.88)),
        })
}

fn toolbar_icon_button(
    icon: Icon,
    label: &'static str,
    message: Option<Message>,
) -> Element<'static, Message> {
    let mut control = button(browser_icon(icon))
        .width(Length::Fixed(28.0))
        .height(Length::Fixed(28.0))
        .padding(6)
        .style(control_style);
    if let Some(message) = message {
        control = control.on_press(message);
    }

    Tooltip::new(
        control,
        container(text(label).size(12)).padding([6, 8]).style(tooltip_dark_style),
        TooltipPosition::Top,
    )
    .gap(6)
    .into()
}

#[cfg(not(target_arch = "wasm32"))]
fn external_browser_message(address: &str) -> Option<Message> {
    normalize_navigation_url(address)
        .ok()
        .map(|url| Message::View(ViewMessage::OpenUrlExternal(url)))
}

#[cfg(target_arch = "wasm32")]
fn external_browser_message(_address: &str) -> Option<Message> {
    None
}

pub(crate) fn view(app: &App) -> Element<'_, Message> {
    let browser_active = !app.work_browser.closed;
    let toolbar = container(
        row![
            toolbar_icon_button(
                Icon::ArrowLeft,
                "后退",
                browser_active.then_some(Message::View(ViewMessage::WorkBrowserBack))
            ),
            toolbar_icon_button(
                Icon::ArrowRight,
                "前进",
                browser_active.then_some(Message::View(ViewMessage::WorkBrowserForward))
            ),
            text_input("输入网址", &app.work_browser.address)
                .on_input(|address| Message::View(ViewMessage::WorkBrowserAddressChanged(address)))
                .on_submit(Message::View(ViewMessage::WorkBrowserNavigate))
                .padding([5, 10])
                .size(12)
                .style(address_style),
            toolbar_icon_button(
                Icon::ArrowClockwise,
                "刷新",
                browser_active.then_some(Message::View(ViewMessage::WorkBrowserReload))
            ),
            toolbar_icon_button(
                Icon::Globe,
                "在外部浏览器中打开",
                external_browser_message(&app.work_browser.address)
            ),
            toolbar_icon_button(
                Icon::X,
                "关闭当前浏览",
                browser_active.then_some(Message::View(ViewMessage::WorkBrowserClose))
            ),
        ]
        .spacing(5)
        .align_y(iced::Alignment::Center),
    )
    .width(Length::Fill)
    .padding([4, 6])
    .style(toolbar_style);

    let surface: Element<'_, Message> = if let Some(error) = app.work_browser.error.as_deref() {
        container(
            column![
                text("内置浏览器不可用").size(16),
                text(error).size(12).style(|theme: &Theme| text::Style {
                    color: Some(theme.extended_palette().danger.base.color),
                }),
                button(text("重试").size(12))
                    .on_press(Message::View(ViewMessage::WorkBrowserNavigate))
                    .padding([6, 10])
                    .style(control_style),
            ]
            .spacing(8)
            .align_x(iced::Alignment::Center),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .center(Length::Fill)
        .into()
    } else {
        Space::new().width(Length::Fill).height(Length::Fill).into()
    };

    column![toolbar, surface].spacing(8).width(Length::Fill).height(Length::Fill).into()
}

pub(super) fn update(app: &mut App, message: ViewMessage) -> Task<Message> {
    match message {
        ViewMessage::WorkBrowserAddressChanged(address) => {
            app.work_browser.address = address;
            Task::none()
        }
        ViewMessage::WorkBrowserNavigate => navigate(app),
        ViewMessage::WorkBrowserBack => control(app, BrowserAction::Back),
        ViewMessage::WorkBrowserForward => control(app, BrowserAction::Forward),
        ViewMessage::WorkBrowserReload => control(app, BrowserAction::Reload),
        ViewMessage::WorkBrowserClose => close_current(app),
        ViewMessage::WorkBrowserRuntimeTick => poll_runtime_events(app),
        _ => Task::none(),
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn poll_runtime_events(app: &mut App) -> Task<Message> {
    if let Some(address) = app.work_browser_controller.take_latest_navigation_url() {
        app.work_browser.address = address;
    }
    Task::none()
}

#[cfg(target_arch = "wasm32")]
fn poll_runtime_events(_app: &mut App) -> Task<Message> {
    Task::none()
}

#[cfg(not(target_arch = "wasm32"))]
pub(super) fn sync_lifecycle(app: &mut App) -> Task<Message> {
    let result = sync_lifecycle_result(app).map(|_| ());
    finish_runtime_result(app, result)
}

#[cfg(target_arch = "wasm32")]
pub(super) fn sync_lifecycle(_app: &mut App) -> Task<Message> {
    Task::none()
}

#[derive(Debug, Clone, Copy)]
enum BrowserAction {
    Back,
    Forward,
    Reload,
}

#[cfg(not(target_arch = "wasm32"))]
fn navigate(app: &mut App) -> Task<Message> {
    let target = match normalize_navigation_url(&app.work_browser.address) {
        Ok(target) => target,
        Err(error) => return finish_runtime_result(app, Err(error)),
    };
    app.work_browser.address.clone_from(&target);
    app.work_browser.closed = false;
    let result = sync_lifecycle_result(app).and_then(|started| {
        if started { Ok(()) } else { app.work_browser_controller.navigate(&target) }
    });
    finish_runtime_result(app, result)
}

#[cfg(target_arch = "wasm32")]
fn navigate(app: &mut App) -> Task<Message> {
    finish_runtime_result(app, Err("当前平台不支持停靠浏览器".to_string()))
}

#[cfg(not(target_arch = "wasm32"))]
fn control(app: &mut App, action: BrowserAction) -> Task<Message> {
    let result = sync_lifecycle_result(app).and_then(|_| match action {
        BrowserAction::Back => app.work_browser_controller.back(),
        BrowserAction::Forward => app.work_browser_controller.forward(),
        BrowserAction::Reload => app.work_browser_controller.reload(),
    });
    finish_runtime_result(app, result)
}

#[cfg(target_arch = "wasm32")]
fn control(app: &mut App, _action: BrowserAction) -> Task<Message> {
    finish_runtime_result(app, Err("当前平台不支持停靠浏览器".to_string()))
}

#[cfg(not(target_arch = "wasm32"))]
fn close_current(app: &mut App) -> Task<Message> {
    let result = app.work_browser_controller.close();
    app.work_browser.address.clear();
    app.work_browser.error = None;
    app.work_browser.closed = true;

    match result {
        Ok(()) => Task::none(),
        Err(error) => app.show_error_toast(error),
    }
}

#[cfg(target_arch = "wasm32")]
fn close_current(app: &mut App) -> Task<Message> {
    app.work_browser.address.clear();
    app.work_browser.error = None;
    app.work_browser.closed = true;
    Task::none()
}

#[cfg(not(target_arch = "wasm32"))]
fn sync_lifecycle_result(app: &mut App) -> Result<bool, String> {
    let visible = work_browser_surface_visible(WorkBrowserSurface {
        screen: app.screen,
        workspace_mode: app.workspace_mode,
        resource_mode: app.work_resource_mode,
        resources_visible: app.work_resources_visible,
        system_settings_open: app.show_system_settings,
        chat_fullscreen: app.work_chat_fullscreen,
        chat_half_fullscreen: app.work_chat_half_fullscreen,
    });
    if !visible {
        if app.work_browser_controller.has_running_child() {
            app.work_browser_controller.set_visible(false)?;
        }
        return Ok(false);
    }
    if app.work_browser.closed {
        return Ok(false);
    }

    let work_sidebar_width = app.work_sidebar_width
        + if app.work_sidebar_collapsed { 0.0 } else { WORK_RESIZE_HANDLE_WIDTH };
    let bounds = work_browser_bounds(WorkBrowserLayout {
        window_x: app.window_position.0,
        window_y: app.window_position.1,
        window_width: app.window_size.0,
        window_height: app.window_size.1,
        work_sidebar_width,
        work_divider_width: WORK_RESIZE_HANDLE_WIDTH,
        top_bar_height: crate::app::components::top_bar::TOP_BAR_HEIGHT,
        resource_header_height: BROWSER_CONTENT_TOP_INSET,
        horizontal_inset: BROWSER_HORIZONTAL_INSET,
        bottom_inset: BROWSER_BOTTOM_INSET,
        split_ratio: app.work_split_ratio,
        resource_fullscreen: app.work_resource_fullscreen,
        resource_half_fullscreen: app.work_resource_half_fullscreen,
    })
    .ok_or_else(|| "当前窗口尺寸不足以停靠内置浏览器".to_string())?;
    let target = normalize_navigation_url(&app.work_browser.address)?;
    let started = app.work_browser_controller.ensure_started(&target, bounds)?;
    app.work_browser_controller.sync_bounds(bounds)?;
    app.work_browser_controller.set_visible(true)?;
    app.work_browser_controller.set_always_on_top(app.work_browser.window_focused)?;
    Ok(started)
}

fn finish_runtime_result(app: &mut App, result: Result<(), String>) -> Task<Message> {
    match result {
        Ok(()) => {
            app.work_browser.error = None;
            Task::none()
        }
        Err(error) => {
            let should_toast = app.work_browser.error.as_deref() != Some(error.as_str());
            app.work_browser.error = Some(error.clone());
            if should_toast { app.show_error_toast(error) } else { Task::none() }
        }
    }
}

#[cfg(test)]
#[path = "work_browser_tests.rs"]
mod work_browser_controller_ui_tests;
