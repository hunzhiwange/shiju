//! 处理工作资源区停靠浏览器的导航、绘制与错误反馈。

use iced::widget::svg;
use iced::widget::tooltip::{Position as TooltipPosition, Tooltip};
use iced::widget::{Space, button, column, container, row, text, text_input};
use iced::{Background, Border, Element, Length, Theme};


use desktop_assets::assets::{self, Icon};
use desktop_ui_controls::tooltip::tooltip_dark_style;
use crate::state::WorkBrowserState;

#[cfg(not(target_arch = "wasm32"))]
use crate::webview_runtime::{
    WorkBrowserLayout, WorkBrowserSurface, normalize_navigation_url, work_browser_bounds,
    work_browser_surface_visible,
};

const WORK_RESIZE_HANDLE_WIDTH: f32 = 6.0;
pub const RESOURCE_PANEL_HEADER_HEIGHT: f32 = 88.0;
pub const BROWSER_TOOLBAR_MAX_HEIGHT: f32 = 38.0;
pub const BROWSER_TOOLBAR_CONTENT_GAP: f32 = 8.0;
pub const BROWSER_CONTENT_TOP_INSET: f32 =
    RESOURCE_PANEL_HEADER_HEIGHT + BROWSER_TOOLBAR_MAX_HEIGHT + BROWSER_TOOLBAR_CONTENT_GAP;
const BROWSER_HORIZONTAL_INSET: f32 = 12.0;
pub const BROWSER_BOTTOM_GAP: f32 = 10.0;

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
pub fn external_browser_message(address: &str) -> Option<Message> {
    normalize_navigation_url(address)
        .ok()
        .map(|url| Message::OpenUrlExternal(url))
}

#[cfg(target_arch = "wasm32")]
pub fn external_browser_message(_address: &str) -> Option<Message> {
    None
}

pub fn view(state: &WorkBrowserState, locale: config_ui::DesktopLocale) -> Element<'_, Message> {
    let browser_active = !state.closed;
    let toolbar = container(
        row![
            toolbar_icon_button(
                Icon::ArrowLeft,
                desktop_i18n::I18n::new(locale).__("后退"),
                browser_active.then_some(Message::WorkBrowserBack)
            ),
            toolbar_icon_button(
                Icon::ArrowRight,
                desktop_i18n::I18n::new(locale).__("前进"),
                browser_active.then_some(Message::WorkBrowserForward)
            ),
            text_input(
                desktop_i18n::I18n::new(locale).__("输入网址"),
                &state.address
            )
            .on_input(|address| Message::WorkBrowserAddressChanged(address))
            .on_submit(Message::WorkBrowserNavigate)
            .padding([5, 10])
            .size(12)
            .style(address_style),
            toolbar_icon_button(
                Icon::ArrowClockwise,
                desktop_i18n::I18n::new(locale).__("刷新"),
                browser_active.then_some(Message::WorkBrowserReload)
            ),
            toolbar_icon_button(
                Icon::Globe,
                desktop_i18n::I18n::new(locale).__("在外部浏览器中打开"),
                external_browser_message(&state.address)
            ),
            toolbar_icon_button(
                Icon::X,
                desktop_i18n::I18n::new(locale).__("关闭当前浏览"),
                browser_active.then_some(Message::WorkBrowserClose)
            ),
        ]
        .spacing(5)
        .align_y(iced::Alignment::Center),
    )
    .width(Length::Fill)
    .padding([4, 6])
    .style(toolbar_style);

    let surface: Element<'_, Message> = if let Some(error) = state.error.as_deref() {
        container(
            column![
                text(desktop_i18n::I18n::new(locale).__("内置浏览器不可用"))
                    .size(16),
                text(error).size(12).style(|theme: &Theme| text::Style {
                    color: Some(theme.extended_palette().danger.base.color),
                }),
                button(text(desktop_i18n::I18n::new(locale).__("重试")).size(12))
                    .on_press(Message::WorkBrowserNavigate)
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

#[derive(Debug, Clone)]
pub enum Message {
    WorkBrowserAddressChanged(String), WorkBrowserNavigate, WorkBrowserBack,
    WorkBrowserForward, WorkBrowserReload, WorkBrowserClose, WorkBrowserRuntimeTick,
    OpenUrlExternal(String),
}
/// Host geometry and cross-domain visibility projected into browser placement.
pub struct Placement {
    pub project_screen: bool, pub work_mode: bool, pub browser_resource: bool,
    pub resources_visible: bool, pub system_settings_open: bool,
    pub chat_fullscreen: bool, pub chat_half_fullscreen: bool,
    pub window_position: (f32, f32), pub window_size: (f32, f32),
    pub sidebar_collapsed: bool, pub sidebar_width: f32, pub sidebar_rail_width: f32,
    pub terminal_visible: bool, pub terminal_fullscreen: bool, pub terminal_height: f32,
    pub split_ratio: f32, pub resource_fullscreen: bool, pub resource_half_fullscreen: bool,
    pub top_bar_height: f32, pub status_bar_height: f32,
}
#[cfg(not(target_arch = "wasm32"))]
impl Placement {
    fn surface(&self) -> WorkBrowserSurface { WorkBrowserSurface {
        project_screen: self.project_screen, work_mode: self.work_mode,
        browser_resource: self.browser_resource,
        resources_visible: self.resources_visible && !self.terminal_fullscreen,
        system_settings_open: self.system_settings_open,
        chat_fullscreen: self.chat_fullscreen, chat_half_fullscreen: self.chat_half_fullscreen,
    } }
    fn layout(&self) -> WorkBrowserLayout {
        let rail_width = if self.sidebar_collapsed { self.sidebar_rail_width } else { 0.0 };
        let dock_height = if self.terminal_visible && !self.resource_fullscreen { self.terminal_height.max(0.0) } else { 0.0 };
        WorkBrowserLayout {
            window_x: self.window_position.0, window_y: self.window_position.1,
            window_width: self.window_size.0, window_height: self.window_size.1,
            work_sidebar_width: self.sidebar_width + rail_width + WORK_RESIZE_HANDLE_WIDTH,
            work_divider_width: WORK_RESIZE_HANDLE_WIDTH, top_bar_height: self.top_bar_height,
            resource_header_height: BROWSER_CONTENT_TOP_INSET,
            horizontal_inset: BROWSER_HORIZONTAL_INSET,
            bottom_inset: self.status_bar_height + BROWSER_BOTTOM_GAP + dock_height,
            split_ratio: self.split_ratio, resource_fullscreen: self.resource_fullscreen,
            resource_half_fullscreen: self.resource_half_fullscreen,
        }
    }
}
pub fn update(state: &mut WorkBrowserState, #[cfg(not(target_arch = "wasm32"))] controller: &mut crate::webview_runtime::WorkBrowserController, placement: &Placement, message: Message) -> Option<String> {
    match message {
        Message::WorkBrowserAddressChanged(address) => {
            state.address = address;
            None
        }
        Message::WorkBrowserNavigate => navigate(state, #[cfg(not(target_arch = "wasm32"))] controller, placement),
        Message::WorkBrowserBack => control(state, #[cfg(not(target_arch = "wasm32"))] controller, placement, BrowserAction::Back),
        Message::WorkBrowserForward => control(state, #[cfg(not(target_arch = "wasm32"))] controller, placement, BrowserAction::Forward),
        Message::WorkBrowserReload => control(state, #[cfg(not(target_arch = "wasm32"))] controller, placement, BrowserAction::Reload),
        Message::WorkBrowserClose => close_current(state, #[cfg(not(target_arch = "wasm32"))] controller, placement),
        Message::WorkBrowserRuntimeTick => poll_runtime_events(state, #[cfg(not(target_arch = "wasm32"))] controller, placement),
        _ => None,
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn poll_runtime_events(state: &mut WorkBrowserState, #[cfg(not(target_arch = "wasm32"))] controller: &mut crate::webview_runtime::WorkBrowserController, _placement: &Placement) -> Option<String> {
    if let Some(address) = controller.take_latest_navigation_url() {
        state.address = address;
    }
    None
}

#[cfg(target_arch = "wasm32")]
fn poll_runtime_events(_state: &mut WorkBrowserState, #[cfg(not(target_arch = "wasm32"))] controller: &mut crate::webview_runtime::WorkBrowserController, _placement: &Placement) -> Option<String> {
    None
}

#[cfg(not(target_arch = "wasm32"))]
pub fn sync_lifecycle(state: &mut WorkBrowserState, #[cfg(not(target_arch = "wasm32"))] controller: &mut crate::webview_runtime::WorkBrowserController, placement: &Placement) -> Option<String> {
    let result = sync_lifecycle_result(state, #[cfg(not(target_arch = "wasm32"))] controller, placement).map(|_| ());
    finish_runtime_result(state, result)
}

#[cfg(target_arch = "wasm32")]
pub fn sync_lifecycle(_state: &mut WorkBrowserState, #[cfg(not(target_arch = "wasm32"))] controller: &mut crate::webview_runtime::WorkBrowserController, _placement: &Placement) -> Option<String> {
    None
}

#[derive(Debug, Clone, Copy)]
enum BrowserAction {
    Back,
    Forward,
    Reload,
}

#[cfg(not(target_arch = "wasm32"))]
fn navigate(state: &mut WorkBrowserState, #[cfg(not(target_arch = "wasm32"))] controller: &mut crate::webview_runtime::WorkBrowserController, placement: &Placement) -> Option<String> {
    let target = match normalize_navigation_url(&state.address) {
        Ok(target) => target,
        Err(error) => return finish_runtime_result(state, Err(error)),
    };
    state.address.clone_from(&target);
    state.closed = false;
    let result = sync_lifecycle_result(state, #[cfg(not(target_arch = "wasm32"))] controller, placement).and_then(|started| {
        if started { Ok(()) } else { controller.navigate(&target) }
    });
    finish_runtime_result(state, result)
}

#[cfg(target_arch = "wasm32")]
fn navigate(state: &mut WorkBrowserState, #[cfg(not(target_arch = "wasm32"))] controller: &mut crate::webview_runtime::WorkBrowserController, _placement: &Placement) -> Option<String> {
    finish_runtime_result(state, Err("当前平台不支持停靠浏览器".to_string()))
}

#[cfg(not(target_arch = "wasm32"))]
fn control(state: &mut WorkBrowserState, #[cfg(not(target_arch = "wasm32"))] controller: &mut crate::webview_runtime::WorkBrowserController, placement: &Placement, action: BrowserAction) -> Option<String> {
    let result = sync_lifecycle_result(state, #[cfg(not(target_arch = "wasm32"))] controller, placement).and_then(|_| match action {
        BrowserAction::Back => controller.back(),
        BrowserAction::Forward => controller.forward(),
        BrowserAction::Reload => controller.reload(),
    });
    finish_runtime_result(state, result)
}

#[cfg(target_arch = "wasm32")]
fn control(state: &mut WorkBrowserState, #[cfg(not(target_arch = "wasm32"))] controller: &mut crate::webview_runtime::WorkBrowserController, _placement: &Placement, _action: BrowserAction) -> Option<String> {
    finish_runtime_result(state, Err("当前平台不支持停靠浏览器".to_string()))
}

#[cfg(not(target_arch = "wasm32"))]
fn close_current(state: &mut WorkBrowserState, #[cfg(not(target_arch = "wasm32"))] controller: &mut crate::webview_runtime::WorkBrowserController, _placement: &Placement) -> Option<String> {
    let result = controller.close();
    state.address.clear();
    state.error = None;
    state.closed = true;

    match result {
        Ok(()) => None,
        Err(error) => Some(error),
    }
}

#[cfg(target_arch = "wasm32")]
fn close_current(state: &mut WorkBrowserState, #[cfg(not(target_arch = "wasm32"))] controller: &mut crate::webview_runtime::WorkBrowserController, _placement: &Placement) -> Option<String> {
    state.address.clear();
    state.error = None;
    state.closed = true;
    None
}

#[cfg(not(target_arch = "wasm32"))]
pub fn sync_lifecycle_result(state: &mut WorkBrowserState, #[cfg(not(target_arch = "wasm32"))] controller: &mut crate::webview_runtime::WorkBrowserController, placement: &Placement) -> Result<bool, String> {
    let visible = work_browser_surface_visible(placement.surface());
    if !visible {
        if controller.has_running_child() {
            controller.set_visible(false)?;
        }
        return Ok(false);
    }
    if state.closed {
        return Ok(false);
    }

    let Some(bounds) = work_browser_bounds(placement.layout()) else {
        if controller.has_running_child() {
            controller.set_visible(false)?;
        }
        return Err("当前窗口尺寸不足以停靠内置浏览器".into());
    };
    let target = normalize_navigation_url(&state.address)?;
    let started = controller.ensure_started(&target, bounds)?;
    controller.sync_bounds(bounds)?;
    controller.set_visible(true)?;
    controller.set_always_on_top(state.window_focused)?;
    Ok(started)
}

fn finish_runtime_result(state: &mut WorkBrowserState, result: Result<(), String>) -> Option<String> {
    match result {
        Ok(()) => {
            state.error = None;
            None
        }
        Err(error) => {
            let should_toast = state.error.as_deref() != Some(error.as_str());
            state.error = Some(error.clone());
            if should_toast { Some(error) } else { None }
        }
    }
}
