//! Workflow 应用列表视图模块，负责展示本地数据库中的应用入口。

#[cfg(test)]
pub(super) fn build_saved_apps_view(
    i18n: desktop_i18n::I18n,
    state: &WorkflowState,
) -> Element<'_, Message> {
    crate::view::app_list::build_saved_apps_view(i18n, state).map(Message::WorkflowTool)
}

#[cfg(test)]
pub(super) fn build_saved_app_delete_confirm_dialog(
    i18n: desktop_i18n::I18n,
    state: &WorkflowState,
) -> Option<Element<'_, Message>> {
    crate::view::app_list::build_saved_app_delete_confirm_dialog(i18n, state)
        .map(|element| element.map(Message::WorkflowTool))
}

#[cfg(test)]
pub(super) fn build_saved_app_run_prompt_dialog(
    i18n: desktop_i18n::I18n,
    state: &WorkflowState,
) -> Option<Element<'_, Message>> {
    crate::view::app_list::build_saved_app_run_prompt_dialog(i18n, state)
        .map(|element| element.map(Message::WorkflowTool))
}

#[cfg(test)]
use super::*;
