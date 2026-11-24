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
    i18n: desktop_i18n::I18n,
    input: &Input,
    count: Option<usize>,
    message: impl Fn(Action) -> M + 'a,
) -> Element<'a, M> {
    let source = if input.loopback {
        i18n.__("本机").to_owned()
    } else {
        desktop_i18n::__!(i18n, "远端主机 %s", (input.host).to_string())
            .expect("valid extension display template")
    };
    let label = match count {
        Some(count) => desktop_i18n::__!(
            i18n,
            "来源：%s · 可导入 %s",
            (source).to_string(),
            (count).to_string()
        )
        .expect("valid extension display template"),
        None => desktop_i18n::__!(i18n, "来源：%s", (source).to_string())
            .expect("valid extension display template"),
    };
    column![
        settings_select(
            [
                i18n.__("在发现里浏览").to_owned(),
                i18n.__("从 ZIP 安装").to_owned(),
                desktop_i18n::__!(i18n, "导入%s已有", (source).to_string())
                    .expect("valid extension display template")
            ],
            None,
            move |label: String| message(if label == i18n.__("在发现里浏览") {
                Action::Discover
            } else if label == i18n.__("从 ZIP 安装") {
                Action::Archive
            } else {
                Action::Import
            })
        )
        .placeholder(i18n.__("添加 Skill")),
        text(label)
    ]
    .spacing(6)
    .into()
}
