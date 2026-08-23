//! 处理预览标签、编辑位置历史与工作区切换相关消息。

use super::PreviewMessage;
use crate::app::message::ViewMessage;
use crate::app::{
    App, AppTab, FocusArea, Message, PreviewAutoSaveMode, PreviewTab, Screen, set_config_field,
    state::{WorkResourceMode, WorkspaceMode},
};
use iced::Task;
use iced::{Font, widget};

const TRACE_HISTORY_LIMIT: usize = 256;

fn push_trace_entry(entries: &mut Vec<(String, usize, usize)>, entry: (String, usize, usize)) {
    if entries.last().is_some_and(|last| *last == entry) {
        return;
    }
    entries.push(entry);
    if entries.len() > TRACE_HISTORY_LIMIT {
        entries.remove(0);
    }
}

fn current_preview_location(app: &App) -> Option<(String, usize, usize)> {
    let path = app.active_preview_path.as_deref()?.to_string();
    let tab = app.preview_tabs.iter().find(|t| t.path == path)?;
    #[cfg(not(target_arch = "wasm32"))]
    {
        let (line, col) = tab.editor.cursor_position();
        Some((path, line, col))
    }
    #[cfg(target_arch = "wasm32")]
    {
        if let Some((ctx_path, line, col, _, _)) = app.preview_context_target.as_ref()
            && *ctx_path == path
        {
            return Some((path, line.saturating_sub(1), col.saturating_sub(1)));
        }
        let _ = tab;
        Some((path, 0, 0))
    }
}

fn preview_is_dirty(tab: &PreviewTab) -> bool {
    tab.is_dirty
}

fn gateway_preview_target(
    project_path: Option<&str>,
    path: &str,
) -> Result<(String, String), String> {
    let preview_path = std::path::Path::new(path);

    if let Some(project_root) = project_path {
        let project_root_path = std::path::Path::new(project_root);
        if preview_path.is_absolute()
            && let Ok(relative) = preview_path.strip_prefix(project_root_path)
            && let Some(relative) = relative.to_str()
        {
            let relative = relative.replace('\\', "/");
            if !relative.is_empty() {
                return Ok((project_root.to_string(), relative));
            }
        }

        if !preview_path.is_absolute() {
            return Ok((project_root.to_string(), path.replace('\\', "/")));
        }
    }

    let parent = preview_path
        .parent()
        .and_then(|value| value.to_str())
        .filter(|value| !value.is_empty())
        .ok_or_else(|| format!("无法确定预览文件目录: {path}"))?;
    let file_name = preview_path
        .file_name()
        .and_then(|value| value.to_str())
        .filter(|value| !value.is_empty())
        .ok_or_else(|| format!("无法确定预览文件名: {path}"))?;

    Ok((parent.to_string(), file_name.to_string()))
}

async fn load_preview_text_content(project_path: Option<String>, path: String) -> (String, bool) {
    let result = async {
        let client = crate::app::gateway_client()?;
        let (directory, relative_path) = gateway_preview_target(project_path.as_deref(), &path)?;
        let response =
            client.file_read_in_directory(Some(&directory), None, &relative_path).await?;
        Ok::<(String, bool), String>(crate::app::preview::format_text_preview_content(
            &response.content,
        ))
    }
    .await;

    result.unwrap_or_else(|error| (format!("读取失败：{error}\n"), false))
}

async fn persist_preview_file(
    project_path: Option<String>,
    path: String,
    content: String,
) -> Result<(), String> {
    let client = crate::app::gateway_client()?;
    let (directory, relative_path) = gateway_preview_target(project_path.as_deref(), &path)?;
    client
        .file_write_in_directory(Some(&directory), None, &relative_path, &content, true)
        .await
        .map(|_| ())
}

fn record_trace_before_jump(app: &mut App, target: &(String, usize, usize)) {
    if app.preview_trace_navigating {
        return;
    }
    if let Some(current) = current_preview_location(app)
        && current != *target
    {
        push_trace_entry(&mut app.preview_trace_back, current);
        app.preview_trace_forward.clear();
    }
}

fn navigate_trace_back(app: &mut App) -> Task<Message> {
    let current = current_preview_location(app);
    while let Some(target) = app.preview_trace_back.pop() {
        if current.as_ref().is_some_and(|cur| *cur == target) {
            continue;
        }
        if let Some(cur) = current.clone() {
            push_trace_entry(&mut app.preview_trace_forward, cur);
        }
        app.preview_trace_navigating = true;
        app.pending_preview_goto = Some(target.clone());
        return Task::done(Message::Preview(PreviewMessage::Open(target.0)));
    }
    Task::none()
}

fn navigate_trace_forward(app: &mut App) -> Task<Message> {
    let current = current_preview_location(app);
    while let Some(target) = app.preview_trace_forward.pop() {
        if current.as_ref().is_some_and(|cur| *cur == target) {
            continue;
        }
        if let Some(cur) = current.clone() {
            push_trace_entry(&mut app.preview_trace_back, cur);
        }
        app.preview_trace_navigating = true;
        app.pending_preview_goto = Some(target.clone());
        return Task::done(Message::Preview(PreviewMessage::Open(target.0)));
    }
    Task::none()
}

/// 处理预览页签消息，并在文件读写或光标跳转需要时返回任务。
pub fn update(app: &mut App, message: PreviewMessage) -> Task<Message> {
    fn goto_with_retry(line: usize, col: usize) -> Task<Message> {
        let goto_msg = Message::Preview(PreviewMessage::EditorEvent(
            iced_code_editor::Message::GotoPosition(line, col),
        ));
        Task::done(goto_msg.clone())
            .chain(crate::app::message::after(std::time::Duration::from_millis(24), goto_msg))
    }

    match message {
        PreviewMessage::Open(path) => {
            let path_clone = path.clone();
            let reveal_work_resources = matches!(app.screen, Screen::Project)
                && app.workspace_mode == WorkspaceMode::Work
                && (!app.work_resources_visible
                    || app.work_resource_mode != WorkResourceMode::WorkspaceFiles
                    || app.work_chat_fullscreen
                    || app.work_chat_half_fullscreen);
            if reveal_work_resources {
                app.work_resources_visible = true;
                app.work_resource_mode = WorkResourceMode::WorkspaceFiles;
                app.work_chat_fullscreen = false;
                app.work_chat_half_fullscreen = false;
            }
            if let Some((pending_path, line, col)) = app.pending_preview_goto.clone()
                && pending_path == path_clone
            {
                record_trace_before_jump(app, &(pending_path, line, col));
            }
            if app.file_manager_show_changes {
                app.file_manager_show_changes = false;
                set_config_field(
                    "file_manager_show_changes",
                    serde_json::Value::Bool(app.file_manager_show_changes),
                );
            }
            if !app.show_file_manager {
                app.show_file_manager = true;
                set_config_field("show_file_manager", serde_json::Value::Bool(true));
            }
            app.show_diff = true;
            let mut load_task = Task::none();

            if let Some(tab) = app.preview_tabs.iter().find(|t| t.path == path) {
                app.active_preview_path = Some(tab.path.clone());
                app.focus_area = FocusArea::Preview;

                if let Some((pending_path, line, col)) = app.pending_preview_goto.clone()
                    && pending_path == path_clone
                {
                    app.pending_preview_goto = None;
                    app.preview_trace_navigating = false;
                    let goto_task = goto_with_retry(line, col);
                    load_task = goto_task;
                }
            } else {
                let title = std::path::Path::new(&path_clone)
                    .file_name()
                    .and_then(|s| s.to_str())
                    .unwrap_or(&path_clone)
                    .to_string();

                let ext = std::path::Path::new(&path_clone)
                    .extension()
                    .and_then(|s| s.to_str())
                    .map(|s| s.to_lowercase())
                    .unwrap_or_default();

                let instant_preview = matches!(
                    ext.as_str(),
                    "png"
                        | "jpg"
                        | "jpeg"
                        | "gif"
                        | "bmp"
                        | "webp"
                        | "svg"
                        | "zip"
                        | "tar"
                        | "gz"
                        | "rar"
                        | "7z"
                        | "bz2"
                        | "xz"
                        | "pdf"
                );

                let syntax = if ext.is_empty() { "plaintext" } else { ext.as_str() };

                let (initial_content, truncated) = if instant_preview {
                    crate::app::safe_preview(&path_clone)
                } else {
                    ("加载中…".to_string(), false)
                };

                let mut editor =
                    crate::app::components::editor::Editor::new(&initial_content, syntax);
                editor.set_font(Font::with_name("JetBrains Mono"));
                editor.set_theme(app.effective_editor_theme());
                editor.set_font_size(app.current_font_size);
                editor.set_line_height(app.current_line_height.clamp(10.0, 60.0));
                editor.set_ui_language(app.current_language);
                editor.set_search_replace_enabled(true);
                editor.configure_preview_context_menu(
                    app.current_language,
                    cfg!(not(target_arch = "wasm32")),
                );

                app.preview_tabs.push(PreviewTab {
                    path: path_clone.clone(),
                    title,
                    content: initial_content,
                    is_dirty: false,
                    truncated,
                    auto_save_revision: 0,
                    editor,
                    scroll_id: widget::Id::unique(),
                    #[cfg(not(target_arch = "wasm32"))]
                    lsp_server_key: None,
                    #[cfg(not(target_arch = "wasm32"))]
                    lsp_uri: None,
                    #[cfg(not(target_arch = "wasm32"))]
                    lsp_language_id: None,
                });
                app.active_preview_path = Some(path_clone.clone());
                app.focus_area = FocusArea::Preview;

                #[cfg(not(target_arch = "wasm32"))]
                {
                    super::lsp::sync_lsp_for_path(app, &path_clone);
                }

                if !instant_preview {
                    let project_path = app.project_path.clone();
                    let load_path = path_clone.clone();
                    let msg_path = path_clone.clone();
                    load_task = Task::perform(
                        load_preview_text_content(project_path, load_path),
                        move |(content, truncated)| {
                            Message::Preview(PreviewMessage::OpenLoaded {
                                path: msg_path.clone(),
                                content,
                                truncated,
                            })
                        },
                    );
                }
            }
            if matches!(app.screen, Screen::Project) {
                return if reveal_work_resources {
                    Task::batch(vec![
                        load_task,
                        Task::done(Message::View(ViewMessage::WorkResourceModeSelected(
                            WorkResourceMode::WorkspaceFiles,
                        ))),
                    ])
                } else {
                    load_task
                };
            }
            // Add a top-level Preview tab for this file and select it
            let title = std::path::Path::new(&path_clone)
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or(&path_clone)
                .to_string();
            let id = format!("preview:{}", path_clone);
            if !app.open_tabs.iter().any(|t| t.id == id) {
                app.open_tabs.push(AppTab {
                    id: id.clone(),
                    title,
                    screen: Screen::Preview,
                    project_path: None,
                });
            }
            app.active_tab_id = Some(id);
            app.screen = Screen::Preview;
            load_task
        }
        PreviewMessage::OpenLoaded { path, content, truncated } => {
            let editor_theme = app.effective_editor_theme();
            let font_size = app.current_font_size;
            let line_height = app.current_line_height;
            let language = app.current_language;

            if let Some(tab) = app.preview_tabs.iter_mut().find(|t| t.path == path) {
                tab.content = content.clone();
                tab.is_dirty = false;
                tab.truncated = truncated;
                let ext = std::path::Path::new(&path)
                    .extension()
                    .and_then(|s| s.to_str())
                    .map(|s| s.to_lowercase())
                    .unwrap_or_default();
                let syntax = if ext.is_empty() { "plaintext" } else { ext.as_str() };

                let mut editor = crate::app::components::editor::Editor::new(&content, syntax);
                editor.set_font(Font::with_name("JetBrains Mono"));
                editor.set_theme(editor_theme);
                editor.set_font_size(font_size);
                editor.set_line_height(line_height.clamp(10.0, 60.0));
                editor.set_ui_language(language);
                editor.set_search_replace_enabled(true);
                editor.configure_preview_context_menu(language, cfg!(not(target_arch = "wasm32")));
                tab.editor = editor;
                #[cfg(not(target_arch = "wasm32"))]
                {
                    super::lsp::sync_lsp_for_path(app, &path);
                }
            }

            if let Some((pending_path, line, col)) = app.pending_preview_goto.clone()
                && pending_path == path
            {
                app.pending_preview_goto = None;
                app.preview_trace_navigating = false;
                return goto_with_retry(line, col);
            }

            Task::none()
        }
        PreviewMessage::TraceBack => navigate_trace_back(app),
        PreviewMessage::TraceForward => navigate_trace_forward(app),
        PreviewMessage::AutoSaveModeChanged(mode) => {
            app.preview_auto_save_mode = mode;
            crate::app::config::update_system_settings_config_async(move |system| {
                system.preview_auto_save = mode;
            })
        }
        PreviewMessage::Select(path) => {
            if path.is_empty() {
                app.active_preview_path = None;
            } else {
                app.active_preview_path = Some(path.clone());
                app.focus_area = FocusArea::Preview;
            }
            Task::none()
        }
        PreviewMessage::Close(path) => {
            if let Some(pos) = app.preview_tabs.iter().position(|t| t.path == path) {
                #[cfg(not(target_arch = "wasm32"))]
                {
                    if let Some(tab) = app.preview_tabs.get_mut(pos) {
                        super::lsp::detach_lsp_for_tab(tab);
                    }
                }
                app.preview_tabs.remove(pos);
                if app.active_preview_path == Some(path.clone()) {
                    app.active_preview_path = app.preview_tabs.last().map(|t| t.path.clone());
                }
            }
            Task::none()
        }
        PreviewMessage::SaveFile => {
            let Some(path) = app.active_preview_path.clone() else {
                return Task::none();
            };
            Task::done(Message::Preview(PreviewMessage::SaveFilePath { path, notify: false }))
        }
        PreviewMessage::SaveFilePath { path, notify } => {
            let Some(tab) = app.preview_tabs.iter().find(|tab| tab.path == path) else {
                return Task::none();
            };
            if !preview_is_dirty(tab) {
                app.show_preview_context_menu = false;
                return Task::none();
            }

            let project_path = app.project_path.clone();
            let content = tab.editor.content();
            app.show_preview_context_menu = false;

            Task::perform(
                persist_preview_file(project_path, path.clone(), content.clone()),
                move |result| {
                    Message::Preview(PreviewMessage::SaveFileFinished {
                        path: path.clone(),
                        content: content.clone(),
                        notify,
                        result,
                    })
                },
            )
        }
        PreviewMessage::SaveFileFinished { path, content, notify, result } => match result {
            Ok(()) => {
                if let Some(tab) = app.preview_tabs.iter_mut().find(|tab| tab.path == path) {
                    let editor_matches_saved_content = tab.editor.content() == content;
                    tab.content = content;
                    if editor_matches_saved_content {
                        tab.editor.inner.mark_saved();
                        tab.is_dirty = false;
                    } else {
                        tab.is_dirty = true;
                    }
                    #[cfg(not(target_arch = "wasm32"))]
                    {
                        tab.editor.lsp_did_save();
                    }
                }
                if notify { app.show_success_toast("已保存") } else { Task::none() }
            }
            Err(error) => app.show_error_toast(format!("保存失败: {error}")),
        },
        PreviewMessage::AutoSaveDelayElapsed { path, revision } => {
            if !matches!(
                app.preview_auto_save_mode,
                PreviewAutoSaveMode::AfterDelay | PreviewAutoSaveMode::OnFocusChange
            ) {
                return Task::none();
            }

            let Some(tab) = app.preview_tabs.iter().find(|tab| tab.path == path) else {
                return Task::none();
            };
            if tab.auto_save_revision != revision || !preview_is_dirty(tab) {
                return Task::none();
            }

            Task::done(Message::Preview(PreviewMessage::SaveFilePath { path, notify: false }))
        }
        PreviewMessage::WindowUnfocused => {
            if !matches!(
                app.preview_auto_save_mode,
                PreviewAutoSaveMode::OnFocusChange | PreviewAutoSaveMode::OnWindowChange
            ) {
                return Task::none();
            }

            let Some(path) = app.active_preview_path.clone() else {
                return Task::none();
            };
            let Some(tab) = app.preview_tabs.iter().find(|tab| tab.path == path) else {
                return Task::none();
            };
            if !preview_is_dirty(tab) {
                return Task::none();
            }

            Task::done(Message::Preview(PreviewMessage::SaveFilePath { path, notify: false }))
        }
        PreviewMessage::PathSegmentClicked(path, position) => {
            super::dismiss_preview_popup_menus(app);
            app.git_diff_context_menu = None;

            #[cfg(not(target_arch = "wasm32"))]
            {
                super::lsp::clear_lsp_hover(app);
                super::lsp::clear_lsp_completion(app, true);
            }
            let p = std::path::Path::new(&path);
            let list_root = if p.is_dir() {
                Some(p)
            } else if p.is_file() {
                p.parent()
            } else {
                None
            };

            if let Some(list_root) = list_root
                && let Ok(entries) = std::fs::read_dir(list_root)
            {
                let mut items = Vec::new();
                for entry in entries.flatten() {
                    let name = entry.file_name().to_string_lossy().to_string();
                    if name.starts_with('.') {
                        continue;
                    }
                    let is_dir = entry.file_type().map(|t| t.is_dir()).unwrap_or(false);
                    items.push((name, is_dir));
                }
                items.sort_by(|a, b| match (a.1, b.1) {
                    (true, false) => std::cmp::Ordering::Less,
                    (false, true) => std::cmp::Ordering::Greater,
                    _ => a.0.to_lowercase().cmp(&b.0.to_lowercase()),
                });

                let (x, y) = position.unwrap_or((app.cursor_position.x, app.cursor_position.y));
                app.preview_nav_popup =
                    Some((list_root.to_string_lossy().to_string(), x, y, items));
                Task::none()
            } else {
                Task::none()
            }
        }
        PreviewMessage::CloseNavPopup => {
            #[cfg(not(target_arch = "wasm32"))]
            {
                super::lsp::clear_lsp_hover(app);
                super::lsp::clear_lsp_completion(app, false);
            }
            app.preview_nav_popup = None;
            Task::none()
        }
        PreviewMessage::TabRightClicked(path, x, y) => {
            super::dismiss_preview_popup_menus(app);
            app.git_diff_context_menu = None;

            #[cfg(not(target_arch = "wasm32"))]
            {
                super::lsp::clear_lsp_hover(app);
                super::lsp::clear_lsp_completion(app, false);
            }

            app.preview_tab_menu_path = Some(path);
            app.preview_tab_menu_pos = Some(iced::Point::new(x, y));
            Task::none()
        }
        PreviewMessage::TabMenuClose => {
            app.preview_tab_menu_path = None;
            app.preview_tab_menu_pos = None;
            Task::none()
        }
        PreviewMessage::TabMenuCloseLeft(path) => {
            if let Some(pos) = app.preview_tabs.iter().position(|t| t.path == path) {
                let removed_count = pos;
                for _ in 0..removed_count {
                    #[cfg(not(target_arch = "wasm32"))]
                    {
                        if let Some(tab) = app.preview_tabs.first_mut() {
                            super::lsp::detach_lsp_for_tab(tab);
                        }
                    }
                    app.preview_tabs.remove(0);
                }
                if app
                    .active_preview_path
                    .as_ref()
                    .is_some_and(|p| !app.preview_tabs.iter().any(|t| &t.path == p))
                {
                    app.active_preview_path = app.preview_tabs.first().map(|t| t.path.clone());
                }
            }
            app.preview_tab_menu_path = None;
            app.preview_tab_menu_pos = None;
            Task::none()
        }
        PreviewMessage::TabMenuCloseRight(path) => {
            if let Some(pos) = app.preview_tabs.iter().position(|t| t.path == path) {
                while app.preview_tabs.len() > pos + 1 {
                    #[cfg(not(target_arch = "wasm32"))]
                    {
                        if let Some(tab) = app.preview_tabs.last_mut() {
                            super::lsp::detach_lsp_for_tab(tab);
                        }
                    }
                    app.preview_tabs.pop();
                }
                if app
                    .active_preview_path
                    .as_ref()
                    .is_some_and(|p| !app.preview_tabs.iter().any(|t| &t.path == p))
                {
                    app.active_preview_path = app.preview_tabs.last().map(|t| t.path.clone());
                }
            }
            app.preview_tab_menu_path = None;
            app.preview_tab_menu_pos = None;
            Task::none()
        }
        PreviewMessage::TabMenuCloseAll => {
            #[cfg(not(target_arch = "wasm32"))]
            {
                for tab in &mut app.preview_tabs {
                    super::lsp::detach_lsp_for_tab(tab);
                }
            }
            app.preview_tabs.clear();
            app.active_preview_path = None;
            app.preview_tab_menu_path = None;
            app.preview_tab_menu_pos = None;
            Task::none()
        }
        _ => Task::none(),
    }
}

#[cfg(test)]
#[path = "tab_tests.rs"]
mod tests;
