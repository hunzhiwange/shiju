//! 桌面标题栏装配与局部操作映射。
use crate::app::{App, Message, message};
use crate::app::state::{ChatPanelMode, WorkspaceMode};
use iced::Element;
use desktop_chat_view::header::{self as view, HeaderAction, HeaderInput};
pub(super) use view::header_icon_button;
pub(super) fn show_half_fullscreen_control(_mode: WorkspaceMode) -> bool { view::show_half_fullscreen_control() }

fn input(app: &App) -> HeaderInput<'_> {
    let runtime = app.active_session_id.as_ref().and_then(|id| app.session_runtime_states.get(id));
    let chrome = crate::app::theme_catalog::resolve_chrome_style_from_skin(app.active_theme_skin.as_ref(), &app.app_theme);
    HeaderInput {
        active_session_id: app.active_session_id.clone(),
        sessions: &app.sessions, project_sessions: &app.project_sessions,
        project_path: app.navigation_tabs.project_path.clone(), desktop_locale: app.desktop_locale,
        chat_panel_mode: app.chat_panel_mode == ChatPanelMode::Trajectory,
        fullscreen: app.is_chat_fullscreen(), half_fullscreen: app.is_chat_half_fullscreen(),
        show_session_actions_popover: app.popover_visibility.session_actions,
        status_animation_frame: app.status_animation_frame,
        title: super::utils::get_session_title(app),
        usage_percent: crate::app::components::input_panel::usage::get_usage_rate_percent(app),
        usage_details: crate::app::components::input_panel::usage::get_usage_details(app),
        input_limit: app.usage_panel.usage_model_info.as_ref().map(|info| info.input_limit).unwrap_or(0),
        limit_is_default: app.usage_panel.usage_model_info.as_ref().is_some_and(|info| info.limit_is_default),
        runtime: (runtime.is_some_and(|r| r.request_lifecycle.is_requesting), runtime.map(|r| r.request_lifecycle.queue.len()).unwrap_or(0), runtime.is_some_and(|r| r.has_unseen_success)),
        now_ms: crate::app::time::now_ms(), header_background: chrome.chat_header_bg,
    }
}
fn map(action: HeaderAction) -> Message {
    use message::{ChatMessage as C, ProjectMessage as P, ViewMessage as V};
    match action {
        HeaderAction::None => Message::None,
        HeaderAction::Title(id) => Message::Project(P::SessionTitleClicked(id)),
        HeaderAction::Mode(trajectory) => Message::Chat(C::ChatPanelModeSelected(if trajectory { ChatPanelMode::Trajectory } else { ChatPanelMode::Conversation })),
        HeaderAction::Fullscreen => Message::Chat(C::ToggleFullscreen),
        HeaderAction::HalfFullscreen => Message::Chat(C::ToggleHalfFullscreen),
        HeaderAction::Usage => Message::View(V::OpenUsage),
        HeaderAction::ToggleActions => Message::View(V::ToggleSessionActionsPopover),
        HeaderAction::ClosePopovers => Message::View(V::ClosePopovers),
        HeaderAction::Rename(id) => Message::Project(P::SessionRenamePressed(id)),
        HeaderAction::Archive(id) => Message::Project(P::SessionArchivePressed(id)),
        HeaderAction::Delete(id) => Message::Project(P::SessionDeletePressed(id)),
        HeaderAction::OpenParent(path, id) => Message::Project(P::OpenProjectSessionPressed(path, id)),
        HeaderAction::Compact { session_id, project_path } => Message::Chat(C::CompactSessionRequested { session_id, project_path }),
    }
}
pub fn chat_header_view(app: &App) -> Element<'_, Message> {
    let header = view::chat_header_view(&input(app)).map(map);
    let header_bg = app.active_theme_skin.as_ref().and_then(|skin| skin.chat_header_path.as_deref());
    let fit = app.active_theme_skin.as_ref().map(|skin| crate::app::theme_catalog::PanelImageFit::parse(skin.chrome.chat_header_image_fit.as_deref()).unwrap_or_default()).unwrap_or_default();
    crate::app::theme_catalog::with_panel_background(header, header_bg, fit)
}
pub(super) fn return_to_parent_session_pressed(app: &App) -> Option<Message> {
    view::return_to_parent_session_pressed(&input(app)).map(map)
}
#[cfg(test)]
use iced::{Background, Color, Theme};
#[cfg(test)]
use crate::app::assets::Icon;
#[cfg(test)]
use view::*;
#[cfg(test)]
fn session_workspace_badge_state(app: &App) -> Option<(String, bool)> { view::session_workspace_badge_state(&input(app)) }
#[cfg(test)]
fn get_session_status(app: &App) -> (bool, usize, bool, f32, Color) { view::get_session_status(&input(app)) }
#[cfg(test)]
fn session_actions_popover(app: &App) -> Element<'_, Message> { view::session_actions_popover(&input(app)).map(map) }
#[cfg(test)]
fn session_compact_message(app: &App) -> Option<Message> { view::session_compact_message(&input(app)).map(map) }
#[cfg(test)]
fn session_actions_button<'a>(label: &str, msg: Option<Message>) -> Element<'a, Message> {
    view::session_actions_button(label, msg.as_ref().map(|_| HeaderAction::None)).map(move |_| msg.clone().unwrap_or(Message::None))
}
#[cfg(test)]
#[path = "header_tests.rs"]
mod header_tests;

#[cfg(test)]
fn active_session_directory(app: &App) -> Option<String> { view::active_session_directory(&input(app)) }
#[cfg(test)]
fn session_workspace_badge(app: &App) -> Option<Element<'_, Message>> { view::session_workspace_badge(&input(app)).map(|element| element.map(map)) }
