//! Add menu projection. The page host maps actions to discovery and scoped modals.
use crate::{import_modal, update::Input};
use desktop_ui_controls::system_settings_common::*;
use iced::{
    Element,
    widget::{column, text},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Discover,
    Archive,
    Import,
}
impl std::fmt::Display for Action {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Discover => "在发现里浏览",
            Self::Archive => "从 ZIP 安装",
            Self::Import => "导入本机已有",
        })
    }
}
/// No count until a successful scan in exactly the current scope.
pub fn import_count(state: &import_modal::State, input: &Input) -> Option<usize> {
    if state.busy
        || state.context != input.context
        || state.catalog != input.catalog
        || state.host != input.host
        || state.project != input.project
    {
        return None;
    }
    state
        .report
        .as_ref()
        .filter(|report| {
            report.scope.target_host_id == input.host
                && report.scope.project_id.as_ref()
                    == vw_api_transport::local_settings_project(input.project.as_deref())
        })
        .map(|report| report.skills.iter().filter(|row| import_modal::eligible(row)).count())
}
/// Native select anchors its menu, captures outside clicks and restores keyboard focus.
pub fn view<'a, M: Clone + 'a>(
    input: &Input,
    count: Option<usize>,
    message: impl Fn(Action) -> M + 'a,
) -> Element<'a, M> {
    let source =
        if input.loopback { "本机".to_owned() } else { format!("远端主机 {}", input.host) };
    let label = match count {
        Some(count) => format!("来源：{source} · 可导入 {count}"),
        None => format!("来源：{source}"),
    };
    column![
        settings_select(
            ["在发现里浏览".to_owned(), "从 ZIP 安装".to_owned(), format!("导入{source}已有")],
            None,
            move |label: String| message(if label == "在发现里浏览" {
                Action::Discover
            } else if label == "从 ZIP 安装" {
                Action::Archive
            } else {
                Action::Import
            })
        )
        .placeholder("添加 Skill"),
        text(label)
    ]
    .spacing(6)
    .into()
}
