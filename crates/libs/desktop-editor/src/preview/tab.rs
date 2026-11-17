//! 预览标签、历史导航、保存与菜单的局部更新。
use super::{PreviewTab, message::PreviewMessage};
use config_ui::PreviewAutoSaveMode;
use iced::{Task, Font, widget};

pub struct TabState<'a> {
    pub active_preview_path: &'a mut Option<String>,
    pub preview_tabs: &'a mut Vec<PreviewTab>,
    pub preview_context_target: &'a mut Option<(String, usize, usize, usize, usize)>,
    pub preview_trace_back: &'a mut Vec<(String, usize, usize)>,
    pub preview_trace_forward: &'a mut Vec<(String, usize, usize)>,
    pub preview_trace_navigating: &'a mut bool,
    pub pending_preview_goto: &'a mut Option<(String, usize, usize)>,
    pub preview_messages: &'a mut std::collections::HashMap<String, desktop_i18n::format::Message>,
    pub preview_auto_save_mode: &'a mut config_ui::PreviewAutoSaveMode,
    pub show_preview_context_menu: &'a mut bool,
    pub preview_context_menu_pos: &'a mut Option<(f32, f32)>,
    pub preview_nav_popup: &'a mut Option<(String, f32, f32, Vec<(String, bool)>)>,
    pub preview_tab_menu_path: &'a mut Option<String>,
    pub preview_tab_menu_pos: &'a mut Option<iced::Point>,
}
#[derive(Clone)]
pub enum Effect {
    Preview(PreviewMessage),
    GotoRetry(usize, usize),
    Load(String),
    Save { path: String, content: String, notify: bool },
}
pub enum HostAction {
    FocusPreview,
    SyncLsp(String),
    Toast { success: bool, source: &'static str, arguments: Vec<String> },
}
pub struct Appearance {
    pub theme: iced::Theme,
    pub font_size: f32,
    pub line_height: f32,
    pub locale: config_ui::DesktopLocale,
    pub cursor_position: iced::Point,
}
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

fn current_preview_location(app: &TabState<'_>) -> Option<(String, usize, usize)> {
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
    !tab.read_only && tab.is_dirty
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

pub async fn load_preview_text_content(
    client: Result<vw_gateway_client::GatewayClient, String>,
    project_path: Option<String>,
    path: String,
) -> Result<(String, bool), String> {
    let result = async {
        let client = client?;
        let (directory, relative_path) = gateway_preview_target(project_path.as_deref(), &path)?;
        let response = gateway_files::GatewayFilesClient::new(client.transport().clone())
            .file_read_in_directory(Some(&directory), None, &relative_path)
            .await?;
        Ok::<(String, bool), String>(preview_content::format_text_preview_content(
            &response.content,
        ))
    }
    .await;

    result
}

pub async fn persist_preview_file(
    client: Result<vw_gateway_client::GatewayClient, String>,
    project_path: Option<String>,
    path: String,
    content: String,
) -> Result<(), String> {
    let client = client?;
    let (directory, relative_path) = gateway_preview_target(project_path.as_deref(), &path)?;
    gateway_files::GatewayFilesClient::new(client.transport().clone())
        .file_write_in_directory(Some(&directory), None, &relative_path, &content, true)
        .await
        .map(|_| ())
}

fn record_trace_before_jump(app: &mut TabState<'_>, target: &(String, usize, usize)) {
    if *app.preview_trace_navigating {
        return;
    }
    if let Some(current) = current_preview_location(app)
        && current != *target
    {
        push_trace_entry(&mut *app.preview_trace_back, current);
        app.preview_trace_forward.clear();
    }
}

fn navigate_trace_back(app: &mut TabState<'_>) -> Task<Effect> {
    let current = current_preview_location(app);
    while let Some(target) = app.preview_trace_back.pop() {
        if current.as_ref().is_some_and(|cur| *cur == target) {
            continue;
        }
        if let Some(cur) = current.clone() {
            push_trace_entry(&mut *app.preview_trace_forward, cur);
        }
        (*app.preview_trace_navigating) = true;
        (*app.pending_preview_goto) = Some(target.clone());
        return Task::done(Effect::Preview(PreviewMessage::Open(target.0)));
    }
    Task::none()
}

fn navigate_trace_forward(app: &mut TabState<'_>) -> Task<Effect> {
    let current = current_preview_location(app);
    while let Some(target) = app.preview_trace_forward.pop() {
        if current.as_ref().is_some_and(|cur| *cur == target) {
            continue;
        }
        if let Some(cur) = current.clone() {
            push_trace_entry(&mut *app.preview_trace_back, cur);
        }
        (*app.preview_trace_navigating) = true;
        (*app.pending_preview_goto) = Some(target.clone());
        return Task::done(Effect::Preview(PreviewMessage::Open(target.0)));
    }
    Task::none()
}

/// 处理预览页签消息，并在文件读写或光标跳转需要时返回任务。
pub fn update(app: &mut TabState<'_>, appearance: Appearance, actions: &mut Vec<HostAction>, message: PreviewMessage) -> Task<Effect> {
    fn goto_with_retry(line: usize, col: usize) -> Task<Effect> {
        Task::done(Effect::GotoRetry(line, col))
    }
    match message {
        PreviewMessage::Open(path) => {
            let path_clone = path.clone();
            if let Some((pending_path, line, col)) = app.pending_preview_goto.clone()
                && pending_path == path_clone
            {
                record_trace_before_jump(app, &(pending_path, line, col));
            }
            let mut load_task = Task::none();

            if let Some(tab) = app.preview_tabs.iter().find(|t| t.path == path) {
                (*app.active_preview_path) = Some(tab.path.clone());
                actions.push(HostAction::FocusPreview);

                if let Some((pending_path, line, col)) = app.pending_preview_goto.clone()
                    && pending_path == path_clone
                {
                    (*app.pending_preview_goto) = None;
                    (*app.preview_trace_navigating) = false;
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
                    safe_preview(&path_clone)
                } else {
                    ("加载中…".to_string(), false)
                };

                let mut editor =
                    crate::components::editor::Editor::new(&initial_content, syntax);
                editor.set_font(Font::with_name("JetBrains Mono"));
                editor.set_theme(appearance.theme.clone());
                editor.set_font_size(appearance.font_size);
                editor.set_line_height(appearance.line_height.clamp(10.0, 60.0));
                editor.set_ui_language(desktop_i18n::editor::language(appearance.locale));
                editor.set_search_replace_enabled(true);
                editor.configure_preview_context_menu(appearance.locale, true);

                app.preview_messages.remove(&path_clone);
                if !instant_preview {
                    app.preview_messages.insert(
                        path_clone.clone(),
                        desktop_i18n::format::Message::new("加载中…", vec![]),
                    );
                }
                if let Some(description) = preview_description(&path_clone) {
                    app.preview_messages.insert(path_clone.clone(), description);
                }
                app.preview_tabs.push(PreviewTab {
                    path: path_clone.clone(),
                    title,
                    content: initial_content,
                    is_dirty: false,
                    truncated,
                    read_only: false,
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
                (*app.active_preview_path) = Some(path_clone.clone());
                actions.push(HostAction::FocusPreview);

                #[cfg(not(target_arch = "wasm32"))]
                {
                    actions.push(HostAction::SyncLsp(path_clone.clone()));
                }

                if !instant_preview {
                    load_task = Task::done(Effect::Load(path_clone.clone()));
                }
            }
            load_task
        }
        PreviewMessage::OpenFailed { path, error } => {
            if app.preview_tabs.iter().any(|tab| tab.path == path) {
                app.preview_messages.insert(
                    path,
                    desktop_i18n::format::Message::new("读取失败：%s\n", vec![error.into()]),
                );
            }
            Task::none()
        }
        PreviewMessage::OpenLoaded { path, content, truncated } => {
            app.preview_messages.remove(&path);
            let content = if truncated {
                content.strip_prefix("文件较大，预览已截断。\n\n").unwrap_or(&content).to_string()
            } else {
                content
            };
            let editor_theme = appearance.theme.clone();
            let font_size = appearance.font_size;
            let line_height = appearance.line_height;
            let language = desktop_i18n::editor::language(appearance.locale);

            if let Some(tab) = app.preview_tabs.iter_mut().find(|t| t.path == path) {
                if tab.read_only {
                    return Task::none();
                }
                tab.content = content.clone();
                tab.is_dirty = false;
                tab.truncated = truncated;
                let ext = std::path::Path::new(&path)
                    .extension()
                    .and_then(|s| s.to_str())
                    .map(|s| s.to_lowercase())
                    .unwrap_or_default();
                let syntax = if ext.is_empty() { "plaintext" } else { ext.as_str() };

                let mut editor = crate::components::editor::Editor::new(&content, syntax);
                editor.set_font(Font::with_name("JetBrains Mono"));
                editor.set_theme(editor_theme);
                editor.set_font_size(font_size);
                editor.set_line_height(line_height.clamp(10.0, 60.0));
                editor.set_ui_language(language);
                editor.set_search_replace_enabled(true);
                editor.configure_preview_context_menu(appearance.locale, true);
                tab.editor = editor;
                #[cfg(not(target_arch = "wasm32"))]
                {
                    actions.push(HostAction::SyncLsp(path.clone()));
                }
            }

            if let Some((pending_path, line, col)) = app.pending_preview_goto.clone()
                && pending_path == path
            {
                (*app.pending_preview_goto) = None;
                (*app.preview_trace_navigating) = false;
                return goto_with_retry(line, col);
            }

            Task::none()
        }
        PreviewMessage::TraceBack => navigate_trace_back(app),
        PreviewMessage::TraceForward => navigate_trace_forward(app),
        PreviewMessage::AutoSaveModeChanged(mode) => {
            *app.preview_auto_save_mode = mode;
            Task::none()
        }
        PreviewMessage::Select(path) => {
            if path.is_empty() {
                (*app.active_preview_path) = None;
            } else {
                (*app.active_preview_path) = Some(path.clone());
                actions.push(HostAction::FocusPreview);
            }
            Task::none()
        }
        PreviewMessage::Close(path) => {
            app.preview_messages.remove(&path);
            if let Some(pos) = app.preview_tabs.iter().position(|t| t.path == path) {
                #[cfg(not(target_arch = "wasm32"))]
                {
                    if let Some(tab) = app.preview_tabs.get_mut(pos) {
                        super::lsp::detach_lsp_for_tab(tab);
                    }
                }
                app.preview_tabs.remove(pos);
                if (*app.active_preview_path) == Some(path.clone()) {
                    (*app.active_preview_path) = app.preview_tabs.last().map(|t| t.path.clone());
                }
            }
            Task::none()
        }
        PreviewMessage::SaveFile => {
            let Some(path) = app.active_preview_path.clone() else {
                return Task::none();
            };
            Task::done(Effect::Preview(PreviewMessage::SaveFilePath { path, notify: false }))
        }
        PreviewMessage::SaveFilePath { path, notify } => {
            let Some(tab) = app.preview_tabs.iter().find(|tab| tab.path == path) else {
                return Task::none();
            };
            if !preview_is_dirty(tab) {
                (*app.show_preview_context_menu) = false;
                return Task::none();
            }

            let content = tab.editor.content();
            *app.show_preview_context_menu = false;
            Task::done(Effect::Save { path, content, notify })
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
                if notify {
                    actions.push(HostAction::Toast { success: true, source: "已保存", arguments: vec![] });
                }
                Task::none()
            }
            Err(error) => {
                actions.push(HostAction::Toast { success: false, source: "保存失败: %s", arguments: vec![error] });
                Task::none()
            }
        },
        PreviewMessage::AutoSaveDelayElapsed { path, revision } => {
            if !matches!(
                *app.preview_auto_save_mode,
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

            Task::done(Effect::Preview(PreviewMessage::SaveFilePath { path, notify: false }))
        }
        PreviewMessage::WindowUnfocused => {
            if !matches!(
                *app.preview_auto_save_mode,
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

            Task::done(Effect::Preview(PreviewMessage::SaveFilePath { path, notify: false }))
        }
        PreviewMessage::PathSegmentClicked(path, position) => {
            dismiss_preview_popup_menus(app);

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

                let (x, y) = position.unwrap_or((appearance.cursor_position.x, appearance.cursor_position.y));
                (*app.preview_nav_popup) =
                    Some((list_root.to_string_lossy().to_string(), x, y, items));
                Task::none()
            } else {
                Task::none()
            }
        }
        PreviewMessage::CloseNavPopup => {
            (*app.preview_nav_popup) = None;
            Task::none()
        }
        PreviewMessage::TabRightClicked(path, x, y) => {
            dismiss_preview_popup_menus(app);

            (*app.preview_tab_menu_path) = Some(path);
            (*app.preview_tab_menu_pos) = Some(iced::Point::new(x, y));
            Task::none()
        }
        PreviewMessage::TabMenuClose => {
            (*app.preview_tab_menu_path) = None;
            (*app.preview_tab_menu_pos) = None;
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
                if (*app.active_preview_path)
                    .as_ref()
                    .is_some_and(|p| !app.preview_tabs.iter().any(|t| &t.path == p))
                {
                    (*app.active_preview_path) = app.preview_tabs.first().map(|t| t.path.clone());
                }
            }
            (*app.preview_tab_menu_path) = None;
            (*app.preview_tab_menu_pos) = None;
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
                if (*app.active_preview_path)
                    .as_ref()
                    .is_some_and(|p| !app.preview_tabs.iter().any(|t| &t.path == p))
                {
                    (*app.active_preview_path) = app.preview_tabs.last().map(|t| t.path.clone());
                }
            }
            (*app.preview_tab_menu_path) = None;
            (*app.preview_tab_menu_pos) = None;
            Task::none()
        }
        PreviewMessage::TabMenuCloseAll => {
            #[cfg(not(target_arch = "wasm32"))]
            {
                for tab in app.preview_tabs.iter_mut() {
                    super::lsp::detach_lsp_for_tab(tab);
                }
            }
            app.preview_tabs.clear();
            (*app.active_preview_path) = None;
            (*app.preview_tab_menu_path) = None;
            (*app.preview_tab_menu_pos) = None;
            Task::none()
        }
        _ => Task::none(),
    }
}

pub fn dismiss_preview_popup_menus(app: &mut TabState<'_>) {
    *app.show_preview_context_menu = false;
    *app.preview_context_menu_pos = None;
    *app.preview_nav_popup = None;
    *app.preview_tab_menu_path = None;
    *app.preview_tab_menu_pos = None;
}

pub fn safe_preview(path: &str) -> (String, bool) {
    use preview_content::{FilePreview, UnsupportedPreviewKind};
    let kind = match preview_content::read_file_preview(path) {
        FilePreview::Text { content, truncated } => return (content, truncated),
        FilePreview::Image => "图片",
        FilePreview::Unsupported(UnsupportedPreviewKind::Archive) => "压缩包",
        FilePreview::Unsupported(UnsupportedPreviewKind::Pdf) => "PDF",
        FilePreview::Binary => "二进制文件",
    };
    let name = std::path::Path::new(path).file_name().and_then(|s| s.to_str()).unwrap_or(path);
    let info = if kind == "图片" {
        format!("文件: {}\n类型: 图片\n", name)
    } else {
        format!("文件: {}\n类型: {}\n不支持文本预览\n", name, kind)
    };
    (info, false)
}

pub fn preview_description(path: &str) -> Option<desktop_i18n::format::Message> {
    use desktop_i18n::format::Message;
    let name =
        std::path::Path::new(path).file_name().and_then(|name| name.to_str()).unwrap_or(path);
    let kind = preview_content::unsupported_preview_kind(path).map(|kind| match kind { preview_content::UnsupportedPreviewKind::Archive => "压缩包", preview_content::UnsupportedPreviewKind::Pdf => "PDF" })?;
    let kind =
        if kind == "PDF" { Message::literal(kind.into()) } else { Message::new(kind, vec![]) };
    Some(Message::nested(
        "文件: %s\n类型: %s\n不支持文本预览\n".into(),
        vec![Message::literal(name.into()), kind],
    ))
}

#[cfg(test)]
#[path = "tab_tests.rs"]
mod tests;

/// 标签导航开始时关闭编辑器弹层，包括打开失败或尚未加载的路径。
pub fn prepare_popover(popover: &mut Option<crate::message::editor::EditorPopover>, message: &PreviewMessage) {
    if matches!(message, PreviewMessage::Select(_) | PreviewMessage::Close(_) | PreviewMessage::Open(_) | PreviewMessage::OpenLoaded { .. } | PreviewMessage::TabMenuCloseLeft(_) | PreviewMessage::TabMenuCloseRight(_) | PreviewMessage::TabMenuCloseAll) {
        *popover = None;
    }
}
