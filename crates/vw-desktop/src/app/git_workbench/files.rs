//! 将文件管理器事件和网关读取绑定到当前 Git 检出目录。
use std::collections::HashSet;
use std::path::Path;

use super::{Workbench, message, update_git};
use crate::app::message::{GitMessage, PreviewMessage};
use crate::app::{App, Message};
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
    SelectPreview(u64),
    ClosePreview(u64),
    OpenInEditor,
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
    pub(super) content: Option<Result<String, String>>,
    pub(super) truncated: bool,
    pub(super) location: Option<(usize, usize)>,
    source: PreviewSource,
    request: u64,
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
}

pub(super) const PREVIEW_SCROLL_ID: &str = "git-workbench-preview";

impl State {
    pub(super) fn active_preview(&self) -> Option<&Preview> {
        self.previews.iter().find(|tab| Some(tab.id) == self.active_preview_id)
    }

    fn select_preview(&mut self, id: u64) -> Task<Message> {
        if self.active_preview_id == Some(id) || !self.previews.iter().any(|tab| tab.id == id) {
            return Task::none();
        }
        self.active_preview_id = Some(id);
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
        Self::reset_preview_scroll()
    }

    fn reset_preview_scroll() -> Task<Message> {
        iced::widget::operation::snap_to(
            PREVIEW_SCROLL_ID,
            iced::widget::scrollable::RelativeOffset::START,
        )
    }

    fn finish_preview(
        &mut self,
        request: u64,
        result: Result<(String, bool), String>,
    ) -> Task<Message> {
        let Some(tab) = self.previews.iter_mut().find(|tab| tab.request == request) else {
            return Task::none();
        };
        tab.content = Some(result.map(|(content, truncated)| {
            tab.truncated = truncated;
            content
        }));
        if self.active_preview_id == Some(tab.id) {
            Self::reset_preview_scroll()
        } else {
            Task::none()
        }
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
        Event::Opened(request, result) => return state.files.finish_preview(request, result),
        Event::SelectPreview(id) => return state.files.select_preview(id),
        Event::ClosePreview(id) => return state.files.close_preview(id),
        Event::OpenInEditor => {
            if state.local()
                && state.client().is_ok()
                && let Some(root) = state.checkout_directory()
                && let Some(preview) = state.files.active_preview()
                && let Some(relative) = preview.path()
            {
                let path = absolute_path(root, relative);
                app.preview_state.pending_preview_goto = preview.location.map(|(line, column)| {
                    (path.clone(), line.saturating_sub(1), column.saturating_sub(1))
                });
                return crate::app::message::preview::update(app, PreviewMessage::Open(path));
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
        ProjectMessage::FileManagerShowChanges(changes) => select_mode(state, !changes),
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
                    state.show_files = true;
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
    if state.repo.is_none() {
        state.git.git_focused_file = Some(path.clone());
        return open_file(state, path, None);
    }
    state.show_files = false;
    state.git.git_repository.history = false;
    update_git(app, GitMessage::FocusFile(path))
}

pub(super) fn select_mode(state: &mut Workbench, show_files: bool) -> Task<Message> {
    state.show_files = show_files;
    state.files.show_all_files = show_files;
    state.files.find.active_find_results_tab_id = None;
    state.files.close_menu();
    if show_files && !state.files.loaded.contains("") {
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
    state.show_files = true;
    let client = state.client();
    state.files.preview_request = state.files.preview_request.wrapping_add(1);
    let request = state.files.preview_request;
    let existing = state.files.previews.iter_mut().find(|tab| tab.path() == Some(path.as_str()));
    let preview = Preview {
        id: existing.as_ref().map_or(request, |tab| tab.id),
        title: path.clone(),
        content: None,
        truncated: false,
        location,
        source: PreviewSource::File(path.clone()),
        request,
    };
    state.files.active_preview_id = Some(preview.id);
    if let Some(tab) = existing {
        *tab = preview;
    } else {
        state.files.previews.push(preview);
    }
    let epoch = state.epoch;
    Task::perform(
        async move {
            let file = client?.file_read_in_directory(Some(&directory), None, &path).await?;
            Ok(preview_content::format_text_preview_content(preview_from_line(
                &file.content,
                location.map_or(1, |(line, _)| line),
            )))
        },
        move |result| message(super::Event::Files(epoch, Event::Opened(request, result))),
    )
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
    let browse = select_mode(state, true);
    let mut first_unloaded_folder = None;
    for (index, _) in relative.match_indices('/') {
        let folder = &relative[..index];
        state.files.expanded.insert(folder.to_owned());
        if first_unloaded_folder.is_none() && !state.files.loaded.contains(folder) {
            first_unloaded_folder = Some(folder.to_owned());
        }
    }
    if let Some(root) = state.files.root.as_deref() {
        state.files.selection.select(root, path, false, false, &[]);
    }
    let expand = if state.files.loaded.contains("") {
        first_unloaded_folder.map(|folder| list_folder(state, folder)).unwrap_or_default()
    } else {
        Task::none()
    };
    Task::batch([browse, expand, open_file(state, relative, None)])
}

pub(crate) fn preview_snapshot(
    app: &mut App,
    key: String,
    title: String,
    content: String,
) -> Task<Message> {
    let state = &mut app.git_workbench;
    let browse = select_mode(state, true);
    let source = PreviewSource::Snapshot(key);
    if let Some(id) = state.files.previews.iter().find(|tab| tab.source == source).map(|tab| tab.id)
    {
        return Task::batch([browse, state.files.select_preview(id)]);
    }
    state.files.preview_request = state.files.preview_request.wrapping_add(1);
    let id = state.files.preview_request;
    let (content, truncated) = preview_content::format_text_preview_content(&content);
    state.files.previews.push(Preview {
        id,
        title,
        content: Some(Ok(content)),
        truncated,
        location: None,
        source,
        request: id,
    });
    Task::batch([browse, state.files.select_preview(id)])
}
