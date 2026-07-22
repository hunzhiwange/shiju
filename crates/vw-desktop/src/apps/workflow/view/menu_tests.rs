//! 工作流菜单视图测试模块，覆盖菜单入口的元素构建与节点类型过滤分支。

use super::*;
use iced::Size;
use serde_yaml::Value;

use crate::apps::workflow::model::{
    WorkflowDocument, WorkflowHandleSide, WorkflowNodeIconDescriptor,
};

fn assert_element_size(
    element: &Element<'_, Message>,
    expected_width: Length,
    expected_height: Length,
) {
    let size = element.as_widget().size();

    assert_eq!(size.width, expected_width);
    assert_eq!(size.height, expected_height);
}

fn context_node_picker_item_count(element: &Element<'_, Message>) -> usize {
    let children = element.as_widget().children();

    assert_eq!(children.len(), 2, "picker should contain a heading and a scrollable list");
    children[1].children[0].children.len()
}

fn workflow_node(id: &str, block_type: &str) -> WorkflowNode {
    WorkflowNode {
        id: id.to_string(),
        block_type: block_type.to_string(),
        title: "节点".to_string(),
        description: String::new(),
        position: Point::new(0.0, 0.0),
        size: Size::new(180.0, 120.0),
        parent_id: None,
        selected: false,
        source_side: WorkflowHandleSide::Right,
        target_side: WorkflowHandleSide::Left,
        source_handles: Vec::new(),
        target_handles: Vec::new(),
        z_index: 0.0,
        raw_node: Value::Null,
    }
}

#[test]
fn workflow_node_icon_badge_builds_known_icon_element() {
    let icon = workflow_node_icon("start");

    let element = workflow_node_icon_badge(icon, workflow_node_accent_color("start"), 14.0);

    assert_element_size(&element, Length::Fixed(28.0), Length::Fixed(28.0));
}

#[test]
fn workflow_node_icon_badge_builds_fallback_dot_for_unknown_icon() {
    let icon = WorkflowNodeIconDescriptor { family: "missing-family", name: "missing-icon" };

    let element = workflow_node_icon_badge(icon, workflow_node_accent_color("unknown"), 18.0);

    assert_element_size(&element, Length::Fixed(32.0), Length::Fixed(32.0));
}

#[test]
fn workflow_node_icon_badge_supports_minimum_fallback_icon_size() {
    let icon = WorkflowNodeIconDescriptor { family: "missing-family", name: "missing-icon" };

    let element = workflow_node_icon_badge(icon, workflow_node_accent_color("unknown"), 1.0);

    assert_element_size(&element, Length::Fixed(15.0), Length::Fixed(15.0));
}

#[test]
fn context_menu_content_button_builds_action_message() {
    let content: Element<'static, Message> = text("创建").into();

    let element =
        context_menu_content_button(content, WorkflowMessage::CreateContextNode("llm".to_string()));

    assert_element_size(&element, Length::Fill, Length::Shrink);
    assert_eq!(element.as_widget().children().len(), 1);
}

#[test]
fn context_menu_button_accepts_owned_label() {
    let element = context_menu_button(
        String::from("删除"),
        WorkflowMessage::OpenCreateNodeEditor("code".to_string()),
    );

    assert_element_size(&element, Length::Fill, Length::Shrink);
    assert_eq!(element.as_widget().children().len(), 1);
}

#[test]
fn context_node_picker_button_builds_create_context_node_action() {
    let node_type = supported_node_types()
        .iter()
        .copied()
        .find(|node_type| node_type.block_type == "llm")
        .expect("llm node type should be registered");

    let element = context_node_picker_button(node_type);

    assert_element_size(&element, Length::Fill, Length::Shrink);
    assert_eq!(element.as_widget().children().len(), 1);
}

#[test]
fn context_node_picker_buttons_keep_uniform_width_for_all_registered_nodes() {
    for node_type in supported_node_types().iter().copied() {
        let element = context_node_picker_button(node_type);

        assert_element_size(&element, Length::Fill, Length::Shrink);
        assert_eq!(element.as_widget().children().len(), 1, "{}", node_type.block_type);
    }
}

#[test]
fn context_node_picker_menu_includes_start_when_state_has_no_start() {
    let state = WorkflowState::default();

    let node_types = available_node_types(&state, false);

    assert!(node_types.iter().any(|node_type| { node_type.block_type == "start" }));

    let element =
        build_context_node_picker_menu(&state, "添加节点", "选择要创建的节点类型。", false);

    assert_element_size(&element, Length::Fixed(208.0), Length::Shrink);
    assert_eq!(context_node_picker_item_count(&element), node_types.len());
}

#[test]
fn context_node_picker_menu_excludes_start_when_requested() {
    let state = WorkflowState::default();

    let node_types = available_node_types(&state, true);

    assert!(!node_types.iter().any(|node_type| { node_type.block_type == "start" }));

    let element =
        build_context_node_picker_menu(&state, "添加下游节点", "选择下游节点类型。", true);

    assert_element_size(&element, Length::Fixed(208.0), Length::Shrink);
    assert_eq!(context_node_picker_item_count(&element), node_types.len());
}

#[test]
fn context_node_picker_menu_excludes_start_when_document_already_has_start() {
    let state = WorkflowState {
        document: WorkflowDocument {
            nodes: vec![workflow_node("start_1", "start")],
            ..WorkflowDocument::default()
        },
        ..WorkflowState::default()
    };

    let node_types = available_node_types(&state, false);

    assert!(!node_types.iter().any(|node_type| { node_type.block_type == "start" }));

    let element = build_context_node_picker_menu(&state, "添加节点", "开始节点已存在。", false);

    assert_element_size(&element, Length::Fixed(208.0), Length::Shrink);
    assert_eq!(context_node_picker_item_count(&element), node_types.len());
}

#[test]
fn error_banner_builds_dismissible_error_element() {
    let element = error_banner("保存失败");

    assert_element_size(&element, Length::Fill, Length::Shrink);
    assert_eq!(element.as_widget().children().len(), 3);
}
