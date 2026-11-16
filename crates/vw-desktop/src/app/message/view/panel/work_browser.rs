use super::ViewMessage;
use crate::app::{App, Message};
use iced::{Task, Element};
use desktop_web_runtime::work_browser as domain;
fn map(message: domain::Message) -> Message { Message::View(match message {
    domain::Message::WorkBrowserAddressChanged(address) => ViewMessage::WorkBrowserAddressChanged(address),
    domain::Message::WorkBrowserNavigate => ViewMessage::WorkBrowserNavigate,
    domain::Message::WorkBrowserBack => ViewMessage::WorkBrowserBack,
    domain::Message::WorkBrowserForward => ViewMessage::WorkBrowserForward,
    domain::Message::WorkBrowserReload => ViewMessage::WorkBrowserReload,
    domain::Message::WorkBrowserClose => ViewMessage::WorkBrowserClose,
    domain::Message::WorkBrowserRuntimeTick => ViewMessage::WorkBrowserRuntimeTick,
    domain::Message::OpenUrlExternal(url) => ViewMessage::OpenUrlExternal(url),
}) }
fn placement(app: &App) -> domain::Placement {
    domain::Placement {
        project_screen: app.navigation_tabs.screen == crate::app::Screen::Project,
        work_mode: app.navigation_mode.workspace_mode == crate::app::state::WorkspaceMode::Work,
        browser_resource: app.navigation_workspace.work_resource_mode == crate::app::state::WorkResourceMode::Browser,
        resources_visible: app.navigation_workspace.work_resources_visible, system_settings_open: app.navigation_panels.show_system_settings,
        chat_fullscreen: app.navigation_workspace.work_chat_fullscreen, chat_half_fullscreen: app.navigation_workspace.work_chat_half_fullscreen,
        window_position: app.window_position, window_size: app.window_size,
        sidebar_collapsed: app.navigation_sidebar.work_sidebar_collapsed, sidebar_width: app.layout_drag.work_sidebar_width,
        sidebar_rail_width: crate::app::state::WORK_SIDEBAR_RAIL_WIDTH,
        terminal_visible: app.terminal.is_visible, terminal_fullscreen: app.terminal.is_fullscreen,
        terminal_height: app.terminal.height, split_ratio: app.navigation_workspace.work_split_ratio,
        resource_fullscreen: app.navigation_workspace.work_resource_fullscreen, resource_half_fullscreen: app.navigation_workspace.work_resource_half_fullscreen,
        top_bar_height: crate::app::components::top_bar::TOP_BAR_HEIGHT,
        status_bar_height: crate::app::impls::STATUS_BAR_HEIGHT,
    }
}
pub(crate) fn view(app: &App) -> Element<'_, Message> { domain::view(&app.work_browser, app.desktop_locale).map(map) }
fn finish(app: &mut App, error: Option<String>) -> Task<Message> {
    error.map(|error| app.show_error_toast(error)).unwrap_or_else(Task::none)
}
pub(super) fn update(app: &mut App, message: ViewMessage) -> Task<Message> {
    let message = match message {
        ViewMessage::WorkBrowserAddressChanged(address) => domain::Message::WorkBrowserAddressChanged(address),
        ViewMessage::WorkBrowserNavigate => domain::Message::WorkBrowserNavigate,
        ViewMessage::WorkBrowserBack => domain::Message::WorkBrowserBack,
        ViewMessage::WorkBrowserForward => domain::Message::WorkBrowserForward,
        ViewMessage::WorkBrowserReload => domain::Message::WorkBrowserReload,
        ViewMessage::WorkBrowserClose => domain::Message::WorkBrowserClose,
        ViewMessage::WorkBrowserRuntimeTick => domain::Message::WorkBrowserRuntimeTick,
        ViewMessage::OpenUrlExternal(url) => domain::Message::OpenUrlExternal(url),
        _ => return Task::none(),
    };
    let placement = placement(app);
    let error = domain::update(&mut app.work_browser, #[cfg(not(target_arch = "wasm32"))] &mut app.work_browser_controller, &placement, message);
    finish(app, error)
}
pub(super) fn sync_lifecycle(app: &mut App) -> Task<Message> {
    let placement = placement(app);
    let error = domain::sync_lifecycle(&mut app.work_browser, #[cfg(not(target_arch = "wasm32"))] &mut app.work_browser_controller, &placement);
    finish(app, error)
}
#[cfg(all(test, not(target_arch = "wasm32")))]
fn sync_lifecycle_result(app: &mut App) -> Result<bool, String> {
    let placement = placement(app);
    domain::sync_lifecycle_result(&mut app.work_browser, &mut app.work_browser_controller, &placement)
}
#[cfg(test)]
const BROWSER_BOTTOM_INSET: f32 = crate::app::impls::STATUS_BAR_HEIGHT + domain::BROWSER_BOTTOM_GAP;
#[cfg(test)]
#[path = "work_browser_tests.rs"]
mod work_browser_controller_ui_tests;
