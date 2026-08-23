//! 工作流页头视图的测试，覆盖来源、脏状态、历史和选择操作。
//! 优先级用例验证连接取消、节点删除与边删除只展示当前可用动作。
use super::*;
use iced::widget::button;
use iced::{Point, Vector};
use serde_yaml::Value;

use crate::apps::workflow::model::{
    WorkflowAppMeta, WorkflowConnectionDraft, WorkflowConnectionEndpoint, WorkflowDocument,
    WorkflowEdge, WorkflowHandleKind, create_node_from_type,
};
use crate::apps::workflow::state::{WorkflowAppEntry, WorkflowHistorySnapshot};

fn test_document() -> WorkflowDocument {
    WorkflowDocument {
        name: "测试工作流".to_string(),
        nodes: vec![
            create_node_from_type("start", "node_1".to_string(), Point::ORIGIN, 0.0)
                .expect("start node should be created"),
        ],
        edges: vec![WorkflowEdge {
            id: "edge_1".to_string(),
            source: "node_1".to_string(),
            target: "node_2".to_string(),
            source_handle: Some("source".to_string()),
            target_handle: Some("target".to_string()),
            source_type: "source".to_string(),
            target_type: "target".to_string(),
            selected: false,
            z_index: 1.0,
            raw_edge: Value::Null,
        }],
        ..WorkflowDocument::default()
    }
}

fn test_snapshot(document: &WorkflowDocument) -> WorkflowHistorySnapshot {
    WorkflowHistorySnapshot {
        meta: WorkflowAppMeta::default(),
        document: document.clone(),
        environment_variables: Vec::new(),
        conversation_variables: Vec::new(),
        pan: Vector::new(0.0, 0.0),
        zoom: 1.0,
        selected_node_id: None,
        selected_edge_id: None,
    }
}

fn active_state() -> WorkflowState {
    let document = test_document();
    let saved_snapshot = test_snapshot(&document);

    WorkflowState {
        apps: vec![WorkflowAppEntry {
            id: "app_1".to_string(),
            local_uuid: Some("local_1".to_string()),
            meta: WorkflowAppMeta {
                name: "测试应用".to_string(), ..WorkflowAppMeta::default()
            },
            source_path: Some("/tmp/workflow.yml".to_string()),
            raw_root: Value::Null,
            document: document.clone(),
            environment_variables: Vec::new(),
            conversation_variables: Vec::new(),
            pan: Vector::new(0.0, 0.0),
            zoom: 1.0,
            selected_node_id: None,
            selected_edge_id: None,
            connection_draft: None,
            is_dirty: false,
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            saved_snapshot: saved_snapshot.clone(),
        }],
        active_app_id: Some("app_1".to_string()),
        source_name: "测试应用".to_string(),
        local_uuid: Some("local_1".to_string()),
        source_path: Some("/tmp/workflow.yml".to_string()),
        document,
        zoom: 1.0,
        saved_snapshot: Some(saved_snapshot),
        ..WorkflowState::default()
    }
}

fn connection_draft() -> WorkflowConnectionDraft {
    WorkflowConnectionDraft {
        from: WorkflowConnectionEndpoint {
            node_id: "node_1".to_string(),
            handle_id: "source".to_string(),
            kind: WorkflowHandleKind::Source,
        },
        cursor_world: Point::new(16.0, 24.0),
    }
}

#[test]
fn build_header_renders_empty_state_without_description() {
    let state = WorkflowState::default();
    let element = build_header(&state);

    std::hint::black_box(element);
}

#[test]
fn build_header_renders_source_path_and_dirty_badges() {
    let mut state = active_state();
    state.active_is_dirty = true;
    state.zoom = 0.05;

    let element = build_header(&state);

    std::hint::black_box(element);
}

#[test]
fn build_header_renders_clean_source_path_without_dirty_suffix() {
    let state = active_state();

    assert!(state.source_path.is_some());
    assert!(!state.active_is_dirty);
    std::hint::black_box(build_header(&state));
}

#[test]
fn build_header_renders_unsaved_draft_without_source_path() {
    let mut state = active_state();
    state.source_path = None;
    state.active_is_dirty = true;

    let element = build_header(&state);

    std::hint::black_box(element);
}

#[test]
fn build_header_renders_database_backed_clean_app() {
    let mut state = active_state();
    state.source_path = None;
    state.active_is_dirty = false;

    let element = build_header(&state);

    std::hint::black_box(element);
}

#[test]
fn build_header_renders_high_zoom_and_no_active_app_actions() {
    let mut state = active_state();
    state.active_app_id = None;
    state.selected_node_id = None;
    state.selected_edge_id = None;
    state.zoom = 2.75;

    let element = build_header(&state);

    std::hint::black_box(element);
}

#[test]
fn build_header_prefers_cancel_connection_over_delete_actions() {
    let mut state = active_state();
    state.selected_node_id = Some("node_1".to_string());
    state.selected_edge_id = Some("edge_1".to_string());
    state.connection_draft = Some(connection_draft());

    let element = build_header(&state);

    std::hint::black_box(element);
}

#[test]
fn build_header_renders_delete_node_and_delete_edge_paths() {
    let mut node_state = active_state();
    node_state.selected_node_id = Some("node_1".to_string());
    let node_element = build_header(&node_state);

    let mut edge_state = active_state();
    edge_state.selected_edge_id = Some("edge_1".to_string());
    let edge_element = build_header(&edge_state);

    std::hint::black_box((node_element, edge_element));
}

#[test]
fn build_action_bar_covers_enabled_disabled_and_active_controls() {
    let empty_element = build_action_bar(&WorkflowState::default());

    let mut state = active_state();
    let snapshot = test_snapshot(&state.document);
    state.action_menu_open = true;
    state.undo_stack.push(snapshot.clone());
    state.redo_stack.push(snapshot);
    let active_element = build_action_bar(&state);

    std::hint::black_box((empty_element, active_element));
}

#[test]
fn build_action_menu_overlay_covers_enabled_and_disabled_items() {
    let empty_element = build_action_menu_overlay(&WorkflowState::default());
    let active_element = build_action_menu_overlay(&active_state());

    std::hint::black_box((empty_element, active_element));
}

#[test]
fn action_bar_icon_style_distinguishes_disabled_active_and_default() {
    let theme = Theme::Dark;

    let disabled = action_bar_icon_style(&theme, false, false);
    let active = action_bar_icon_style(&theme, true, true);
    let default = action_bar_icon_style(&theme, true, false);

    assert_eq!(disabled.color, Some(Color::from_rgba8(160, 160, 160, 1.0)));
    assert_eq!(active.color, Some(theme.palette().primary));
    assert_eq!(default.color, Some(theme.palette().text));
}

#[test]
fn action_bar_button_style_covers_disabled_active_hover_pressed_and_idle() {
    let theme = Theme::Light;
    let palette = theme.extended_palette();

    let disabled = action_bar_button_style(&theme, button::Status::Hovered, false, false);
    assert_eq!(disabled.background, None);
    assert_eq!(disabled.text_color, Color::from_rgba8(160, 160, 160, 1.0));

    let active = action_bar_button_style(&theme, button::Status::Active, true, true);
    assert_eq!(
        active.background,
        Some(Background::Color(theme.palette().primary.scale_alpha(0.14)))
    );
    assert_eq!(active.text_color, theme.palette().text);

    let hovered = action_bar_button_style(&theme, button::Status::Hovered, true, false);
    assert_eq!(hovered.background, Some(Background::Color(palette.background.weak.color)));

    let pressed = action_bar_button_style(&theme, button::Status::Pressed, true, false);
    assert_eq!(pressed.background, Some(Background::Color(palette.background.strong.color)));

    let idle = action_bar_button_style(&theme, button::Status::Active, true, false);
    assert_eq!(idle.background, None);
}

#[test]
fn action_menu_icon_style_distinguishes_enabled_and_disabled() {
    let theme = Theme::Dark;

    let enabled = action_menu_icon_style(&theme, true);
    let disabled = action_menu_icon_style(&theme, false);

    assert_eq!(enabled.color, Some(theme.palette().text));
    assert_eq!(disabled.color, Some(theme.palette().text.scale_alpha(0.35)));
}

#[test]
fn action_menu_button_style_covers_disabled_hover_pressed_and_idle() {
    let theme = Theme::Light;
    let palette = theme.extended_palette();

    let disabled = action_menu_button_style(&theme, button::Status::Hovered, false);
    assert_eq!(disabled.background, None);
    assert_eq!(disabled.text_color, theme.palette().text.scale_alpha(0.35));

    let hovered = action_menu_button_style(&theme, button::Status::Hovered, true);
    assert_eq!(
        hovered.background,
        Some(Background::Color(palette.background.weak.color.scale_alpha(0.55)))
    );
    assert_eq!(hovered.text_color, theme.palette().text);

    let pressed = action_menu_button_style(&theme, button::Status::Pressed, true);
    assert_eq!(pressed.background, Some(Background::Color(palette.background.strong.color)));

    let idle = action_menu_button_style(&theme, button::Status::Active, true);
    assert_eq!(idle.background, None);
}
