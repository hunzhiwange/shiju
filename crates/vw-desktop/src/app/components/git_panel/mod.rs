//! Git 页面与跨域消息的桌面装配。
use crate::app::{App, Message, message};
pub(crate) use desktop_git_ui::git_panel::left_panel::traverse_commit_focus;
pub use desktop_git_ui::git_panel::ops::{
    DiffFileMeta, checkout_branch, current_branch, list_branches,
};
pub(crate) use desktop_git_ui::git_panel::repository_ops;
use desktop_git_ui::page;
fn input(app: &App) -> page::ViewContext<'_> {
    page::ViewContext {
        git: &app.git,
        app_theme: &app.appearance.app_theme,
        desktop_locale: app.general_settings.desktop_locale,
        project_path: &app.navigation_tabs.project_path,
        repo_path: git_repo_path_for_app(app),
        context_path: crate::app::message::git::git_context_path_for_app(app),
        code_mode: app.navigation_mode.workspace_mode == crate::app::state::WorkspaceMode::Code,
        refresh_frame: app.file_manager.file_manager_refresh_frame,
        last_copied_code_hash: app.clipboard_state.last_copied_code_hash,
    }
}
fn map(message: page::Message) -> Message {
    match message {
        page::Message::Git(value) => Message::Git(value),
        page::Message::None => Message::None,
        page::Message::OpenTerminal => Message::View(message::ViewMessage::OpenTerminalPressed),
        page::Message::ToggleSummary => Message::View(message::ViewMessage::ToggleGitDiffSummary),
        page::Message::CopyCode(text) => Message::CopyCode(text),
        page::Message::AppendChat(text) => Message::Chat(message::ChatMessage::AppendText(text)),
        page::Message::OpenUrl(url) => Message::View(message::ViewMessage::OpenUrlExternal(url)),
    }
}
pub fn view(app: &App) -> iced::Element<'_, Message> {
    page::view(&input(app)).map(map)
}
pub(crate) fn diff_comment_modal(app: &App) -> Option<iced::Element<'_, Message>> {
    page::diff_comment_modal(&input(app)).map(|element| element.map(map))
}
pub(crate) fn discard_file_modal(app: &App) -> Option<iced::Element<'_, Message>> {
    page::discard_file_modal(&input(app)).map(|element| element.map(map))
}
pub fn embedded_custom_text_diff_view(
    app: &App,
    title: String,
    file: Option<String>,
    before: String,
    after: String,
    close_message: Option<Message>,
) -> iced::Element<'_, Message> {
    let close = close_message.as_ref().map(|_| page::Message::None);
    page::embedded_custom_text_diff_view(&input(app), title, file, before, after, close).map(
        move |event| match event {
            page::Message::None => close_message.clone().unwrap_or(Message::None),
            event => map(event),
        },
    )
}
pub fn changed_files_in_repo(path: &str) -> Vec<String> {
    desktop_git_ui::git_panel::ops::get_changed_file_paths(path)
}
pub fn changed_files(app: &App) -> Vec<String> {
    app.navigation_tabs.project_path.as_deref().map(changed_files_in_repo).unwrap_or_default()
}
pub(crate) fn git_repo_path_for_app(app: &App) -> Option<String> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let active = app.session_state.active_session_id
            .as_ref()
            .and_then(|id| app.session_state.sessions.iter().find(|s| &s.id == id))
            .map(|s| s.directory.as_str());
        desktop_git_ui::git_panel::ops::git_repo_path(
            app.git.selected_git_worktree_directory.as_deref(),
            active,
            app.navigation_tabs.project_path.as_deref(),
        )
    }
    #[cfg(target_arch = "wasm32")]
    {
        None
    }
}
#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn get_diff_file_metas_for_repo_path(path: &str) -> Vec<DiffFileMeta> {
    desktop_git_ui::git_panel::ops::get_diff_file_metas_for_repo_path(path)
}
pub(crate) fn load_diff_content_for_repo_path(path: &str, meta: &DiffFileMeta) -> (String, String) {
    desktop_git_ui::git_panel::ops::load_diff_content_for_repo_path(path, meta)
}
#[cfg(target_arch = "wasm32")]
pub fn open_terminal(_path: &str) -> Result<(), String> {
    Err("Not supported on Web".to_owned())
}
#[cfg(not(target_arch = "wasm32"))]
pub fn open_terminal(path: &str) -> Result<(), std::io::Error> {
    crate::app::terminal::open_local_directory(path)
}

#[cfg(target_arch = "wasm32")]
pub(crate) fn get_diff_file_metas_for_repo_path(_path: &str) -> Vec<DiffFileMeta> {
    Vec::new()
}
#[cfg(test)]
mod repository_tests;
