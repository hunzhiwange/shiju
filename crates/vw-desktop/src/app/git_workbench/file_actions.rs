//! File menu actions operate on the selected checkout and use the project file APIs.
use std::path::Path;

use desktop_file_tree::FileTreeAction;
use desktop_workspace::file_tree::{
    outcome, paste_targets, replace_path_prefix, top_level_paths, unique_name_for_target,
};
use desktop_workspace::interaction::{FileTreeClipboard, FileTreeClipboardMode};
use desktop_workspace::operations::{self, Operation};
use desktop_workspace::project_message::gateway::{
    GatewayFileContext, resolve_checkout_file_context,
};
use iced::Task;
use vw_gateway_client::GatewayClient;

use super::{
    Event, PreviewSource, Workbench, folder_mut, open_changed, open_file, relative_path,
    toggle_directory,
};
use crate::app::message::{ViewMessage, project};
use crate::app::state::ToastKind;
use crate::app::{App, Message};

#[derive(Clone)]
pub(crate) enum Mutation {
    Duplicate(Vec<String>),
    Paste(FileTreeClipboard, String),
    Rename { source: String, target: String },
    Delete(Vec<(String, bool)>),
}

#[derive(Clone)]
pub(crate) struct MutationResult {
    completed: Vec<String>,
    moves: Vec<(String, String)>,
    result: Result<(), String>,
}

pub(super) fn run(app: &mut App, root: &str, action: FileTreeAction) -> Task<Message> {
    let state = &mut app.git_workbench;
    let Some((path, source)) = state
        .files
        .interaction
        .file_tree_menu_path
        .clone()
        .zip(state.files.interaction.file_tree_menu_source.clone())
    else {
        return Task::none();
    };
    let visible =
        desktop_file_tree::selection::visible_paths(&state.files.tree, root, &state.files.expanded);
    let paths = state.files.interaction.menu_paths(Some(root), &visible);
    state.files.interaction.close_menu();

    let Some(relative) = relative_path(root, &path).filter(|path| !path.is_empty()) else {
        return Task::none();
    };
    if matches!(action, FileTreeAction::AddToChat | FileTreeAction::OpenInTerminal)
        && state.client().is_err()
    {
        return app.show_localized_toast(ToastKind::Error, "网关已切换，请刷新项目列表", vec![]);
    }
    let directory = matches!(source.as_str(), "tree-dir" | "changes-dir");
    match action {
        FileTreeAction::Open if source == "changes-file" => open_changed(app, relative),
        FileTreeAction::Open if source == "changes-dir" => {
            toggle_directory(state, format!("chg:{relative}"))
        }
        FileTreeAction::Open if directory => toggle_directory(state, relative),
        FileTreeAction::Open => open_file(state, relative, None),
        FileTreeAction::FindInFolder => {
            if directory {
                state.files.find.open_scope(path, crate::app::time::now_ms().to_string());
            }
            Task::none()
        }
        FileTreeAction::RevealInFinder => {
            let client = state.client();
            Task::perform(
                async move {
                    let result = match client {
                        Ok(client) => {
                            crate::app::config::reveal_gateway_path_locally(
                                client.endpoint(),
                                &path,
                            )
                            .await
                        }
                        Err(error) => Err(error),
                    };
                    ViewMessage::PathRevealFinished { path, result }
                },
                Message::View,
            )
        }
        FileTreeAction::OpenInTerminal => {
            if !state.local() {
                return app.show_localized_toast(
                    ToastKind::Error,
                    "远端终端只能通过网关 PTY 打开",
                    vec![],
                );
            }
            let Some(project) = state.project_directory().map(str::to_owned) else {
                return Task::none();
            };
            let dir = if directory {
                Path::new(&path)
            } else {
                Path::new(&path).parent().unwrap_or(Path::new(root))
            };
            let switch = show_project(app, project.clone());
            app.select_terminal_project(project);
            Task::batch([switch, project::file_tree::open_terminal_at(app, dir.to_path_buf())])
        }
        FileTreeAction::AddToChat => add_to_chat(app, root, paths),
        FileTreeAction::Cut | FileTreeAction::Copy => {
            let (mode, label) = if action == FileTreeAction::Cut {
                (FileTreeClipboardMode::Cut, "已剪切到文件树剪贴板")
            } else {
                (FileTreeClipboardMode::Copy, "已复制到文件树剪贴板")
            };
            state.files.interaction.set_clipboard(mode, paths);
            app.show_localized_toast(ToastKind::Success, label, vec![])
        }
        FileTreeAction::CopyPath => iced::clipboard::write(paths.join("\n")),
        FileTreeAction::CopyRelativePath => iced::clipboard::write(
            paths
                .iter()
                .filter_map(|path| relative_path(root, path))
                .collect::<Vec<_>>()
                .join("\n"),
        ),
        FileTreeAction::Duplicate => start(app, root, Mutation::Duplicate(top_level_paths(paths))),
        FileTreeAction::Paste => {
            let Some(clipboard) = state.files.interaction.file_tree_clipboard.clone() else {
                return Task::none();
            };
            let directory = if directory {
                path
            } else {
                Path::new(&path).parent().unwrap_or(Path::new(root)).to_string_lossy().into_owned()
            };
            if clipboard.mode == FileTreeClipboardMode::Cut
                && has_unsaved(state, root, &clipboard.src_paths)
            {
                return crate::app::git_workbench::unsaved_toast(app);
            }
            start(app, root, Mutation::Paste(clipboard, directory))
        }
        FileTreeAction::Rename => {
            if state.busy() {
                return Task::none();
            }
            state.files.interaction.begin_rename(path);
            let end = desktop_file_tree::inline_rename::selection_end(
                &state.files.interaction.file_tree_rename_value,
                directory,
            );
            let id = desktop_file_tree::inline_rename::INPUT_ID;
            iced::widget::operation::focus(id)
                .chain(iced::widget::operation::select_range(id, 0, end))
        }
        FileTreeAction::Delete => {
            if !state.busy() {
                state.files.interaction.file_tree_pending_delete =
                    Some((root.to_owned(), top_level_paths(paths)));
            }
            Task::none()
        }
    }
}

fn show_project(app: &mut App, path: String) -> Task<Message> {
    let (tab_id, _) = desktop_workspace::projects::project_tab(&path);
    if app.navigation_tabs.project_path.as_deref() == Some(path.as_str())
        && app.navigation_tabs.open_tabs.iter().any(|tab| tab.id == tab_id)
    {
        crate::app::message::view::update(app, ViewMessage::TabSelected(tab_id))
    } else {
        app.open_project_and_index(path)
    }
}

fn add_to_chat(app: &mut App, root: &str, paths: Vec<String>) -> Task<Message> {
    let Some(project_path) = app.git_workbench.project_directory().map(str::to_owned) else {
        return Task::none();
    };
    let switch = show_project(app, project_path.clone());
    let active_session = app.session_state.active_session_id.as_deref();
    let draft_directory = app
        .current_session_runtime_ref()
        .and_then(|runtime| runtime.draft.as_ref())
        .map(|draft| draft.directory.clone());
    let (draft, directory) = if let Some(id) = active_session {
        (Task::none(), app.known_session_directory(id))
    } else if draft_directory.is_some() || project_path == root {
        (Task::none(), draft_directory.or(Some(project_path)))
    } else {
        (project::open_session_draft(app, project_path, root.to_owned()), Some(root.to_owned()))
    };
    let references = paths
        .into_iter()
        .map(|path| {
            let path = directory
                .as_deref()
                .and_then(|directory| relative_path(directory, &path))
                .unwrap_or(path);
            format!(
                "{} ",
                desktop_editor::components::input_mention_highlighter::file_mention(&path)
            )
        })
        .collect::<String>();
    Task::batch([switch, draft, project::file_tree::append_file_references(app, references)])
}

pub(super) fn rename(app: &mut App, root: &str) -> Task<Message> {
    let state = &mut app.git_workbench;
    if state.busy() {
        return Task::none();
    }
    let rename = match state.files.interaction.prepare_rename(Some(root), &[]) {
        Ok(Some(rename)) => rename,
        Ok(None) => return Task::none(),
        Err(_) => return app.show_localized_toast(ToastKind::Error, "文件名不合法", vec![]),
    };
    if has_unsaved(state, root, std::slice::from_ref(&rename.source)) {
        return crate::app::git_workbench::unsaved_toast(app);
    }
    start(app, root, Mutation::Rename { source: rename.source, target: rename.target })
}

pub(super) fn confirm_delete(app: &mut App, confirm: bool) -> Task<Message> {
    let state = &mut app.git_workbench;
    let Some((root, paths)) = state.files.interaction.file_tree_pending_delete.take() else {
        return Task::none();
    };
    if !confirm || state.files.root.as_deref() != Some(&root) || state.busy() {
        return Task::none();
    }
    if has_unsaved(state, &root, &paths) {
        return crate::app::git_workbench::unsaved_toast(app);
    }
    let paths = paths
        .into_iter()
        .filter_map(|path| {
            let relative = relative_path(&root, &path).filter(|path| !path.is_empty())?;
            let directory = folder_mut(&mut state.files.tree, &relative).is_some()
                || state
                    .git
                    .git_changed_files
                    .iter()
                    .any(|file| file != &relative && Path::new(file).starts_with(&relative));
            Some((path, directory))
        })
        .collect();
    start(app, &root, Mutation::Delete(paths))
}

fn has_unsaved(state: &Workbench, root: &str, paths: &[String]) -> bool {
    state.files.previews.iter().any(|tab| {
        tab.has_unsaved()
            && tab.path().is_some_and(|path| {
                let absolute = Path::new(root).join(path);
                paths.iter().any(|path| absolute.starts_with(path))
            })
    })
}

impl Mutation {
    async fn plan(&self, client: &GatewayClient, root: &str) -> Result<Vec<Operation>, String> {
        Ok(match self {
            Self::Duplicate(paths) => paths
                .iter()
                .map(|path| Operation::Duplicate { root: root.to_owned(), path: path.clone() })
                .collect(),
            Self::Delete(paths) => paths
                .iter()
                .map(|(path, recursive)| Operation::Delete {
                    root: root.to_owned(),
                    path: path.clone(),
                    recursive: *recursive,
                })
                .collect(),
            Self::Paste(clipboard, directory) => {
                let siblings = list_siblings(client, root, Path::new(directory)).await?;
                paste_targets(&siblings, directory, &clipboard.src_paths)
                    .into_iter()
                    .map(|(source, target)| match clipboard.mode {
                        FileTreeClipboardMode::Copy => {
                            Operation::Copy { root: root.to_owned(), source, target }
                        }
                        FileTreeClipboardMode::Cut => {
                            Operation::Move { root: root.to_owned(), source, target }
                        }
                    })
                    .collect()
            }
            Self::Rename { source, target } => {
                let target = Path::new(target);
                let parent = target.parent().ok_or("无法定位目标目录")?;
                let name =
                    target.file_name().and_then(|name| name.to_str()).ok_or("文件名不合法")?;
                let siblings = list_siblings(client, root, parent).await?;
                let target = unique_name_for_target(parent, name, &siblings.into_iter().collect());
                vec![Operation::Move { root: root.to_owned(), source: source.clone(), target }]
            }
        })
    }
}

async fn list_siblings(
    client: &GatewayClient,
    root: &str,
    directory: &Path,
) -> Result<Vec<String>, String> {
    let relative = relative_path(root, &directory.to_string_lossy()).ok_or("无法定位目标目录")?;
    let entries = gateway_files::GatewayFilesClient::new(client.transport().clone())
        .directory_file_entries(root, &relative)
        .await?;
    Ok(entries
        .into_iter()
        .map(|entry| directory.join(entry.name).to_string_lossy().into_owned())
        .collect())
}

fn start(app: &mut App, root: &str, mutation: Mutation) -> Task<Message> {
    let state = &mut app.git_workbench;
    if state.busy() {
        return Task::none();
    }
    let Some(project) = state.project().cloned() else {
        return Task::none();
    };
    let client = state.client();
    state.files.interaction.file_tree_operation_in_progress = true;
    let root = root.to_owned();
    let epoch = state.epoch;
    Task::perform(
        async move {
            let result = async {
                let client = client?;
                let context = resolve_checkout_file_context(
                    &client,
                    project.id.into(),
                    &project.directory,
                    &root,
                )
                .await?;
                let operations = mutation.plan(&client, &root).await?;
                let mut moves = operations
                    .iter()
                    .filter_map(|operation| {
                        if let Operation::Move { source, target, .. } = operation {
                            Some((source.clone(), target.clone()))
                        } else {
                            None
                        }
                    })
                    .collect::<Vec<_>>();
                let result =
                    operations::run(operations, |operation| execute(&client, &context, operation))
                        .await;
                moves.retain(|(source, _)| result.completed.contains(source));
                Ok::<_, String>(MutationResult {
                    moves,
                    completed: result.completed,
                    result: outcome(result.errors),
                })
            }
            .await
            .unwrap_or_else(|error| MutationResult {
                completed: vec![],
                moves: vec![],
                result: Err(error),
            });
            (mutation, result)
        },
        move |(mutation, result)| {
            super::message(crate::app::git_workbench::Event::Files(
                epoch,
                Event::MutationReady(mutation, result),
            ))
        },
    )
}

async fn execute(
    client: &GatewayClient,
    context: &GatewayFileContext,
    operation: Operation,
) -> Result<(), String> {
    match operation {
        Operation::Duplicate { path, .. } => context.duplicate_path(client, &path).await,
        Operation::Copy { source, target, .. } => context.copy_path(client, &source, &target).await,
        Operation::Move { source, target, .. } => context.move_path(client, &source, &target).await,
        Operation::Delete { path, recursive, .. } => {
            context.delete_path(client, &path, recursive).await
        }
    }
}

pub(super) fn complete(app: &mut App, mutation: Mutation, result: MutationResult) -> Task<Message> {
    let state = &mut app.git_workbench;
    state.files.interaction.file_tree_operation_in_progress = false;
    let Some(root) = state.files.root.clone() else {
        return Task::none();
    };
    for (source, target) in &result.moves {
        relocate(state, &root, source, target);
    }
    let mut tasks = Vec::new();
    let (success, failure, arguments) = match mutation {
        Mutation::Duplicate(_) => ("已创建副本", "创建副本失败: %s", vec![]),
        Mutation::Paste(clipboard, _) => {
            state.files.interaction.complete_paste(clipboard, &result.completed);
            ("文件树操作已完成", "粘贴失败: %s", vec![])
        }
        Mutation::Rename { .. } => {
            let arguments = result
                .moves
                .first()
                .map(|(_, target)| desktop_workspace::file_tree::path_display_name(target).into())
                .into_iter()
                .collect();
            ("已重命名为 %s", "重命名失败: %s", arguments)
        }
        Mutation::Delete(_) => {
            state.files.interaction.complete_delete(true, &result.completed);
            let closed = state
                .files
                .previews
                .iter()
                .filter(|tab| {
                    !tab.has_unsaved()
                        && tab.path().is_some_and(|path| {
                            let absolute = Path::new(&root).join(path);
                            result.completed.iter().any(|deleted| absolute.starts_with(deleted))
                        })
                })
                .map(|tab| tab.id)
                .collect::<Vec<_>>();
            tasks.extend(closed.into_iter().map(|id| state.files.close_preview(id)));
            ("已删除", "删除失败: %s", vec![])
        }
    };
    if state.files.show_all_files
        || state.files.loaded.contains("")
        || state.files.loading.contains("")
    {
        tasks.push(super::refresh_tree(state));
    }
    tasks.push(super::refresh_changes_after_edit(state));
    tasks.push(match result.result {
        Ok(()) => app.show_localized_toast(ToastKind::Success, success, arguments),
        Err(error) => app.show_localized_toast(ToastKind::Error, failure, vec![error.into()]),
    });
    Task::batch(tasks)
}

fn relocate(state: &mut Workbench, root: &str, source: &str, target: &str) {
    let Some((old, new)) = relative_path(root, source).zip(relative_path(root, target)) else {
        return;
    };
    state.files.interaction.file_tree_selection.relocate(root, source, target);
    let changes_old = format!("chg:{old}");
    let changes_new = format!("chg:{new}");
    state.files.expanded = state
        .files
        .expanded
        .drain()
        .map(|path| {
            replace_path_prefix(&path, &old, &new)
                .or_else(|| replace_path_prefix(&path, &changes_old, &changes_new))
                .unwrap_or(path)
        })
        .collect();
    for tab in &mut state.files.previews {
        if let Some(path) = tab.path().and_then(|path| replace_path_prefix(path, &old, &new)) {
            tab.title = path.clone();
            tab.source = PreviewSource::File(path);
        }
    }
    if let Some(path) =
        state.git.git_focused_file.as_deref().and_then(|path| replace_path_prefix(path, &old, &new))
    {
        state.git.git_focused_file = Some(path);
    }
}
