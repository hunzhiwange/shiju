//! 预览面板视图组件。
//!
//! 本模块负责预览内容、菜单、面包屑、LSP 标识或浮层宿主的局部构建。

mod content;
mod editor_popovers;
#[cfg(test)]
#[path = "editor_popovers_tests.rs"]
mod editor_popovers_tests;
/// header_tabs 子模块承载当前组件的一部分独立职责。
mod header_tabs;
/// menus 子模块承载当前组件的一部分独立职责。
mod menus;
/// settings 子模块承载当前组件的一部分独立职责。
mod settings;

#[cfg(test)]
#[path = "content_tests.rs"]
mod content_tests;
#[cfg(test)]
#[path = "header_tabs_tests.rs"]
mod header_tabs_tests;
#[cfg(test)]
#[path = "menus_tests.rs"]
mod menus_tests;
#[cfg(test)]
#[path = "mod_tests.rs"]
mod mod_tests;
#[cfg(test)]
#[path = "settings_tests.rs"]
mod settings_tests;

use crate::app::components::editor_toolbar;
use crate::app::{App, Message, message};
use iced::Element;

/// 重新导出 content::build_content_base，让上层模块通过稳定路径访问。
pub use content::build_content_base;
/// 重新导出 header_tabs::build_header_tabs，让上层模块通过稳定路径访问。
pub use header_tabs::build_header_tabs;
/// 重新导出 menus::build_menu_ui，让上层模块通过稳定路径访问。
pub use menus::build_menu_ui;
/// 重新导出 settings::build_settings_overlay，让上层模块通过稳定路径访问。
pub use settings::build_settings_overlay;

fn show_fullscreen_controls(app: &App) -> bool {
    matches!(app.navigation_tabs.screen, crate::app::Screen::Project)
        && matches!(app.navigation_mode.workspace_mode, crate::app::state::WorkspaceMode::Code)
}

pub fn view(app: &App) -> Element<'_, Message> {
    use desktop_editor::components::preview_panel::view::layout::{self, ViewInput, ViewParts, ViewMessage};
    #[cfg(not(target_arch = "wasm32"))]
    let lsp_overlay_show = app.navigation_tabs.active_preview_path.is_some()
        && app.preview_state.lsp_overlay_path.as_ref() == app.navigation_tabs.active_preview_path.as_ref()
        && (app.preview_state.lsp_overlay.hover_visible || app.preview_state.lsp_overlay.completion_visible);
    #[cfg(not(target_arch = "wasm32"))]
    tracing::debug!(
        "[LSP OVERLAY] lsp_overlay_show={}, hover_visible={}, completion_visible={}, context_menu={}, nav_popup={}",
        lsp_overlay_show,
        app.preview_state.lsp_overlay.hover_visible,
        app.preview_state.lsp_overlay.completion_visible,
        app.preview_state.show_preview_context_menu,
        app.preview_state.preview_nav_popup.is_some()
    );

    let tab = app.navigation_tabs.preview_tabs.iter().find(|tab| Some(&tab.path) == app.navigation_tabs.active_preview_path.as_ref());
    layout::view(ViewInput {
        active_path: app.navigation_tabs.active_preview_path.as_deref(), project_path: app.navigation_tabs.project_path.as_deref(),
        read_only_title: tab.filter(|tab| tab.read_only).map(|tab| tab.title.as_str()),
        show_fullscreen_controls: show_fullscreen_controls(app), fullscreen: app.git.git_diff_fullscreen,
        show_changes: app.file_manager.file_manager_show_changes, show_context_menu: app.preview_state.show_preview_context_menu,
        context_menu_pos: app.preview_state.preview_context_menu_pos, i18n: desktop_i18n::I18n::new(app.general_settings.desktop_locale),
    }, ViewParts {
        header_tabs: build_header_tabs(app).map(ViewMessage::External),
        header_tools: tab.map(|tab| editor_toolbar::view(app,
            (!tab.read_only).then_some(Message::Preview(message::PreviewMessage::SaveFile)), tab.is_dirty).map(ViewMessage::External)),
        tools: editor_popovers::tools(app).map(ViewMessage::External),
        content: build_content_base(app).map(ViewMessage::External),
        menu_ui: build_menu_ui(app).map(ViewMessage::External),
        #[cfg(not(target_arch = "wasm32"))]
        lsp_badge: active_preview_lsp_badge(app).map(|badge| badge.map(ViewMessage::External)),
    }).map(|action| match action {
        ViewMessage::External(message) => message,
        ViewMessage::OpenPathInFinder(path) => Message::View(message::ViewMessage::OpenPathInFinder(path)),
        ViewMessage::ToggleHalfFullscreen => Message::Git(message::GitMessage::ToggleHalfFullscreen),
        ViewMessage::ToggleFullscreen => Message::Git(message::GitMessage::ToggleFullscreen),
        ViewMessage::ContextMenuClose => Message::Preview(message::PreviewMessage::ContextMenuClose),
    })
}

#[cfg(not(target_arch = "wasm32"))]
fn active_preview_lsp_badge(app: &App) -> Option<Element<'_, Message>> {
    let path = app.navigation_tabs.active_preview_path.as_deref()?;
    let tab = app.navigation_tabs.preview_tabs.iter().find(|tab| tab.path == path)?;
    if tab.read_only {
        return None;
    }
    let language = crate::app::lsp::config::lsp_language_for_path(std::path::Path::new(path));
    let progress = tab.lsp_server_key.is_some_and(|key| app.preview_state.lsp_progress.get(key).is_some_and(|entries| !entries.is_empty()));
    Some(desktop_editor::components::preview_panel::view::layout::lsp_badge(
        tab.lsp_server_key, progress, language.is_some(), &app.effective_editor_theme(),
        desktop_i18n::I18n::new(app.general_settings.desktop_locale)))
}



#[cfg(test)]
#[path = "editor_search_tests.rs"]
mod editor_search_tests;

#[cfg(test)]
#[path = "editor_tools_acceptance_tests.rs"]
mod editor_tools_acceptance_tests;

fn map_editor_view_message(action: desktop_editor::components::preview_panel::view::Message) -> Message {
    use desktop_editor::components::preview_panel::view::Message as Action;
    use crate::app::message::editor::EditorMessage;
    let message = match action {
        Action::EditorEvent(event) => return Message::Preview(message::PreviewMessage::EditorEvent(event)),
        Action::SetOption(option, enabled) => EditorMessage::SetOption(option, enabled),
        Action::SetWrapColumn(value) => EditorMessage::SetWrapColumn(value),
        Action::SetIndentStyle(style) => EditorMessage::SetIndentStyle(style),
        Action::ToggleSettings => EditorMessage::ToggleSettings,
        Action::ClosePopover => EditorMessage::ClosePopover,
        Action::OpenSearch => EditorMessage::OpenSearch,
        Action::OpenReplace => EditorMessage::OpenReplace,
        Action::TogglePopover(kind) => EditorMessage::TogglePopover(kind),
    };
    Message::Editor(message)
}
