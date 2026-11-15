//! 桌面空状态的只读数据装配。
use chrono::Timelike;
use iced::Element;
use crate::app::{App, Message};
use super::utils::{current_branch_label, current_project_path_label, relative_modified_label};

pub fn session_loading_placeholder(app: &App) -> Element<'_, Message> {
    desktop_chat_view::empty::session_loading_placeholder(
        crate::app::i18n::I18n::new(app.desktop_locale).__("正在恢复会话"),
    )
}
pub fn empty_session_placeholder(app: &App) -> Element<'_, Message> {
    let i18n = crate::app::i18n::I18n::new(app.desktop_locale);
    #[cfg(not(target_arch = "wasm32"))]
    let name = app.account.active_account().and_then(|a| a.profile()).map(|p| p.name());
    #[cfg(target_arch = "wasm32")]
    let name: Option<&str> = None;
    desktop_chat_view::empty::empty_session_placeholder(desktop_chat_view::empty::EmptySessionInput {
        greeting: name.map(|name| i18n.format(if chrono::Local::now().hour() < 12 { "上午好，%s" } else { "下午好，%s" }, &[name.into()]).expect("valid greeting template")),
        title: i18n.__(desktop_chat_view::empty::empty_session_title(matches!(app.workspace_mode, crate::app::state::WorkspaceMode::Work))).to_string(),
        path: current_project_path_label(app),
        git: app.project_is_git_repo.then(|| (
            i18n.format("分支 %s", &[current_branch_label(app).into()]).expect("valid branch template"),
            relative_modified_label(app),
        )),
    })
}
#[cfg(test)]
#[path = "empty_tests.rs"]
mod empty_tests;
