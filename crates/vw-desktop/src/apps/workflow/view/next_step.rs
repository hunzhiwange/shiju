//! 工作流下一步视图模块，负责展示节点后续连接、分支出口和起始节点快捷创建按钮。

use super::*;
use iced::widget::{column, row};

#[cfg(test)]
#[path = "next_step_tests.rs"]
mod tests;

/// 构建 node next step existing item 对应的界面元素。
///
/// 参数由当前工作流状态或编辑草稿提供；返回值是可直接嵌入 iced 视图树的元素。
pub(super) fn build_node_next_step_existing_item<'a>(
    i18n: crate::app::i18n::I18n,
    edge: &'a WorkflowEdge,
    node: &'a WorkflowNode,
) -> Element<'a, Message> {
    let accent = workflow_node_accent_color(&node.block_type);
    let block_label = i18n.__(&pretty_block_type(&node.block_type)).to_string();
    let subtitle = if node.description.trim().is_empty() {
        block_label.clone()
    } else {
        format!("{} · {}", block_label, node.description.trim())
    };

    row![
        button(
            row![
                workflow_node_icon_badge(workflow_node_icon(&node.block_type), accent, 14.0),
                column![
                    text(&node.title).size(13),
                    text(subtitle).size(12).style(settings_muted_text_style),
                ]
                .spacing(4)
                .width(Length::Fill),
            ]
            .spacing(10)
            .align_y(Alignment::Center),
        )
        .style(next_step_jump_card_style)
        .padding([10, 12])
        .width(Length::Fill)
        .on_press(Message::WorkflowTool(WorkflowMessage::JumpToNode(node.id.clone()))),
        row![
            button(text(i18n.__("更改")).size(12))
                .style(rounded_action_btn_style)
                .padding([8, 10])
                .on_press(Message::WorkflowTool(WorkflowMessage::OpenEditNodeEditor(Some(
                    node.id.clone(),
                )))),
            button(text(i18n.__("断开")).size(12))
                .style(rounded_action_btn_style)
                .padding([8, 10])
                .on_press(Message::WorkflowTool(WorkflowMessage::DeleteEdgeById(edge.id.clone(),))),
            button(text(i18n.__("删除")).size(12))
                .style(danger_action_btn_style)
                .padding([8, 10])
                .on_press(Message::WorkflowTool(WorkflowMessage::DeleteNodeById(node.id.clone(),))),
        ]
        .spacing(6)
        .align_y(Alignment::Center),
    ]
    .spacing(10)
    .align_y(Alignment::Center)
    .into()
}

/// 构建 node next step section 对应的界面元素。
///
/// 参数由当前工作流状态或编辑草稿提供；返回值是可直接嵌入 iced 视图树的元素。
pub(super) fn build_node_next_step_section<'a>(
    i18n: crate::app::i18n::I18n,
    state: &'a WorkflowState,
    editor: &'a super::state::WorkflowNodeEditorDraft,
) -> Element<'a, Message> {
    let existing_connections: Element<'a, Message> = match &editor.mode {
        super::state::WorkflowNodeEditorMode::Edit(node_id) => {
            build_node_next_step_connection_list(
                i18n,
                state,
                node_id,
                None,
                i18n.__("当前还没有已连接的下游节点。"),
            )
        }
        super::state::WorkflowNodeEditorMode::Create => container(
            text(i18n.__("保存当前节点后，才能查看和管理它的下游节点。"))
                .size(12)
                .style(settings_muted_text_style),
        )
        .padding([12, 14])
        .style(value_card_style)
        .into(),
    };

    let add_content: Element<'a, Message> = match &editor.mode {
        super::state::WorkflowNodeEditorMode::Edit(node_id) => {
            let mut content =
                column![build_start_next_step_button_group(state, node_id.clone(), None)]
                    .spacing(12);

            let has_fail_branch = node_next_step_supports_fail_branch(state, editor, node_id);

            if has_fail_branch {
                content = content.push(build_node_next_step_branch_section(
                    i18n.__("异常时"),
                    i18n.__("代码节点进入异常分支后，会从这里继续流转。"),
                    build_node_next_step_connection_list(
                        i18n,
                        state,
                        node_id,
                        Some("fail-branch"),
                        i18n.__("当前还没有异常分支节点。"),
                    ),
                    build_start_next_step_button_group(state, node_id.clone(), Some("fail-branch")),
                ));
            }

            content.into()
        }
        super::state::WorkflowNodeEditorMode::Create => container(
            text(i18n.__("保存节点后，可在这里继续添加下游节点。"))
                .size(12)
                .style(settings_muted_text_style),
        )
        .padding([12, 14])
        .style(value_card_style)
        .into(),
    };

    column![existing_connections, add_content].spacing(12).into()
}

fn build_node_next_step_connection_list<'a>(
    i18n: crate::app::i18n::I18n,
    state: &'a WorkflowState,
    node_id: &str,
    source_handle_id: Option<&str>,
    empty_text: &'a str,
) -> Element<'a, Message> {
    let mut list = column![].spacing(8);
    let mut has_items = false;

    for edge in state.document.edges.iter().filter(|edge| {
        edge.source == node_id
            && match source_handle_id {
                Some(handle_id) => edge.source_handle.as_deref() == Some(handle_id),
                None => edge.source_handle.as_deref() != Some("fail-branch"),
            }
    }) {
        if let Some(target_node) = state.document.node(&edge.target) {
            has_items = true;
            list = list.push(build_node_next_step_existing_item(i18n, edge, target_node));
        }
    }

    if has_items {
        list.into()
    } else {
        container(text(empty_text).size(12).style(settings_muted_text_style))
            .padding([12, 14])
            .style(value_card_style)
            .into()
    }
}

fn build_node_next_step_branch_section<'a>(
    title: &'a str,
    description: &'a str,
    existing_content: Element<'a, Message>,
    add_content: Element<'a, Message>,
) -> Element<'a, Message> {
    container(
        column![
            text(title).size(13),
            text(description).size(12).style(settings_muted_text_style),
            existing_content,
            add_content,
        ]
        .spacing(10),
    )
    .padding([12, 14])
    .style(value_card_style)
    .into()
}

fn build_start_next_step_button_group<'a>(
    state: &'a WorkflowState,
    source_node_id: String,
    source_handle_id: Option<&'a str>,
) -> Element<'a, Message> {
    let mut buttons = row![].spacing(8);

    for node_type in available_node_types(state, true) {
        buttons = buttons.push(build_start_next_step_button(
            source_node_id.clone(),
            source_handle_id,
            node_type,
        ));
    }

    container(buttons.wrap()).width(Length::Fill).into()
}

fn node_next_step_supports_fail_branch(
    state: &WorkflowState,
    editor: &super::state::WorkflowNodeEditorDraft,
    node_id: &str,
) -> bool {
    if state
        .document
        .node(node_id)
        .is_some_and(|node| node.source_handles.iter().any(|handle| handle.id == "fail-branch"))
    {
        return true;
    }

    matches!(
        (
            &editor.mode,
            editor.block_type.as_str(),
            editor.visual_draft.as_ref(),
        ),
        (
            super::state::WorkflowNodeEditorMode::Edit(editor_node_id),
            "code",
            Some(WorkflowNodeVisualDraft::Code { error_strategy, .. }),
        ) if editor_node_id == node_id && error_strategy == "fail-branch"
    )
}

/// 构建 start next step button 对应的界面元素。
///
/// 参数由当前工作流状态或编辑草稿提供；返回值是可直接嵌入 iced 视图树的元素。
pub(super) fn build_start_next_step_button(
    source_node_id: String,
    source_handle_id: Option<&str>,
    node_type: WorkflowNodeTypeDescriptor,
) -> Element<'static, Message> {
    let accent = workflow_node_accent_color(node_type.block_type);
    let message = match source_handle_id {
        Some(handle_id) => WorkflowMessage::InsertDownstreamNodeFromHandle(
            source_node_id,
            handle_id.to_string(),
            node_type.block_type.to_string(),
        ),
        None => {
            WorkflowMessage::InsertDownstreamNode(source_node_id, node_type.block_type.to_string())
        }
    };

    button(
        row![
            workflow_node_icon_badge(node_type.icon, accent, 14.0),
            text(node_type.label).size(12),
        ]
        .spacing(8)
        .align_y(Alignment::Center),
    )
    .style(rounded_action_btn_style)
    .padding([8, 12])
    .on_press(Message::WorkflowTool(message))
    .into()
}
