//! Desktop mapping for workflow domain views.
use super::*;
use state::WorkflowRunPreview;
pub(super) fn build_saved_app_run_preview_layer(
    i18n: desktop_i18n::I18n,
    preview: &WorkflowRunPreview,
    error_copied: bool,
) -> Element<'_, Message> {
    crate::view::run_preview::build_saved_app_run_preview_layer(i18n, preview, error_copied)
        .map(Message::WorkflowTool)
}
pub(super) fn build_saved_app_run_detail_layer(
    i18n: desktop_i18n::I18n,
    detail: &state::WorkflowRunDetailDialog,
) -> Element<'_, Message> {
    crate::view::run_preview::build_saved_app_run_detail_layer(i18n, detail)
        .map(Message::WorkflowTool)
}
pub(super) fn workflow_run_output(i18n: desktop_i18n::I18n, output: &str) -> Element<'_, Message> {
    crate::view::run_preview::workflow_run_output(i18n, output).map(Message::WorkflowTool)
}
pub(super) fn workflow_run_error(
    i18n: desktop_i18n::I18n,
    preview: &WorkflowRunPreview,
) -> Element<'_, Message> {
    crate::view::run_preview::workflow_run_error(i18n, preview).map(Message::WorkflowTool)
}
#[allow(unused_imports)]
pub(super) use crate::view::run_preview::{
    RUN_PREVIEW_PENDING_RUN_ID_LABEL, WorkflowPreviewStatusKind, run_preview_run_id_label,
    saved_app_run_preview_action, saved_app_run_preview_close_message,
    workflow_run_node_data_preview_items, workflow_run_node_status_label, workflow_run_output_text,
    workflow_run_preview_chip_style, workflow_run_preview_error_style,
    workflow_run_preview_node_style, workflow_run_preview_output_empty_style,
    workflow_run_preview_output_style, workflow_run_status_label,
};
