//! 变量赋值、变量聚合与列表操作节点的可视化面板。

#[cfg(test)]
fn message(edit: WorkflowDataNodeEdit) -> Message {
    Message::WorkflowTool(crate::view::data_nodes::message(edit))
}

#[cfg(test)]
pub(super) fn build_variable_assigner_section<'a>(
    i18n: desktop_i18n::I18n,
    items: &'a [state::WorkflowAssignmentDraft],
) -> Element<'a, Message> {
    crate::view::data_nodes::build_variable_assigner_section(i18n, items).map(Message::WorkflowTool)
}

#[cfg(test)]
pub(super) fn build_variable_aggregator_section<'a>(
    i18n: desktop_i18n::I18n,
    variables: &'a [String],
    output_type: &'a str,
) -> Element<'a, Message> {
    crate::view::data_nodes::build_variable_aggregator_section(i18n, variables, output_type)
        .map(Message::WorkflowTool)
}

#[allow(clippy::too_many_arguments)]
#[cfg(test)]
pub(super) fn build_list_operator_section<'a>(
    i18n: desktop_i18n::I18n,
    variable: &'a str,
    conditions: &'a [state::WorkflowListConditionDraft],
    filter_enabled: bool,
    order_enabled: bool,
    order_key: &'a str,
    order_direction: &'a str,
    limit_enabled: bool,
    limit_size: &'a str,
) -> Element<'a, Message> {
    crate::view::data_nodes::build_list_operator_section(
        i18n,
        variable,
        conditions,
        filter_enabled,
        order_enabled,
        order_key,
        order_direction,
        limit_enabled,
        limit_size,
    )
    .map(Message::WorkflowTool)
}

#[cfg(all(test, not(target_arch = "wasm32")))]
#[path = "data_nodes_ui_refresh_37_tests.rs"]
mod ui_refresh_37_tests;

#[cfg(test)]
use super::*;
