//! 将文件管理器事件和网关读取绑定到当前 Git 检出目录。
use std::collections::HashSet;
use std::path::Path;

use super::{Workbench, message, update_git};
use crate::app::message::{GitMessage, PreviewMessage};
use crate::app::{App, Message};
use desktop_editor::components::editor::Editor;
use desktop_editor::components::preview_panel::view::{
    Message as EditorTool, options::EditorPopover,
};
use desktop_file_tree::model::{FileTreeNode, build_file_tree_model_from_dto};
use desktop_file_tree::selection::FileTreeSelection;
use desktop_file_tree::{FileTreeAction, ProjectMessage};
use desktop_workspace::project_message::find::FindState;
use iced::{Point, Task};
use vw_api_file::file::{FileNodeDto, FileNodeKind};

#[derive(Clone)]
pub(crate) enum Event {
    Tree(ProjectMessage),
    FolderReady(u64, String, Result<Vec<FileNodeDto>, String>),
    RemoteChangesReady(Result<Vec<String>, String>),
    Opened(u64, Result<(String, bool), String>),
    Editor(u64, iced_code_editor::Message),
    EditorTool(u64, EditorTool),
    Saved(u64, String, Result<(), String>),
    SelectPreview(u64),
    ClosePreview(u64),
    ConfirmClosePreview(bool),
    RefreshTick,
    SearchReady(Box<desktop_workspace::project_message::tasks::SearchCompletion>),
}

#[derive(Default)]
pub(super) struct State {
    pub(super) root: Option<String>,
    pub(super) tree: FileTreeNode,
    pub(super) expanded: HashSet<String>,
    pub(super) selection: FileTreeSelection,
    pub(super) find: FindState,
    pub(super) menu_path: Option<String>,
    pub(super) menu_source: Option<String>,
    pub(super) menu_anchor: Option<Point>,
    pub(super) previews: Vec<Preview>,
    active_preview_id: Option<u64>,
    pub(super) editor_popover: Option<EditorPopover>,
    pub(super) closing_preview_id: Option<u64>,
    pub(super) refresh_frame: usize,
    pub(super) changes_refreshing: bool,
    pub(super) files_refreshing: bool,
    pub(super) show_all_files: bool,
    loaded: HashSet<String>,
    loading: HashSet<String>,
    listing_generation: u64,
    preview_request: u64,
}

pub(super) struct Preview {
    pub(super) id: u64,
    pub(super) title: String,
    pub(super) content: Option<Result<PreviewContent, String>>,
    pub(super) location: Option<(usize, usize)>,
    source: PreviewSource,
    request: u64,
}

pub(super) enum PreviewContent {
    Editor(Document),
    ReadOnly { text: String, truncated: bool },
}

pub(super) struct Document {
    pub(super) editor: Editor,
    saved_content: String,
    pub(super) dirty: bool,
    saving: bool,
}

#[derive(PartialEq, Eq)]
enum PreviewSource {
    File(String),
    Snapshot(String),
}

impl Preview {
    pub(super) fn path(&self) -> Option<&str> {
        match &self.source {
            PreviewSource::File(path) => Some(path.as_str()),
            PreviewSource::Snapshot(_) => None,
        }
    }

    pub(super) fn editor(&self) -> Option<&Document> {
        match self.content.as_ref()? {
            Ok(PreviewContent::Editor(document)) => Some(document),
            _ => None,
        }
    }

    pub(super) fn document(&mut self) -> Option<&mut Document> {
        match self.content.as_mut()? {
            Ok(PreviewContent::Editor(document)) => Some(document),
            _ => None,
        }
    }

    fn has_unsaved(&self) -> bool {
        self.editor().is_some_and(|document| document.saving || document.dirty)
    }
}

pub(super) const PREVIEW_SCROLL_ID: &str = "git-workbench-preview";

impl State {
    pub(super) fn active_preview(&self) -> Option<&Preview> {
        self.previews.iter().find(|tab| Some(tab.id) == self.active_preview_id)
    }

    pub(super) fn has_unsaved(&self) -> bool {
        self.previews.iter().any(Preview::has_unsaved)
    }

    fn select_preview(&mut self, id: u64) -> Task<Message> {
        if self.active_preview_id == Some(id) || !self.previews.iter().any(|tab| tab.id == id) {
            return Task::none();
        }
        self.active_preview_id = Some(id);
        self.editor_popover = None;
        Self::reset_preview_scroll()
    }

    fn close_preview(&mut self, id: u64) -> Task<Message> {
        let Some(index) = self.previews.iter().position(|tab| tab.id == id) else {
            return Task::none();
        };
        self.previews.remove(index);
        if self.active_preview_id != Some(id) {
            return Task::none();
        }
        self.active_preview_id =
            self.previews.get(index).or_else(|| self.previews.last()).map(|tab| tab.id);
        self.editor_popover = None;
        Self::reset_preview_scroll()
    }

    fn reset_preview_scroll() -> Task<Message> {
        iced::widget::operation::snap_to(
            PREVIEW_SCROLL_ID,
            iced::widget::scrollable::RelativeOffset::START,
        )
    }

    pub(super) fn expand_changes(&mut self, paths: &[String]) {
        self.expanded.extend(desktop_file_tree::tree_list::change_list_expand_keys(
            paths.iter().map(String::as_str),
        ));
    }

    fn close_menu(&mut self) {
        self.menu_path = None;
        self.menu_source = None;
        self.menu_anchor = None;
    }
}

impl Workbench {
    pub(super) fn file_message(&self, event: Event) -> Message {
        message(super::Event::Files(self.epoch, event))
    }
}

pub(super) fn update(app: &mut App, event: Event) -> Task<Message> {
    let state = &mut app.git_workbench;
    match event {
        Event::Tree(event) => return tree_event(app, event),
        Event::FolderReady(generation, path, result)
            if generation == state.files.listing_generation =>
        {
            state.files.loading.remove(&path);
            let manual_refresh =
                path.is_empty() && std::mem::take(&mut state.files.files_refreshing);
            let entries = match result {
                Ok(entries) => entries,
                Err(error) => return app.show_error_toast(error),
            };
            let Some(root) = state.files.root.as_deref() else {
                return Task::none();
            };
            let folder = FileNodeDto {
                path: path.clone(),
                name: String::new(),
                kind: FileNodeKind::Directory,
                size_bytes: None,
                children: Some(entries),
            };
            let mut tree = build_file_tree_model_from_dto(root, &folder);
            if path.is_empty() {
                state.files.loaded.clear();
            }
            let (Some(node), Some(listing)) =
                (folder_mut(&mut state.files.tree, &path), folder_mut(&mut tree, &path))
            else {
                return Task::none();
            };
            *node = std::mem::take(listing);
            state.files.loaded.insert(path.clone());
            let open_folders = node
                .children
                .keys()
                .map(|name| if path.is_empty() { name.clone() } else { format!("{path}/{name}") })
                .filter(|child| state.files.expanded.contains(child))
                .collect::<Vec<_>>();
            let mut tasks: Vec<_> =
                open_folders.into_iter().map(|child| list_folder(state, child)).collect();
            if manual_refresh {
                tasks.push(app.show_localized_toast(
                    crate::app::state::ToastKind::Success,
                    "文件树已刷新",
                    vec![],
                ));
            }
            return Task::batch(tasks);
        }
        Event::RemoteChangesReady(result) => {
            let manual_refresh = std::mem::take(&mut state.files.changes_refreshing);
            state.git.git_changed_files_loading = false;
            match result {
                Ok(paths) => {
                    state.files.expand_changes(&paths);
                    state.git.git_changed_files = paths;
                    if manual_refresh {
                        return app.show_localized_toast(
                            crate::app::state::ToastKind::Success,
                            "Git 更改已刷新",
                            vec![],
                        );
                    }
                }
                Err(error) if manual_refresh => return app.show_error_toast(error),
                Err(_) => {}
            }
        }
        Event::Opened(request, result) => return finish_open(app, request, result),
        Event::Editor(id, iced_code_editor::Message::WriteRequested) => {
            return save(state, id);
        }
        Event::Editor(id, event) => return editor_update(state, id, event),
        Event::EditorTool(id, action) => return editor_tool(app, id, action),
        Event::Saved(id, content, result) => return finish_save(app, id, content, result),
        Event::SelectPreview(id) => return state.files.select_preview(id),
        Event::ClosePreview(id) => {
            if let Some(document) =
                state.files.previews.iter().find(|tab| tab.id == id).and_then(Preview::editor)
            {
                if document.saving {
                    return Task::none();
                }
                if document.dirty {
                    state.files.closing_preview_id = Some(id);
                    return Task::none();
                }
            }
            return state.files.close_preview(id);
        }
        Event::ConfirmClosePreview(discard) => {
            if let Some(id) = state.files.closing_preview_id.take()
                && discard
                && !state
                    .files
                    .previews
                    .iter()
                    .any(|tab| tab.id == id && tab.editor().is_some_and(|document| document.saving))
            {
                return state.files.close_preview(id);
            }
        }
        Event::RefreshTick => {
            if state.files.files_refreshing || state.files.changes_refreshing {
                state.files.refresh_frame = state.files.refresh_frame.wrapping_add(1);
                return schedule_tick(state);
            }
        }
        Event::SearchReady(completion) => super::find::complete(state, *completion),
        Event::FolderReady(..) => {}
    }
    Task::none()
}

fn tree_event(app: &mut App, event: ProjectMessage) -> Task<Message> {
    let state = &mut app.git_workbench;
    let Some(root) = state.files.root.clone() else {
        return Task::none();
    };
    if state.switching_branch {
        return Task::none();
    }
    match event {
        ProjectMessage::FileManagerShowChanges(changes) => select_tree_mode(state, !changes),
        ProjectMessage::FileManagerRefreshFileTree if !state.files.files_refreshing => {
            let tick = begin_refresh(state);
            state.files.files_refreshing = true;
            Task::batch([refresh_tree(state), tick])
        }
        ProjectMessage::FileManagerRefreshChanges
            if !state.files.changes_refreshing && !state.git.git_changed_files_loading =>
        {
            if state.local() && state.repo.is_none() {
                return Task::none();
            }
            let tick = begin_refresh(state);
            state.files.changes_refreshing = true;
            if state.local() {
                let source = state.source();
                Task::batch([
                    Task::done(source.message(GitMessage::RefreshChangedFiles)),
                    Task::done(source.message(GitMessage::RefreshDiffFileMetas)),
                    tick,
                ])
            } else {
                Task::batch([refresh_remote_changes(state), tick])
            }
        }
        ProjectMessage::FileManagerRefreshFileTree | ProjectMessage::FileManagerRefreshChanges => {
            Task::none()
        }
        ProjectMessage::ToggleTreeDir(path) => toggle_directory(state, path),
        ProjectMessage::FileTreeSelected { project_path, path, directory, modifiers } => {
            if root != project_path {
                return Task::none();
            }
            let command = modifiers.command();
            let shift = modifiers.shift();
            let visible = desktop_file_tree::selection::visible_paths(
                &state.files.tree,
                &root,
                &state.files.expanded,
            );
            state.files.selection.select(&root, path.clone(), command, shift, &visible);
            if command || shift {
                return Task::none();
            }
            let Some(relative) = relative_path(&root, &path) else {
                return Task::none();
            };
            if directory {
                toggle_directory(state, relative)
            } else {
                open_file(state, relative, None)
            }
        }
        ProjectMessage::OpenChangedFile(path) => open_changed(app, path),
        ProjectMessage::FileTreeRightClicked(path, source, x, y) => {
            if relative_path(&root, &path).is_some() {
                state.files.menu_path = Some(path);
                state.files.menu_source = Some(source);
                state.files.menu_anchor = Some(Point::new(x, y));
            }
            Task::none()
        }
        ProjectMessage::FileTreeMenuClose => {
            state.files.close_menu();
            Task::none()
        }
        ProjectMessage::FileTreeAction(action) => {
            let path = state.files.menu_path.clone();
            let source = state.files.menu_source.clone();
            state.files.close_menu();
            let Some((path, source)) = path.zip(source) else {
                return Task::none();
            };
            let Some(relative) = relative_path(&root, &path) else {
                return Task::none();
            };
            match action {
                FileTreeAction::Open if source == "changes-file" => open_changed(app, relative),
                FileTreeAction::Open if source == "changes-dir" => {
                    toggle_directory(state, format!("chg:{relative}"))
                }
                FileTreeAction::Open if source == "tree-dir" => toggle_directory(state, relative),
                FileTreeAction::Open => open_file(state, relative, None),
                FileTreeAction::FindInFolder
                    if matches!(source.as_str(), "changes-dir" | "tree-dir") =>
                {
                    state.files.find.open_scope(path, crate::app::time::now_ms().to_string());
                    Task::none()
                }
                FileTreeAction::CopyPath => iced::clipboard::write(path),
                FileTreeAction::CopyRelativePath => iced::clipboard::write(relative),
                _ => Task::none(),
            }
        }
        event => super::find::update(state, event),
    }
}

fn open_changed(app: &mut App, path: String) -> Task<Message> {
    let state = &mut app.git_workbench;
    if state.repo.is_some() {
        state.show_editor = false;
        state.files.editor_popover = None;
        state.git.git_repository.history = false;
        state.git.git_diff_scroll_offset_y = 0.0;
        return update_git(app, GitMessage::FocusFile(path));
    }
    open_file(state, path, None)
}

fn select_tree_mode(state: &mut Workbench, show_all_files: bool) -> Task<Message> {
    state.files.show_all_files = show_all_files;
    state.files.find.active_find_results_tab_id = None;
    state.files.close_menu();
    if show_all_files && !state.files.loaded.contains("") {
        list_folder(state, String::new())
    } else {
        Task::none()
    }
}

pub(super) fn refresh_tree(state: &mut Workbench) -> Task<Message> {
    state.files.listing_generation = state.files.listing_generation.wrapping_add(1);
    state.files.loading.clear();
    list_folder(state, String::new())
}

pub(super) fn refresh_remote_changes(state: &mut Workbench) -> Task<Message> {
    let Some(directory) = state.files.root.clone() else {
        return Task::none();
    };
    if state.git.git_changed_files_loading {
        return Task::none();
    }
    state.git.git_changed_files_loading = true;
    let client = state.client();
    let epoch = state.epoch;
    Task::perform(
        async move {
            #[derive(serde::Deserialize)]
            struct Entry {
                path: String,
            }
            let status = gateway_files::GatewayFilesClient::new(client?.transport().clone())
                .file_status_in_directory(Some(&directory))
                .await?;
            let entries: Vec<Entry> =
                serde_json::from_value(status).map_err(|error| error.to_string())?;
            Ok(entries.into_iter().map(|entry| entry.path).collect())
        },
        move |result| message(super::Event::Files(epoch, Event::RemoteChangesReady(result))),
    )
}

fn begin_refresh(state: &Workbench) -> Task<Message> {
    if state.files.files_refreshing || state.files.changes_refreshing {
        Task::none()
    } else {
        schedule_tick(state)
    }
}

fn schedule_tick(state: &Workbench) -> Task<Message> {
    crate::app::message::after(
        std::time::Duration::from_millis(100),
        state.file_message(Event::RefreshTick),
    )
}

fn toggle_directory(state: &mut Workbench, path: String) -> Task<Message> {
    if state.files.expanded.remove(&path) {
        return Task::none();
    }
    state.files.expanded.insert(path.clone());
    if path.starts_with("chg:") || state.files.loaded.contains(&path) {
        Task::none()
    } else {
        list_folder(state, path)
    }
}

fn list_folder(state: &mut Workbench, path: String) -> Task<Message> {
    let Some(directory) = state.files.root.clone() else {
        return Task::none();
    };
    if !state.files.loading.insert(path.clone()) {
        return Task::none();
    }
    let client = state.client();
    let epoch = state.epoch;
    let generation = state.files.listing_generation;
    Task::perform(
        async move {
            let result = async {
                gateway_files::GatewayFilesClient::new(client?.transport().clone())
                    .directory_file_entries(&directory, &path)
                    .await
            }
            .await;
            (path, result)
        },
        move |(path, result)| {
            message(super::Event::Files(epoch, Event::FolderReady(generation, path, result)))
        },
    )
}

fn folder_mut<'a>(mut tree: &'a mut FileTreeNode, path: &str) -> Option<&'a mut FileTreeNode> {
    for part in path.split('/').filter(|part| !part.is_empty()) {
        tree = tree.children.get_mut(part)?;
    }
    Some(tree)
}

pub(super) fn relative_path(root: &str, path: &str) -> Option<String> {
    let relative = Path::new(path).strip_prefix(root).ok()?;
    relative
        .components()
        .all(|part| matches!(part, std::path::Component::Normal(_)))
        .then(|| relative.to_string_lossy().replace('\\', "/"))
}

pub(super) fn absolute_path(root: &str, path: &str) -> String {
    Path::new(root).join(path).to_string_lossy().into_owned()
}

pub(super) fn open_file(
    state: &mut Workbench,
    path: String,
    location: Option<(usize, usize)>,
) -> Task<Message> {
    let Some(directory) = state.checkout_directory().map(str::to_owned) else {
        return Task::none();
    };
    if path.is_empty()
        || !std::path::Path::new(&path)
            .components()
            .all(|component| matches!(component, std::path::Component::Normal(_)))
        || state.switching_branch
    {
        return Task::none();
    }
    state.show_editor = true;
    state.files.editor_popover = None;
    state.git.git_focused_file = Some(path.clone());
    if let Some(root) = state.files.root.as_deref() {
        state.files.selection.select(root, absolute_path(root, &path), false, false, &[]);
    }
    let files = &mut state.files;
    let existing = files.previews.iter().position(|tab| tab.path() == Some(path.as_str()));
    if let Some(index) = existing {
        let tab = &mut files.previews[index];
        tab.location = location;
        if !matches!(&tab.content, Some(Err(_))) {
            let id = tab.id;
            let navigate = goto_location(tab, state.epoch);
            return Task::batch([files.select_preview(id), navigate]);
        }
    }
    files.preview_request = files.preview_request.wrapping_add(1);
    let request = files.preview_request;
    let preview = Preview {
        id: existing.map_or(request, |index| files.previews[index].id),
        title: path.clone(),
        content: None,
        location,
        source: PreviewSource::File(path.clone()),
        request,
    };
    files.active_preview_id = Some(preview.id);
    if let Some(index) = existing {
        files.previews[index] = preview;
    } else {
        files.previews.push(preview);
    }
    let client = state.client();
    let epoch = state.epoch;
    Task::perform(
        async move {
            let file = client?.file_read_in_directory(Some(&directory), None, &path).await?;
            if file.content.len() > preview_content::PREVIEW_MAX_BYTES {
                let excerpt =
                    preview_from_line(&file.content, location.map_or(1, |(line, _)| line));
                let (text, _) = preview_content::format_text_preview_content(excerpt);
                Ok((text, true))
            } else {
                Ok((file.content, false))
            }
        },
        move |result| message(super::Event::Files(epoch, Event::Opened(request, result))),
    )
}

fn finish_open(
    app: &mut App,
    request: u64,
    result: Result<(String, bool), String>,
) -> Task<Message> {
    let theme = app.effective_editor_theme();
    let epoch = app.git_workbench.epoch;
    let files = &mut app.git_workbench.files;
    let Some(tab) = files.previews.iter_mut().find(|tab| tab.request == request) else {
        return Task::none();
    };
    tab.content = Some(result.map(|(text, truncated)| {
        if truncated {
            return PreviewContent::ReadOnly { text, truncated };
        }
        let syntax = Path::new(&tab.title)
            .extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or("plaintext")
            .to_ascii_lowercase();
        let mut editor = Editor::new(&text, &syntax);
        editor.set_font(iced::Font::with_name("JetBrains Mono"));
        editor.set_theme(theme);
        editor.set_font_size(app.preview_state.current_font_size);
        editor.set_line_height(app.preview_state.current_line_height.clamp(10.0, 60.0));
        editor.set_ui_language(app.preview_state.current_language);
        editor.set_search_replace_enabled(true);
        PreviewContent::Editor(Document {
            editor,
            saved_content: text,
            dirty: false,
            saving: false,
        })
    }));
    let navigation = goto_location(tab, epoch);
    if files.active_preview_id == Some(tab.id) {
        Task::batch([State::reset_preview_scroll(), navigation])
    } else {
        navigation
    }
}

fn goto_location(tab: &mut Preview, epoch: u64) -> Task<Message> {
    let (Some((line, column)), id) = (tab.location, tab.id) else {
        return Task::none();
    };
    let Some(document) = tab.document() else {
        return Task::none();
    };
    document
        .editor
        .inner
        .update(&iced_code_editor::Message::GotoPosition(
            line.saturating_sub(1),
            column.saturating_sub(1),
        ))
        .map(move |event| message(super::Event::Files(epoch, Event::Editor(id, event))))
}

fn editor_update(
    state: &mut Workbench,
    id: u64,
    event: iced_code_editor::Message,
) -> Task<Message> {
    if state.switching_branch {
        return Task::none();
    }
    let response =
        state.files.previews.iter_mut().find(|tab| tab.id == id).and_then(Preview::document).map(
            |document| {
                let response = document.editor.inner.update(&event);
                if desktop_editor::preview::selection::editor_message_may_change_content(&event)
                    || matches!(
                        event,
                        iced_code_editor::Message::Backtab
                            | iced_code_editor::Message::DeleteWordBack
                            | iced_code_editor::Message::DeleteWordForward
                    )
                {
                    document.dirty = document.editor.content() != document.saved_content;
                }
                response
            },
        );
    let epoch = state.epoch;
    response
        .unwrap_or_else(Task::none)
        .map(move |event| message(super::Event::Files(epoch, Event::Editor(id, event))))
}

fn editor_tool(app: &mut App, id: u64, action: EditorTool) -> Task<Message> {
    use crate::app::message::editor::{self, EditorMessage};
    use iced_code_editor::Message as CodeEvent;

    let state = &mut app.git_workbench;
    if !state.show_editor || state.switching_branch || state.files.active_preview_id != Some(id) {
        return Task::none();
    }
    if matches!(action, EditorTool::OpenStandalone) {
        return open_in_standalone(app);
    }
    let files = &mut state.files;
    let document = files.previews.iter_mut().find(|tab| tab.id == id).and_then(Preview::document);
    if let EditorTool::TogglePopover(kind) = action {
        desktop_editor::message::editor::toggle_popover(
            &mut files.editor_popover,
            document.is_some(),
            kind,
        );
        return editor_update(state, id, CodeEvent::CloseCommandPalette);
    }
    if !matches!(action, EditorTool::SetOption(..) | EditorTool::SetIndentStyle(_)) {
        files.editor_popover = None;
    }
    let Some(document) = document else {
        return Task::none();
    };
    let event = match action {
        EditorTool::EditorEvent(event) => {
            document.editor.inner.request_focus();
            let _ = document.editor.inner.update(&CodeEvent::CanvasFocusGained);
            event
        }
        EditorTool::OpenSearch => CodeEvent::OpenSearch,
        EditorTool::OpenReplace => CodeEvent::OpenSearchReplace,
        EditorTool::SetOption(option, enabled) => {
            option.set(&mut document.editor.inner, enabled);
            return Task::none();
        }
        EditorTool::SetIndentStyle(style) => {
            document.editor.inner.set_indent_style(style);
            return Task::none();
        }
        EditorTool::ToggleSettings => return editor::update(app, EditorMessage::ToggleSettings),
        EditorTool::ClosePopover | EditorTool::TogglePopover(_) | EditorTool::OpenStandalone => {
            return Task::none();
        }
    };
    editor_update(state, id, event)
}

fn open_in_standalone(app: &mut App) -> Task<Message> {
    let state = &mut app.git_workbench;
    state.files.editor_popover = None;
    let Some(preview) = state.files.active_preview() else {
        return Task::none();
    };
    let Some((root, relative)) = state.checkout_directory().zip(preview.path()) else {
        return Task::none();
    };
    if !state.local() || state.client().is_err() || !matches!(&preview.content, Some(Ok(_))) {
        return Task::none();
    }
    if preview.has_unsaved() {
        return super::unsaved_toast(app);
    }
    let path = absolute_path(root, relative);
    let cursor = preview.editor().map(|document| document.editor.cursor_position()).or_else(|| {
        preview.location.map(|(line, column)| (line.saturating_sub(1), column.saturating_sub(1)))
    });
    let saved = preview.editor().map(|document| document.saved_content.clone());
    crate::app::message::preview::tab::open_standalone(app, path, cursor, saved.as_deref())
}

pub(crate) fn clipboard_shortcut(
    app: &mut App,
    kind: crate::app::ui_types::ClipboardShortcutKind,
) -> Task<Message> {
    use crate::app::ui_types::ClipboardShortcutKind;
    use iced_code_editor::Message as EditorMessage;

    let state = &app.git_workbench;
    let Some(tab) = state
        .files
        .active_preview()
        .filter(|tab| tab.editor().is_some_and(|document| document.editor.inner.is_focused()))
    else {
        return Task::none();
    };
    if !state.show_editor
        || state.files.closing_preview_id.is_some()
        || app.navigation_panels.show_system_settings
    {
        return Task::none();
    }
    let id = tab.id;
    let epoch = state.epoch;
    match kind {
        ClipboardShortcutKind::Copy => {
            editor_update(&mut app.git_workbench, id, EditorMessage::Copy)
        }
        ClipboardShortcutKind::Cut => editor_update(&mut app.git_workbench, id, EditorMessage::Cut),
        ClipboardShortcutKind::Paste => iced::clipboard::read().map(move |text| {
            text.map_or(Message::None, |text| {
                message(super::Event::Files(epoch, Event::Editor(id, EditorMessage::Paste(text))))
            })
        }),
    }
}

pub(crate) fn save_active(app: &mut App) -> Task<Message> {
    let state = &mut app.git_workbench;
    if state.show_editor
        && state.files.closing_preview_id.is_none()
        && let Some(id) = state.files.active_preview_id
    {
        return save(state, id);
    }
    Task::none()
}

fn save(state: &mut Workbench, id: u64) -> Task<Message> {
    if state.switching_branch {
        return Task::none();
    }
    let Some(directory) = state.files.root.clone() else {
        return Task::none();
    };
    let client = state.client();
    let Some(tab) = state.files.previews.iter_mut().find(|tab| tab.id == id) else {
        return Task::none();
    };
    let Some(path) = tab.path().map(str::to_owned) else {
        return Task::none();
    };
    let Some(document) = tab.document() else {
        return Task::none();
    };
    if document.saving || !document.dirty {
        return Task::none();
    }
    document.saving = true;
    let content = document.editor.content();
    let expected = document.saved_content.clone();
    let epoch = state.epoch;
    Task::perform(
        async move {
            let result = async {
                gateway_files::GatewayFilesClient::new(client?.transport().clone())
                    .replace_file_in_directory(directory, path, expected, content.clone())
                    .await
            }
            .await;
            (content, result)
        },
        move |(content, result)| {
            message(super::Event::Files(epoch, Event::Saved(id, content, result)))
        },
    )
}

fn finish_save(
    app: &mut App,
    id: u64,
    content: String,
    result: Result<(), String>,
) -> Task<Message> {
    let state = &mut app.git_workbench;
    let Some(document) =
        state.files.previews.iter_mut().find(|tab| tab.id == id).and_then(Preview::document)
    else {
        return Task::none();
    };
    document.saving = false;
    let refresh = match result {
        Ok(()) => {
            document.dirty = document.editor.content() != content;
            if !document.dirty {
                document.editor.inner.mark_saved();
            }
            document.saved_content = content;
            if state.local() {
                let source = state.source();
                Task::batch([
                    Task::done(source.message(GitMessage::RefreshChangedFiles)),
                    Task::done(source.message(GitMessage::RefreshDiffFileMetas)),
                ])
            } else {
                refresh_remote_changes(state)
            }
        }
        Err(error) => {
            return app.show_localized_toast(
                crate::app::state::ToastKind::Error,
                "保存失败: %s",
                vec![error.into()],
            );
        }
    };
    Task::batch([
        refresh,
        app.show_localized_toast(crate::app::state::ToastKind::Success, "已保存", vec![]),
    ])
}

fn preview_from_line(content: &str, line: usize) -> &str {
    if line <= 1 {
        return content;
    }
    content.match_indices('\n').nth(line - 2).map_or(content, |(index, _)| &content[index + 1..])
}

pub(crate) fn preview_diff_file(app: &mut App, path: String) -> Task<Message> {
    let state = &mut app.git_workbench;
    let Some(relative) = state.files.root.as_deref().and_then(|root| relative_path(root, &path))
    else {
        return Task::none();
    };
    let mut expand = Task::none();
    if state.files.show_all_files {
        let mut first_unloaded_folder = None;
        for (index, _) in relative.match_indices('/') {
            let folder = &relative[..index];
            state.files.expanded.insert(folder.to_owned());
            if first_unloaded_folder.is_none() && !state.files.loaded.contains(folder) {
                first_unloaded_folder = Some(folder.to_owned());
            }
        }
        if state.files.loaded.contains("")
            && let Some(folder) = first_unloaded_folder
        {
            expand = list_folder(state, folder);
        }
    }
    Task::batch([expand, open_file(state, relative, None)])
}

pub(crate) fn preview_snapshot(
    app: &mut App,
    key: String,
    title: String,
    content: String,
) -> Task<Message> {
    let state = &mut app.git_workbench;
    state.show_editor = true;
    let source = PreviewSource::Snapshot(key);
    if let Some(id) = state.files.previews.iter().find(|tab| tab.source == source).map(|tab| tab.id)
    {
        return state.files.select_preview(id);
    }
    state.files.preview_request = state.files.preview_request.wrapping_add(1);
    let id = state.files.preview_request;
    let (content, truncated) = preview_content::format_text_preview_content(&content);
    state.files.previews.push(Preview {
        id,
        title,
        content: Some(Ok(PreviewContent::ReadOnly { text: content, truncated })),
        location: None,
        source,
        request: id,
    });
    state.files.select_preview(id)
}
