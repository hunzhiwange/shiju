//! 条件分支节点视图模块，负责展示 if/else 节点的条件、分支和可编辑规则摘要。

use super::*;
use iced::widget::{column, row};

/// 构建 if else visual section 对应的界面元素。
///
/// 参数由当前工作流状态或编辑草稿提供；返回值是可直接嵌入 iced 视图树的元素。
pub(super) fn build_if_else_visual_section<'a>(
    i18n: crate::app::i18n::I18n,
    cases: &'a [super::state::WorkflowIfElseCaseDraft],
    validation: &'a super::state::WorkflowNodeEditorValidation,
) -> Element<'a, Message> {
    let mut case_list = column![].spacing(12).width(Length::Fill);

    if cases.is_empty() {
        case_list = case_list.push(
            text(i18n.__("当前没有条件分支。新增后会生成新的 source handle。"))
                .size(12)
                .style(settings_muted_text_style),
        );
    } else {
        for (index, case) in cases.iter().enumerate() {
            case_list = case_list.push(build_if_else_case_card(i18n, index, case, validation));
        }
    }

    column![
        row![
            Space::new().width(Length::Fill),
            button(text(i18n.__("新增分支")))
                .style(primary_action_btn_style)
                .padding([8, 12])
                .on_press(Message::WorkflowTool(WorkflowMessage::NodeEditorIfElseAddCase)),
        ]
        .align_y(Alignment::Center),
        case_list,
    ]
    .spacing(12)
    .into()
}

/// 构建 if else case card 对应的界面元素。
///
/// 参数由当前工作流状态或编辑草稿提供；返回值是可直接嵌入 iced 视图树的元素。
pub(super) fn build_if_else_case_card<'a>(
    i18n: crate::app::i18n::I18n,
    index: usize,
    case: &'a super::state::WorkflowIfElseCaseDraft,
    validation: &'a super::state::WorkflowNodeEditorValidation,
) -> Element<'a, Message> {
    let mut condition_list = column![].spacing(10);
    if case.conditions.is_empty() {
        condition_list = condition_list.push(
            text(i18n.__("当前分支还没有条件。至少新增一条条件后再保存。"))
                .size(12)
                .color(Color::from_rgba8(0x11, 0x18, 0x27, 0.62)),
        );
    } else {
        for (condition_index, condition) in case.conditions.iter().enumerate() {
            condition_list = condition_list.push(build_if_else_condition_card(
                i18n,
                index,
                condition_index,
                condition,
                validation,
            ));
        }
    }

    column![
        row![
            column![
                text(
                    i18n.format("分支 %s", &[(index + 1).to_string().into()])
                        .expect("valid workflow display template")
                )
                .size(14),
                text(format!("handle: {}", case.case_id)).size(11).style(settings_muted_text_style),
            ]
            .spacing(2),
            Space::new().width(Length::Fill),
            button(text(i18n.__("新增条件")))
                .style(primary_action_btn_style)
                .padding([6, 10])
                .on_press(Message::WorkflowTool(WorkflowMessage::NodeEditorIfElseAddCondition(
                    index
                ),)),
        ]
        .align_y(Alignment::Center),
        build_editor_field_validated(
            i18n.__("逻辑运算"),
            workflow_text_input(
                i18n.__("例如：and / or"),
                &case.logical_operator,
                move |value| {
                    Message::WorkflowTool(
                        WorkflowMessage::NodeEditorIfElseCaseLogicalOperatorChanged(index, value),
                    )
                }
            ),
            validation
                .localized_error_for(i18n, &format!("if_else.cases[{index}].logical_operator")),
        ),
        if let Some(error) =
            validation.localized_error_for(i18n, &format!("if_else.cases[{index}].conditions"))
        {
            build_inline_error(error)
        } else {
            container(Space::new().width(1).height(1)).into()
        },
        condition_list,
    ]
    .spacing(12)
    .into()
}

/// 构建 if else condition card 对应的界面元素。
///
/// 参数由当前工作流状态或编辑草稿提供；返回值是可直接嵌入 iced 视图树的元素。
pub(super) fn build_if_else_condition_card<'a>(
    i18n: crate::app::i18n::I18n,
    case_index: usize,
    condition_index: usize,
    condition: &'a super::state::WorkflowIfElseConditionDraft,
    validation: &'a super::state::WorkflowNodeEditorValidation,
) -> Element<'a, Message> {
    column![
        row![
            text(
                i18n.format("条件 %s", &[(condition_index + 1).to_string().into()])
                    .expect("valid workflow display template")
            )
            .size(13),
            Space::new().width(Length::Fill),
            button(text(i18n.__("删除条件")))
                .style(danger_action_btn_style)
                .padding([6, 10])
                .on_press(Message::WorkflowTool(WorkflowMessage::NodeEditorIfElseRemoveCondition(
                    case_index,
                    condition_index,
                ),)),
        ]
        .align_y(Alignment::Center),
        row![
            build_editor_field_validated(
                i18n.__("变量类型"),
                workflow_text_input(i18n.__("例如：string"), &condition.var_type, move |value| {
                    Message::WorkflowTool(WorkflowMessage::NodeEditorIfElseConditionVarTypeChanged(
                        case_index,
                        condition_index,
                        value,
                    ))
                }),
                validation.localized_error_for(
                    i18n,
                    &format!("if_else.cases[{case_index}].conditions[{condition_index}].var_type")
                ),
            ),
            build_editor_field_validated(
                i18n.__("比较符"),
                workflow_text_input(
                    i18n.__("例如：contains / is / not empty"),
                    &condition.comparison_operator,
                    move |value| {
                        Message::WorkflowTool(
                            WorkflowMessage::NodeEditorIfElseConditionOperatorChanged(
                                case_index,
                                condition_index,
                                value,
                            ),
                        )
                    },
                ),
                validation.localized_error_for(
                    i18n,
                    &format!("if_else.cases[{case_index}].conditions[{condition_index}].operator")
                ),
            ),
        ]
        .spacing(12),
        build_editor_field_validated(
            i18n.__("变量选择器"),
            workflow_text_input(
                i18n.__("例如：1711528917469.text"),
                &condition.variable_selector_input,
                move |value| {
                    Message::WorkflowTool(
                        WorkflowMessage::NodeEditorIfElseConditionSelectorChanged(
                            case_index,
                            condition_index,
                            value,
                        ),
                    )
                },
            ),
            validation.localized_error_for(
                i18n,
                &format!("if_else.cases[{case_index}].conditions[{condition_index}].selector")
            ),
        ),
        build_editor_field_validated(
            i18n.__("比较值"),
            workflow_text_input(
                i18n.__("例如：\"type\":\"orders\""),
                &condition.compare_value,
                move |value| {
                    Message::WorkflowTool(WorkflowMessage::NodeEditorIfElseConditionValueChanged(
                        case_index,
                        condition_index,
                        value,
                    ))
                },
            ),
            validation.localized_error_for(
                i18n,
                &format!("if_else.cases[{case_index}].conditions[{condition_index}].value")
            ),
        ),
    ]
    .spacing(10)
    .into()
}

#[cfg(test)]
#[path = "if_else_tests.rs"]
mod if_else_tests;
