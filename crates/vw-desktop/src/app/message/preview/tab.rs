//! 桌面跨域效果编排；预览标签业务由 desktop-editor 实现。
use super::PreviewMessage;
use crate::app::{App, AppTab, FocusArea, Message, Screen, state::{WorkResourceMode, WorkspaceMode}};
use crate::app::message::ViewMessage;
use desktop_editor::preview::tab::{self as domain, Appearance, Effect, HostAction, TabState};
use iced::Task;
#[cfg(test)]
use crate::app::{PreviewAutoSaveMode, PreviewTab};
use serde_json::{Map, Value};

pub(super) fn state(app: &mut App) -> TabState<'_> {
    TabState {
        active_preview_path: &mut app.navigation_tabs.active_preview_path,
        preview_tabs: &mut app.navigation_tabs.preview_tabs,
        preview_context_target: &mut app.preview_context_target,
        preview_trace_back: &mut app.preview_trace_back,
        preview_trace_forward: &mut app.preview_trace_forward,
        preview_trace_navigating: &mut app.preview_trace_navigating,
        pending_preview_goto: &mut app.pending_preview_goto,
        preview_messages: &mut app.preview_messages,
        preview_auto_save_mode: &mut app.preview_auto_save_mode,
        show_preview_context_menu: &mut app.show_preview_context_menu,
        preview_context_menu_pos: &mut app.preview_context_menu_pos,
        preview_nav_popup: &mut app.preview_nav_popup,
        preview_tab_menu_path: &mut app.navigation_tabs.preview_tab_menu_path,
        preview_tab_menu_pos: &mut app.navigation_tabs.preview_tab_menu_pos,
    }
}
pub fn update(app: &mut App, message: PreviewMessage) -> Task<Message> {
    let opening = if let PreviewMessage::Open(path) = &message { Some(path.clone()) } else { None };
    domain::prepare_popover(&mut app.editor_popover, &message);
    let reveal_work_resources = opening.is_some() && matches!(app.navigation_tabs.screen, Screen::Project)
        && app.navigation_mode.workspace_mode == WorkspaceMode::Work
        && (!app.navigation_workspace.work_resources_visible || app.navigation_workspace.work_resource_mode != WorkResourceMode::WorkspaceFiles || app.navigation_workspace.work_chat_fullscreen || app.navigation_workspace.work_chat_half_fullscreen);
    let mut save_task = Task::none();
    if opening.is_some() {
        if reveal_work_resources {
            app.navigation_workspace.work_resources_visible = true;
            app.navigation_workspace.work_resource_mode = WorkResourceMode::WorkspaceFiles;
            app.navigation_workspace.work_chat_fullscreen = false;
            app.navigation_workspace.work_chat_half_fullscreen = false;
        }
        let mut fields = Map::new();
        if app.file_manager.file_manager_show_changes { app.file_manager.file_manager_show_changes = false; fields.insert("file_manager_show_changes".into(), Value::Bool(false)); }
        if !app.file_manager.show_file_manager { app.file_manager.show_file_manager = true; fields.insert("show_file_manager".into(), Value::Bool(true)); }
        save_task = app.save_desktop_preferences(fields);
        app.navigation_panels.show_diff = true;
    }
    let auto_save_mode = if let PreviewMessage::AutoSaveModeChanged(mode) = &message {
        #[cfg(not(target_arch = "wasm32"))] app.note_startup_setting_edit("preview_auto_save");
        Some(*mode)
    } else { None };
    if matches!(message, PreviewMessage::PathSegmentClicked(..) | PreviewMessage::TabRightClicked(..)) { app.git.git_diff_context_menu = None; }
    #[cfg(not(target_arch = "wasm32"))]
    if matches!(message, PreviewMessage::PathSegmentClicked(..) | PreviewMessage::TabRightClicked(..) | PreviewMessage::CloseNavPopup) {
        super::lsp::clear_lsp_hover(app);
        super::lsp::clear_lsp_completion(app, matches!(message, PreviewMessage::PathSegmentClicked(..)));
    }
    let appearance = Appearance { theme: app.effective_editor_theme(), font_size: app.current_font_size, line_height: app.current_line_height, locale: app.desktop_locale, cursor_position: app.cursor_position };
    let mut actions = Vec::new();
    let task = domain::update(&mut state(app), appearance, &mut actions, message);
    let mut tasks = Vec::new();
    for action in actions {
        match action {
            HostAction::FocusPreview => app.focus_area = FocusArea::Preview,
            HostAction::SyncLsp(path) => {
                #[cfg(not(target_arch = "wasm32"))] super::lsp::sync_lsp_for_path(app, &path);
                #[cfg(target_arch = "wasm32")] let _ = path;
            }
            HostAction::Toast { success, source, arguments } => tasks.push(app.show_localized_toast(if success { crate::app::state::ToastKind::Success } else { crate::app::state::ToastKind::Error }, source, arguments.into_iter().map(Into::into).collect())),
        }
    }
    let project_path = app.navigation_tabs.project_path.clone();
    let task = task.then(move |effect| match effect {
        Effect::Preview(message) => Task::done(Message::Preview(message)),
        Effect::GotoRetry(line, col) => {
            let message = Message::Preview(PreviewMessage::EditorEvent(iced_code_editor::Message::GotoPosition(line, col)));
            Task::done(message.clone()).chain(crate::app::message::after(std::time::Duration::from_millis(24), message))
        }
        Effect::Load(path) => {
            let project_path = project_path.clone();
            let load_path = path.clone();
            Task::perform(async move { domain::load_preview_text_content(crate::app::gateway_client(), project_path, load_path).await }, move |result| Message::Preview(match result { Ok((content, truncated)) => PreviewMessage::OpenLoaded { path: path.clone(), content, truncated }, Err(error) => PreviewMessage::OpenFailed { path: path.clone(), error } }))
        }
        Effect::Save { path, content, notify } => {
            let project_path = project_path.clone();
            let save_path = path.clone(); let save_content = content.clone();
            Task::perform(async move { domain::persist_preview_file(crate::app::gateway_client(), project_path, save_path, save_content).await }, move |result| Message::Preview(PreviewMessage::SaveFileFinished { path: path.clone(), content: content.clone(), notify, result }))
        }
    });
    if let Some(mode) = auto_save_mode { tasks.push(crate::app::config::update_system_settings_config_async(move |system| system.preview_auto_save = mode)); }
    if let Some(path) = opening {
        if matches!(app.navigation_tabs.screen, Screen::Project) {
            if reveal_work_resources { tasks.push(Task::done(Message::View(ViewMessage::WorkResourceModeSelected(WorkResourceMode::WorkspaceFiles)))); }
        } else {
            let title = app.navigation_tabs.preview_tabs.iter().find(|tab| tab.path == path).map(|tab| tab.title.clone()).unwrap_or_else(|| path.clone());
            let id = format!("preview:{}", path);
            if !app.navigation_tabs.open_tabs.iter().any(|tab| tab.id == id) { app.navigation_tabs.open_tabs.push(AppTab { id: id.clone(), title, screen: Screen::Preview, project_path: None }); }
            app.navigation_tabs.active_tab_id = Some(id);
            app.navigation_tabs.screen = Screen::Preview;
        }
    }
    tasks.push(task); tasks.push(save_task);
    Task::batch(tasks)
}
#[cfg(test)]
#[path = "tab_tests.rs"]
mod tests;
