//! 领域更新的上下文投影与系统、跨域效果执行。
use super::{GitMessage, shared};
use crate::app::{App, Message};
use desktop_git_ui::message_git::controller::{Context, Effect};
use iced::Task;
pub(super) fn update(
    app: &mut App,
    message: GitMessage,
    handler: fn(
        &mut desktop_git_ui::state::State,
        &Context,
        &mut Vec<Effect>,
        GitMessage,
    ) -> Task<GitMessage>,
) -> Task<Message> {
    let ctx = Context {
        project_path: app.project_path.clone(),
        repo_path: crate::app::components::git_panel::git_repo_path_for_app(app),
        editor_theme: app.effective_editor_theme(),
        font_size: app.current_font_size,
        line_height: app.current_line_height,
        language: crate::app::i18n::editor::language(app.desktop_locale),
        context_path: shared::git_context_path_for_app(app),
        animate: !app.file_manager.file_manager_changes_refreshing
            && !app.file_manager.file_manager_file_tree_refreshing,
        manual_refresh: app.file_manager.file_manager_changes_refreshing,
    };
    let mut effects = Vec::new();
    let task = handler(&mut app.git, &ctx, &mut effects, message).map(Message::Git);
    let mut tasks = vec![task];
    for effect in effects {
        tasks.push(apply(app, effect));
    }
    Task::batch(tasks)
}
fn apply(app: &mut App, effect: Effect) -> Task<Message> {
    match effect {
        Effect::Commit(effect) => super::stage_commit::apply(app, effect),
        Effect::PersistHighlight(value) => {
            app.save_desktop_preference("show_git_diff_highlight", serde_json::Value::Bool(value))
        }
        Effect::DismissPreview => {
            shared::dismiss_preview_transient_ui(app);
            Task::none()
        }
        Effect::ClearPreview => {
            app.active_preview_path = None;
            Task::none()
        }
        Effect::ShowDiff { changes } => {
            app.show_diff = true;
            if changes {
                app.file_manager.file_manager_show_changes = true;
            }
            Task::none()
        }
        Effect::Fullscreen(active) => {
            app.fullscreen_layout_settling = true;
            if active {
                app.chat_panel_fullscreen = false;
                app.chat_panel_half_fullscreen = false;
                app.show_diff = true;
            }
            crate::app::message::after(
                std::time::Duration::from_millis(180),
                Message::View(crate::app::message::ViewMessage::FullscreenLayoutSettled),
            )
        }
        Effect::Preview(full_path) => {
            if let Some(error) = crate::app::preview::preview_open_error_message(&full_path) {
                let fallback = error
                    .render(crate::app::i18n::I18n::new(
                        config_ui::DesktopLocale::SimplifiedChinese,
                    ))
                    .expect("preview error template");
                let task = app.show_error_toast(fallback);
                if let Some(toast) = &mut app.active_toast {
                    toast.localized_message = Some(error);
                }
                return task;
            }
            Task::done(Message::Preview(crate::app::message::PreviewMessage::Open(full_path)))
        }
        Effect::CopyFile(path) => Task::done(Message::CopyFile(path)),
        Effect::CopyCode(text) => Task::done(Message::CopyCode(text)),
        Effect::AppendChat(text) => {
            Task::done(Message::Chat(crate::app::message::ChatMessage::AppendText(text)))
        }
        Effect::Discard(action) => {
            desktop_git_ui::message_git::diff::discard(
                app.project_path.as_deref(),
                app.terminal.shell,
                action,
            );
            Task::none()
        }
        Effect::LoadWorktrees(project_path) => Task::perform(
            async move {
                let result = shared::load_git_worktree_options(project_path.clone()).await;
                (project_path, result)
            },
            |(project_path, result)| {
                Message::Git(GitMessage::WorktreeOptionsReady { project_path, result })
            },
        ),
        Effect::LoadChangedFiles(path) => Task::perform(
            async move {
                let repo_path = path.clone();
                let files = crate::app::message::spawn_blocking_opt(move || {
                    Some(crate::app::components::git_panel::changed_files_in_repo(&repo_path))
                })
                .await
                .unwrap_or_default();
                (path, files)
            },
            |(repo_path, files)| {
                Message::Git(GitMessage::ChangedFilesReady { repo_path: Some(repo_path), files })
            },
        ),
        Effect::LoadDiffMetas(path) => Task::perform(
            async move {
                let repo_path = path.clone();
                let metas = crate::app::message::spawn_blocking_opt(move || {
                    Some(crate::app::components::git_panel::get_diff_file_metas_for_repo_path(
                        &repo_path,
                    ))
                })
                .await
                .unwrap_or_default();
                (path, metas)
            },
            |(repo_path, metas)| {
                Message::Git(GitMessage::DiffFileMetasReady { repo_path: Some(repo_path), metas })
            },
        ),
        Effect::LoadDiff { repo_path, file, meta } => Task::perform(
            async move {
                let path = repo_path.clone();
                let (old_content, new_content) =
                    crate::app::message::spawn_blocking_opt(move || {
                        Some(crate::app::components::git_panel::load_diff_content_for_repo_path(
                            &path, &meta,
                        ))
                    })
                    .await
                    .unwrap_or_default();
                (repo_path, file, old_content, new_content)
            },
            |(repo_path, file, old_content, new_content)| {
                Message::Git(GitMessage::DiffContentReady {
                    repo_path: Some(repo_path),
                    file,
                    old_content,
                    new_content,
                })
            },
        ),
        Effect::InputError { message, error } => {
            app.set_localized_input_error(
                crate::app::i18n::input::ErrorField::General,
                message,
                vec![error.into()],
            );
            Task::none()
        }
        Effect::ClearManualRefresh => {
            app.file_manager.file_manager_changes_refreshing = false;
            Task::none()
        }
        Effect::ChangedFilesApplied => {
            let was_manual_refresh = app.file_manager.file_manager_changes_refreshing;
            app.file_manager.file_manager_changes_refreshing = false;
            let save_task = if app.file_manager.file_manager_show_changes {
                for key in crate::app::components::file_tree::change_list_expand_keys(
                    app.git.git_changed_files.iter().map(String::as_str),
                ) {
                    app.ensure_file_tree_dir_expanded(key);
                }
                shared::persist_file_tree_expanded(app)
            } else {
                Task::none()
            };
            if was_manual_refresh {
                Task::batch([
                    save_task,
                    app.show_localized_toast(
                        crate::app::state::ToastKind::Success,
                        "Git 更改已刷新",
                        vec![],
                    ),
                ])
            } else {
                save_task
            }
        }
    }
}
