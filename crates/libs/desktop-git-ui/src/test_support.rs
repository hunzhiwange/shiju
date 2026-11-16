//! Git 页面用例只拥有领域状态和显式视图输入。
use crate::{page::ViewContext, state::State};
pub struct Fixture {
    pub git: State,
    pub app_theme: iced::Theme,
    pub desktop_locale: config_ui::DesktopLocale,
    pub project_path: Option<String>,
    pub active_session_id: Option<String>,
    pub sessions: Vec<session_info::Info>,
    pub last_copied_code_hash: Option<u64>,
}
impl Default for Fixture {
    fn default() -> Self {
        Self {
            git: State::default(),
            app_theme: iced::Theme::Dark,
            desktop_locale: config_ui::DesktopLocale::SimplifiedChinese,
            project_path: None,
            active_session_id: None,
            sessions: Vec::new(),
            last_copied_code_hash: None,
        }
    }
}
impl Fixture {
    pub fn new() -> (Self, ()) {
        (Self::default(), ())
    }
    pub fn view(&self) -> ViewContext<'_> {
        let active = self
            .active_session_id
            .as_ref()
            .and_then(|id| self.sessions.iter().find(|s| &s.id == id))
            .map(|s| s.directory.as_str());
        #[cfg(not(target_arch = "wasm32"))]
        let repo_path = crate::git_panel::ops::git_repo_path(
            self.git.selected_git_worktree_directory.as_deref(),
            active,
            self.project_path.as_deref(),
        );
        #[cfg(target_arch = "wasm32")]
        let repo_path = None;
        let context_path = crate::message_git::shared::git_context_path(
            self.git.selected_git_worktree_directory.as_deref(),
            active,
            self.project_path.as_deref(),
        );
        ViewContext {
            git: &self.git,
            app_theme: &self.app_theme,
            desktop_locale: self.desktop_locale,
            project_path: &self.project_path,
            repo_path,
            context_path,
            code_mode: true,
            refresh_frame: 0,
            last_copied_code_hash: self.last_copied_code_hash,
        }
    }
    pub fn set_single_expanded_file(&mut self, path: String) {
        crate::message_git::diff::replace_expanded_files(
            &mut self.git.expanded_files,
            &mut self.git.expanded_files_set,
            vec![path],
        );
    }
}

#[derive(Debug, Clone)]
pub enum Output {
    Git(crate::message_git::messages::GitMessage),
    Effect(crate::message_git::controller::Effect),
}
pub fn update(
    app: &mut Fixture,
    message: crate::message_git::messages::GitMessage,
) -> iced::Task<Output> {
    let view = app.view();
    let ctx = crate::message_git::controller::Context {
        project_path: app.project_path.clone(),
        repo_path: view.repo_path,
        context_path: view.context_path,
        ..Default::default()
    };
    let mut effects = Vec::new();
    let task = crate::message_git::controller::update(&mut app.git, &ctx, &mut effects, message)
        .map(Output::Git);
    iced::Task::batch(
        std::iter::once(task)
            .chain(effects.into_iter().map(|effect| iced::Task::done(Output::Effect(effect)))),
    )
}
pub async fn collect_task_messages(task: iced::Task<Output>) -> Vec<Output> {
    use iced::futures::StreamExt;
    let Some(mut stream) = iced_runtime::task::into_stream(task.collect()) else {
        return Vec::new();
    };
    while let Some(action) = stream.next().await {
        if let iced_runtime::Action::Output(messages) = action {
            return messages;
        }
    }
    Vec::new()
}

impl Fixture {
    pub fn ensure_diff_file_expanded(&mut self, path: String) {
        crate::message_git::diff::ensure_diff_file_expanded(
            &mut self.git.expanded_files_set,
            &mut self.git.expanded_files,
            path,
        );
    }
    pub fn is_diff_file_expanded(&self, path: &str) -> bool {
        crate::message_git::diff::is_diff_file_expanded(&self.git.expanded_files_set, path)
    }
}
