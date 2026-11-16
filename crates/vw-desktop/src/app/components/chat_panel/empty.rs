//! 桌面空状态的只读数据装配。
use super::utils::{current_branch_label, current_project_path_label, relative_modified_label};
use crate::app::{App, Message};
use chrono::Timelike;
use iced::Element;

pub fn session_loading_placeholder(app: &App) -> Element<'_, Message> {
    desktop_chat_view::empty::session_loading_placeholder(
        crate::app::i18n::I18n::new(app.general_settings.desktop_locale).__("正在恢复会话"),
    )
}
pub fn empty_session_placeholder(app: &App) -> Element<'_, Message> {
    let i18n = crate::app::i18n::I18n::new(app.general_settings.desktop_locale);
    #[cfg(not(target_arch = "wasm32"))]
    let name = app.account.active_account().and_then(|a| a.profile()).map(|p| p.name());
    #[cfg(target_arch = "wasm32")]
    let name: Option<&str> = None;
    desktop_chat_view::empty::empty_session_placeholder(
        desktop_chat_view::empty::EmptySessionInput::from_context(
            i18n,
            name,
            chrono::Local::now().hour(),
            matches!(app.navigation_mode.workspace_mode, crate::app::state::WorkspaceMode::Work),
            current_project_path_label(app),
            app.workspace_state.project_is_git_repo
                .then(|| (current_branch_label(app), relative_modified_label(app))),
        ),
    )
}
