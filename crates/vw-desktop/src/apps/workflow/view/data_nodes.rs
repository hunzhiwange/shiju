//! 变量赋值、变量聚合与列表操作节点的可视化面板。

use super::*;
use iced::widget::{button, column, row};

fn message(edit: WorkflowDataNodeEdit) -> Message {
    Message::WorkflowTool(WorkflowMessage::NodeEditorDataNodeEdit(edit))
}

pub(super) fn build_variable_assigner_section<'a>(
    items: &'a [state::WorkflowAssignmentDraft],
) -> Element<'a, Message> {
    let mut content = column![text("赋值项").size(13)].spacing(8);
    for (index, item) in items.iter().enumerate() {
        content = content.push(
            column![
                row![
                    workflow_text_input(
                        "目标，例如 conversation.count",
                        &item.variable_selector_input,
                        move |v| { message(WorkflowDataNodeEdit::AssignmentTarget(index, v)) }
                    ),
                    button("删除").on_press(message(WorkflowDataNodeEdit::AssignmentRemove(index))),
                ]
                .spacing(8),
                row![
                    workflow_text_input(
                        "操作，例如 over-write / +=",
                        &item.operation,
                        move |v| message(WorkflowDataNodeEdit::AssignmentOperation(index, v))
                    ),
                    workflow_text_input("constant / variable", &item.input_type, move |v| message(
                        WorkflowDataNodeEdit::AssignmentInputType(index, v)
                    )),
                    workflow_text_input("值或 [node, variable]", &item.value_input, move |v| {
                        message(WorkflowDataNodeEdit::AssignmentValue(index, v))
                    }),
                ]
                .spacing(8),
            ]
            .spacing(6),
        );
    }
    content.push(button("添加赋值").on_press(message(WorkflowDataNodeEdit::AssignmentAdd))).into()
}

pub(super) fn build_variable_aggregator_section<'a>(
    variables: &'a [String],
    output_type: &'a str,
) -> Element<'a, Message> {
    let mut content = column![workflow_text_input("输出类型", output_type, |v| message(
        WorkflowDataNodeEdit::AggregatorOutputType(v)
    ))]
    .spacing(8);
    for (index, variable) in variables.iter().enumerate() {
        content = content.push(
            row![
                workflow_text_input("候选变量，例如 start.value", variable, move |v| {
                    message(WorkflowDataNodeEdit::AggregatorVariable(index, v))
                }),
                button("删除").on_press(message(WorkflowDataNodeEdit::AggregatorRemove(index))),
            ]
            .spacing(8),
        );
    }
    content
        .push(button("添加候选变量").on_press(message(WorkflowDataNodeEdit::AggregatorAdd)))
        .into()
}

#[allow(clippy::too_many_arguments)]
pub(super) fn build_list_operator_section<'a>(
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
        workflow_text_input("输入数组，例如 start.items", variable, |v| message(
            WorkflowDataNodeEdit::ListVariable(v)
        )),
        row![
            toggler(filter_enabled)
                .on_toggle(|v| message(WorkflowDataNodeEdit::ListFilterEnabled(v))),
            text("启用过滤")
        ]
        .spacing(8),
    ]
    .spacing(8);
    for (index, condition) in conditions.iter().enumerate() {
        content = content.push(
            row![
                workflow_text_input("字段", &condition.key, move |v| message(
                    WorkflowDataNodeEdit::ListConditionKey(index, v)
                )),
                workflow_text_input("比较符", &condition.comparison_operator, move |v| message(
                    WorkflowDataNodeEdit::ListConditionOperator(index, v)
                )),
                workflow_text_input("值", &condition.value_input, move |v| message(
                    WorkflowDataNodeEdit::ListConditionValue(index, v)
                )),
                button("删除").on_press(message(WorkflowDataNodeEdit::ListConditionRemove(index))),
            ]
            .spacing(6),
        );
    }
    content
        .push(button("添加过滤条件").on_press(message(WorkflowDataNodeEdit::ListConditionAdd)))
        .push(
            row![
                toggler(order_enabled)
                    .on_toggle(|v| message(WorkflowDataNodeEdit::ListOrderEnabled(v))),
                text("排序")
            ]
            .spacing(8),
        )
        .push(
            row![
                workflow_text_input("排序字段", order_key, |v| message(
                    WorkflowDataNodeEdit::ListOrderKey(v)
                )),
                workflow_text_input("asc / desc", order_direction, |v| message(
                    WorkflowDataNodeEdit::ListOrderDirection(v)
                ))
            ]
            .spacing(8),
        )
        .push(
            row![
                toggler(limit_enabled)
                    .on_toggle(|v| message(WorkflowDataNodeEdit::ListLimitEnabled(v))),
                text("截取")
            ]
            .spacing(8),
        )
        .push(workflow_text_input("数量", limit_size, |v| {
            message(WorkflowDataNodeEdit::ListLimitSize(v))
        }))
        .into()
}
