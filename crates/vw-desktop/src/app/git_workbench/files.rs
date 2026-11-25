//! 通过网关按层浏览和预览所选检出目录。
use super::{Workbench, message};
use crate::app::{App, Message};
use iced::Task;
use vw_api_file::file::FileNodeDto;

#[derive(Clone)]
pub(crate) enum Event {
    Folder(String),
    FolderReady(u64, u64, Result<Vec<FileNodeDto>, String>),
    Open(String),
    Opened(u64, u64, Result<(String, bool), String>),
    OpenInEditor,
}

#[derive(Default)]
pub(super) struct State {
    pub(super) directory: String,
    pub(super) listing: Option<Result<Vec<FileNodeDto>, String>>,
    pub(super) preview: Option<Preview>,
    listing_request: u64,
    preview_request: u64,
}

pub(super) struct Preview {
    pub(super) title: String,
    pub(super) path: Option<String>,
    pub(super) content: Option<Result<String, String>>,
    pub(super) truncated: bool,
}

impl State {
    pub(super) fn needs_listing(&self) -> bool {
        self.listing_request == 0
    }
}

pub(super) fn update(app: &mut App, event: Event) -> Task<Message> {
    use Event::*;
    let state = &mut app.git_workbench;
    match event {
        Folder(path) => list_folder(state, path),
        FolderReady(epoch, request, result)
            if epoch == state.epoch && request == state.files.listing_request =>
        {
            state.files.listing = Some(result);
            Task::none()
        }
        Open(path) => open_file(state, path),
        Opened(epoch, request, result)
            if epoch == state.epoch && request == state.files.preview_request =>
        {
            if let Some(preview) = &mut state.files.preview {
                preview.content = Some(result.map(|(content, truncated)| {
                    preview.truncated = truncated;
                    content
                }));
            }
            Task::none()
        }
        OpenInEditor => {
            if state.local()
                && state.client().is_ok()
                && let Some(root) = state.checkout_directory()
                && let Some(relative) =
                    state.files.preview.as_ref().and_then(|file| file.path.as_ref())
            {
                let path = std::path::Path::new(root).join(relative);
                return Task::done(Message::Preview(crate::app::message::PreviewMessage::Open(
                    path.to_string_lossy().into_owned(),
                )));
            }
            Task::none()
        }
        FolderReady(..) | Opened(..) => Task::none(),
    }
}

pub(super) fn list_folder(state: &mut Workbench, path: String) -> Task<Message> {
    let Some(directory) = state.checkout_directory().map(str::to_owned) else {
        return Task::none();
    };
    if state.switching_branch {
        return Task::none();
    }
    let client = state.client();
    state.files.directory = path.clone();
    state.files.listing = None;
    state.files.listing_request = state.files.listing_request.wrapping_add(1);
    let request = state.files.listing_request;
    let epoch = state.epoch;
    Task::perform(
        async move {
            let client = client?;
            gateway_files::GatewayFilesClient::new(client.transport().clone())
                .directory_file_entries(&directory, &path)
                .await
        },
        move |result| message(super::Event::Files(Event::FolderReady(epoch, request, result))),
    )
}

fn open_file(state: &mut Workbench, path: String) -> Task<Message> {
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
    let client = state.client();
    state.files.preview = Some(Preview {
        title: path.clone(),
        path: Some(path.clone()),
        content: None,
        truncated: false,
    });
    state.files.preview_request = state.files.preview_request.wrapping_add(1);
    let request = state.files.preview_request;
    let epoch = state.epoch;
    Task::perform(
        async move {
            let file = client?.file_read_in_directory(Some(&directory), None, &path).await?;
            Ok(limit_preview(file.content))
        },
        move |result| message(super::Event::Files(Event::Opened(epoch, request, result))),
    )
}

fn limit_preview(mut content: String) -> (String, bool) {
    const MAX_BYTES: usize = 128 * 1024;
    const MAX_LINES: usize = 2_000;

    let limit = content.len().min(MAX_BYTES);
    let mut end = content.as_bytes()[..limit]
        .iter()
        .enumerate()
        .filter(|(_, byte)| **byte == b'\n')
        .nth(MAX_LINES - 1)
        .map_or(limit, |(index, _)| index + 1);
    while !content.is_char_boundary(end) {
        end -= 1;
    }
    let truncated = end < content.len();
    content.truncate(end);
    (content, truncated)
}

pub(crate) fn preview_diff_file(app: &mut App, path: String) -> Task<Message> {
    let state = &mut app.git_workbench;
    let Some(relative) = state.checkout_directory().and_then(|root| {
        desktop_workspace::project_message::paths::relative_to_project(root, &path)
    }) else {
        return Task::none();
    };
    state.show_files = true;
    let parent = relative.rsplit_once('/').map_or("", |(parent, _)| parent).to_owned();
    Task::batch([list_folder(state, parent), open_file(state, relative)])
}

pub(crate) fn preview_snapshot(app: &mut App, title: String, content: String) -> Task<Message> {
    let state = &mut app.git_workbench;
    state.show_files = true;
    state.files.preview_request = state.files.preview_request.wrapping_add(1);
    let (content, truncated) = limit_preview(content);
    state.files.preview =
        Some(Preview { title, path: None, content: Some(Ok(content)), truncated });
    if state.files.needs_listing() {
        return list_folder(state, String::new());
    }
    Task::none()
}
