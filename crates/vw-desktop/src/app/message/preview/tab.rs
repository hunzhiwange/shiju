//! 桌面跨域效果编排；预览标签业务由 desktop-editor 实现。
use super::PreviewMessage;
use crate::app::{App, AppTab, FocusArea, Message, Screen, state::{WorkResourceMode, WorkspaceMode}};
use crate::app::message::ViewMessage;
use desktop_navigation::panel::tabs::EDITOR_TAB_ID;
use desktop_editor::preview::tab::{self as domain, Appearance, Effect, HostAction, TabState};
use iced::Task;
#[cfg(test)]
use crate::app::{PreviewAutoSaveMode, PreviewTab};
use serde_json::{Map, Value};

pub(super) fn state(app: &mut App) -> TabState<'_> {
    let project_path = app.navigation_tabs.project_path.clone();
    state_for_project(app, project_path.as_deref()).expect("active preview state")
}

fn state_for_project<'a>(app: &'a mut App, project_path: Option<&str>) -> Option<TabState<'a>> {
    let navigation = &mut app.navigation_tabs;
    let preview_tabs = if navigation.project_path.as_deref() == project_path {
        &mut navigation.preview_tabs
    } else if let Some(path) = project_path {
        navigation.project_preview_tabs.get_mut(path)?
    } else {
        &mut navigation.standalone_preview_tabs
    };
    Some(TabState {
        active_preview_path: &mut navigation.active_preview_path,
        preview_tabs,
        preview_context_target: &mut app.preview_state.preview_context_target,
        preview_trace_back: &mut app.preview_state.preview_trace_back,
        preview_trace_forward: &mut app.preview_state.preview_trace_forward,
        preview_trace_navigating: &mut app.preview_state.preview_trace_navigating,
        pending_preview_goto: &mut app.preview_state.pending_preview_goto,
        preview_messages: &mut app.preview_state.preview_messages,
        preview_auto_save_mode: &mut app.preview_state.preview_auto_save_mode,
        show_preview_context_menu: &mut app.preview_state.show_preview_context_menu,
        preview_context_menu_pos: &mut app.preview_state.preview_context_menu_pos,
        preview_nav_popup: &mut app.preview_state.preview_nav_popup,
        preview_tab_menu_path: &mut navigation.preview_tab_menu_path,
        preview_tab_menu_pos: &mut navigation.preview_tab_menu_pos,
    })
}

pub(crate) fn finish_io(
    app: &mut App,
    project_path: Option<&str>,
    message: PreviewMessage,
) -> Task<Message> {
    if project_path == app.navigation_tabs.project_path.as_deref() {
        return update(app, message);
    }
    let appearance = appearance(app);
    let mut pending_goto = None;
    let mut actions = Vec::new();
    if let Some(mut state) = state_for_project(app, project_path) {
        // Background file I/O must not move the caret in the visible editor.
        state.pending_preview_goto = &mut pending_goto;
        let _ = domain::update(&mut state, appearance, &mut actions, message);
    }
    Task::batch(actions.into_iter().filter_map(|action| match action {
        HostAction::Toast { success, source, arguments } => Some(show_toast(app, success, source, arguments)),
        _ => None,
    }))
}

fn show_toast(app: &mut App, success: bool, source: &'static str, arguments: Vec<String>) -> Task<Message> {
    let kind = if success { crate::app::state::ToastKind::Success } else { crate::app::state::ToastKind::Error };
    app.show_localized_toast(kind, source, arguments.into_iter().map(Into::into).collect())
}

fn appearance(app: &App) -> Appearance {
    Appearance {
        theme: app.effective_editor_theme(),
        font_size: app.preview_state.current_font_size,
        line_height: app.preview_state.current_line_height,
        locale: app.general_settings.desktop_locale,
        cursor_position: app.cursor_position,
    }
}
pub fn update(app: &mut App, message: PreviewMessage) -> Task<Message> {
    let opening = if let PreviewMessage::Open(path) = &message { Some(path.clone()) } else { None };
    domain::prepare_popover(&mut app.preview_state.editor_popover, &message);
    let reveal_work_resources = opening.is_some() && matches!(app.navigation_tabs.screen, Screen::Project)
        && app.navigation_mode.workspace_mode == WorkspaceMode::Work
        && (!app.navigation_workspace.work_resources_visible || app.navigation_workspace.work_resource_mode != WorkResourceMode::WorkspaceFiles || app.navigation_workspace.work_chat_fullscreen || app.navigation_workspace.work_chat_half_fullscreen);
    let mut save_task = Task::none();
    if opening.is_some() {
        if app.navigation_tabs.screen != Screen::Project {
            app.navigation_tabs.switch_preview_project(None);
        }
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
    let appearance = appearance(app);
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
            HostAction::Toast { success, source, arguments } => tasks.push(show_toast(app, success, source, arguments)),
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
            Task::future(async move {
                let result = domain::load_preview_text_content(
                    crate::app::gateway_client(), project_path.clone(), path.clone(),
                ).await;
                Message::PreviewIoFinished {
                    project_path,
                    message: match result {
                        Ok((content, truncated)) => PreviewMessage::OpenLoaded { path, content, truncated },
                        Err(error) => PreviewMessage::OpenFailed { path, error },
                    },
                }
            })
        }
        Effect::Save { path, content, notify } => {
            let project_path = project_path.clone();
            Task::future(async move {
                let result = domain::persist_preview_file(
                    crate::app::gateway_client(), project_path.clone(), path.clone(), content.clone(),
                ).await;
                Message::PreviewIoFinished {
                    project_path,
                    message: PreviewMessage::SaveFileFinished { path, content, notify, result },
                }
            })
        }
    });
    if let Some(mode) = auto_save_mode { tasks.push(crate::app::config::update_system_settings_config_async(move |system| system.preview_auto_save = mode)); }
    if opening.is_some() {
        if matches!(app.navigation_tabs.screen, Screen::Project) {
            if reveal_work_resources { tasks.push(Task::done(Message::View(ViewMessage::WorkResourceModeSelected(WorkResourceMode::WorkspaceFiles)))); }
        } else {
            if !app.navigation_tabs.open_tabs.iter().any(|tab| tab.id == EDITOR_TAB_ID) {
                app.navigation_tabs.open_tabs.push(AppTab {
                    id: EDITOR_TAB_ID.to_owned(),
                    title: desktop_i18n::I18n::new(app.general_settings.desktop_locale).__("编辑器").to_owned(),
                    screen: Screen::Preview,
                    project_path: None,
                });
            }
            app.navigation_tabs.active_tab_id = Some(EDITOR_TAB_ID.to_owned());
            app.navigation_tabs.screen = Screen::Preview;
        }
    }
    tasks.push(task); tasks.push(save_task);
    Task::batch(tasks)
}
#[cfg(test)]
#[path = "tab_tests.rs"]
mod tests;
