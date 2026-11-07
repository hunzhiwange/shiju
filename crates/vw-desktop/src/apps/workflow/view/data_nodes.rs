//! 变量赋值、变量聚合与列表操作节点的可视化面板。

use super::*;
use crate::app::components::system_settings_common::{settings_group_card, settings_toggle};
use iced::widget::{button, column, row};

fn message(edit: WorkflowDataNodeEdit) -> Message {
    Message::WorkflowTool(WorkflowMessage::NodeEditorDataNodeEdit(edit))
}

pub(super) fn build_variable_assigner_section<'a>(
    i18n: crate::app::i18n::I18n,
    items: &'a [state::WorkflowAssignmentDraft],
) -> Element<'a, Message> {
    let mut content = column![text(i18n.__("赋值项")).size(13)].spacing(8);
    for (index, item) in items.iter().enumerate() {
        content = content.push(
            settings_group_card(
                column![
                    row![
                        workflow_text_input(
                            i18n.__("目标，例如 conversation.count"),
                            &item.variable_selector_input,
                            move |v| { message(WorkflowDataNodeEdit::AssignmentTarget(index, v)) }
                        ),
                        button(text(i18n.__("删除")))
                            .style(danger_action_btn_style)
                            .padding([8, 10])
                            .on_press(message(WorkflowDataNodeEdit::AssignmentRemove(index))),
                    ]
                    .spacing(8)
                    .align_y(Alignment::Center),
                    row![
                        workflow_text_input(
                            i18n.__("操作，例如 over-write / +="),
                            &item.operation,
                            move |v| message(WorkflowDataNodeEdit::AssignmentOperation(index, v))
                        ),
                        workflow_text_input("constant / variable", &item.input_type, move |v| {
                            message(WorkflowDataNodeEdit::AssignmentInputType(index, v))
                        }),
                        workflow_text_input(
                            i18n.__("值或 [node, variable]"),
                            &item.value_input,
                            move |v| { message(WorkflowDataNodeEdit::AssignmentValue(index, v)) }
                        ),
                    ]
                    .spacing(8),
                ]
                .spacing(8),
            )
            .padding(12),
        );
    }
    content
        .push(
            button(i18n.__("添加赋值"))
                .style(rounded_action_btn_style)
                .padding([8, 12])
                .on_press(message(WorkflowDataNodeEdit::AssignmentAdd)),
        )
        .into()
}

pub(super) fn build_variable_aggregator_section<'a>(
    i18n: crate::app::i18n::I18n,
    variables: &'a [String],
    output_type: &'a str,
) -> Element<'a, Message> {
    let mut content =
        column![workflow_text_input(i18n.__("输出类型"), output_type, |v| message(
            WorkflowDataNodeEdit::AggregatorOutputType(v)
        ))]
        .spacing(8);
    for (index, variable) in variables.iter().enumerate() {
        content = content.push(
            settings_group_card(
                row![
                    workflow_text_input(
                        i18n.__("候选变量，例如 start.value"),
                        variable,
                        move |v| { message(WorkflowDataNodeEdit::AggregatorVariable(index, v)) }
                    ),
                    button(text(i18n.__("删除")))
                        .style(danger_action_btn_style)
                        .padding([8, 10])
                        .on_press(message(WorkflowDataNodeEdit::AggregatorRemove(index))),
                ]
                .spacing(8)
                .align_y(Alignment::Center),
            )
            .padding(12),
        );
    }
    content
        .push(
            button(i18n.__("添加候选变量"))
                .style(rounded_action_btn_style)
                .padding([8, 12])
                .on_press(message(WorkflowDataNodeEdit::AggregatorAdd)),
        )
        .into()
}

#[allow(clippy::too_many_arguments)]
pub(super) fn build_list_operator_section<'a>(
    i18n: crate::app::i18n::I18n,
    variable: &'a str,
    conditions: &'a [state::WorkflowListConditionDraft],
    filter_enabled: bool,
    order_enabled: bool,
    order_key: &'a str,
    order_direction: &'a str,
    limit_enabled: bool,
    limit_size: &'a str,
) -> Element<'a, Message> {
    let mut content = column![
        workflow_text_input(i18n.__("输入数组，例如 start.items"), variable, |v| message(
            WorkflowDataNodeEdit::ListVariable(v)
        )),
        row![
            settings_toggle(filter_enabled)
                .on_toggle(|v| message(WorkflowDataNodeEdit::ListFilterEnabled(v))),
            text(i18n.__("启用过滤"))
        ]
        .spacing(8)
        .align_y(Alignment::Center),
    ]
    .spacing(8);
    for (index, condition) in conditions.iter().enumerate() {
        content = content.push(
            settings_group_card(
                row![
                    workflow_text_input(i18n.__("字段"), &condition.key, move |v| message(
                        WorkflowDataNodeEdit::ListConditionKey(index, v)
                    )),
                    workflow_text_input(
                        i18n.__("比较符"),
                        &condition.comparison_operator,
                        move |v| message(WorkflowDataNodeEdit::ListConditionOperator(index, v))
                    ),
                    workflow_text_input(i18n.__("值"), &condition.value_input, move |v| message(
                        WorkflowDataNodeEdit::ListConditionValue(index, v)
                    )),
                    button(text(i18n.__("删除")))
                        .style(danger_action_btn_style)
                        .padding([8, 10])
                        .on_press(message(WorkflowDataNodeEdit::ListConditionRemove(index))),
                ]
                .spacing(8)
                .align_y(Alignment::Center),
            )
            .padding(12),
        );
    }
    content
        .push(
            button(i18n.__("添加过滤条件"))
                .style(rounded_action_btn_style)
                .padding([8, 12])
                .on_press(message(WorkflowDataNodeEdit::ListConditionAdd)),
        )
        .push(
            row![
                settings_toggle(order_enabled)
                    .on_toggle(|v| message(WorkflowDataNodeEdit::ListOrderEnabled(v))),
                text(i18n.__("排序"))
            ]
            .spacing(8)
            .align_y(Alignment::Center),
        )
        .push(
            row![
                workflow_text_input(i18n.__("排序字段"), order_key, |v| message(
                    WorkflowDataNodeEdit::ListOrderKey(v)
                )),
                workflow_text_input("asc / desc", order_direction, |v| message(
                    WorkflowDataNodeEdit::ListOrderDirection(v)
                ))
            ]
            .spacing(8)
            .align_y(Alignment::Center),
        )
        .push(
            row![
                settings_toggle(limit_enabled)
                    .on_toggle(|v| message(WorkflowDataNodeEdit::ListLimitEnabled(v))),
                text(i18n.__("截取"))
            ]
            .spacing(8)
            .align_y(Alignment::Center),
        )
        .push(workflow_text_input(i18n.__("数量"), limit_size, |v| {
            message(WorkflowDataNodeEdit::ListLimitSize(v))
        }))
        .into()
}

#[cfg(all(test, not(target_arch = "wasm32")))]
#[path = "data_nodes_ui_refresh_37_tests.rs"]
mod ui_refresh_37_tests;
