//! 工作流画布测试模块的入口检查，确保拆分用例持续接入父模块。
//! 该锚点只验证模块连线，不承担具体渲染或交互断言。
use iced::widget::canvas::{Event, Program};
use serde_yaml::Value;

use super::*;

fn handle(kind: WorkflowHandleKind, id: &str) -> WorkflowHandle {
    WorkflowHandle { id: id.to_string(), label: id.to_string(), kind }
}

fn node(id: &str, position: Point, z_index: f32) -> WorkflowNode {
    WorkflowNode {
        id: id.to_string(),
        block_type: if id == "a" { "start" } else { "answer" }.to_string(),
        title: id.to_string(),
        description: String::new(),
        position,
        size: Size::new(120.0, 80.0),
        parent_id: None,
        selected: false,
        source_side: WorkflowHandleSide::Right,
        target_side: WorkflowHandleSide::Left,
        source_handles: vec![handle(WorkflowHandleKind::Source, "source")],
        target_handles: vec![handle(WorkflowHandleKind::Target, "target")],
        z_index,
        raw_node: Value::Null,
    }
}

fn edge() -> WorkflowEdge {
    WorkflowEdge {
        id: "edge-a-b".to_string(),
        source: "a".to_string(),
        target: "b".to_string(),
        source_handle: Some("source".to_string()),
        target_handle: Some("target".to_string()),
        source_type: "start".to_string(),
        target_type: "answer".to_string(),
        selected: false,
        z_index: 0.0,
        raw_edge: Value::Null,
    }
}

fn document() -> WorkflowDocument {
    WorkflowDocument {
        nodes: vec![
            node("a", Point::new(20.0, 20.0), 0.0),
            node("b", Point::new(260.0, 20.0), 1.0),
        ],
        edges: vec![edge()],
        ..WorkflowDocument::default()
    }
}

fn canvas<'a>(document: &'a WorkflowDocument) -> WorkflowCanvas<'a> {
    WorkflowCanvas {
        document,
        pan: Vector::new(0.0, 0.0),
        zoom: 1.0,
        selected_node_id: None,
        selected_edge_id: None,
        connection_draft: None,
    }
}

#[test]
fn hit_testing_prefers_topmost_nodes_and_handles() {
    let document = document();
    let canvas = canvas(&document);
    let slots = build_handle_slots(&document);

    assert_eq!(canvas.hit_test_node(Point::new(270.0, 30.0)), Some("b".to_string()));
    assert_eq!(canvas.hit_test_node(Point::new(500.0, 500.0)), None);

    let endpoint = canvas
        .hit_test_handle(Point::new(140.0, 60.0), &slots)
        .expect("source handle should be hit");
    assert_eq!(endpoint.node_id, "a");
    assert_eq!(endpoint.handle_id, "source");
    assert_eq!(endpoint.kind, WorkflowHandleKind::Source);
}

#[test]
fn hit_testing_edges_ignores_missing_nodes() {
    let mut document = document();
    document.edges.push(WorkflowEdge {
        id: "missing".to_string(),
        source: "x".to_string(),
        ..edge()
    });
    let canvas = canvas(&document);
    let slots = build_handle_slots(&document);

    assert_eq!(canvas.hit_test_edge(Point::new(200.0, 60.0), &slots), Some("edge-a-b".to_string()));
}

#[test]
fn update_without_cursor_returns_none_for_press_and_move() {
    let document = document();
    let canvas = canvas(&document);
    let bounds = Rectangle::new(Point::ORIGIN, Size::new(640.0, 480.0));
    let mut state = WorkflowCanvasState::default();

    assert!(
        canvas
            .update(
                &mut state,
                &Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
                bounds,
                mouse::Cursor::Unavailable,
            )
            .is_none()
    );
    assert!(
        canvas
            .update(
                &mut state,
                &Event::Mouse(mouse::Event::CursorMoved { position: Point::new(1.0, 1.0) }),
                bounds,
                mouse::Cursor::Unavailable,
            )
            .is_none()
    );
}

#[test]
fn update_handles_pan_wheel_and_release_paths() {
    let document = document();
    let canvas = canvas(&document);
    let bounds = Rectangle::new(Point::ORIGIN, Size::new(640.0, 480.0));
    let mut state = WorkflowCanvasState::default();

    let action = canvas.update(
        &mut state,
        &Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
        bounds,
        mouse::Cursor::Available(Point::new(500.0, 400.0)),
    );
    assert!(action.is_some());

    let action = canvas.update(
        &mut state,
        &Event::Mouse(mouse::Event::CursorMoved { position: Point::new(510.0, 410.0) }),
        bounds,
        mouse::Cursor::Available(Point::new(510.0, 410.0)),
    );
    assert!(action.is_some());

    let action = canvas.update(
        &mut state,
        &Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)),
        bounds,
        mouse::Cursor::Available(Point::new(510.0, 410.0)),
    );
    assert!(action.is_some());

    let wheel = canvas.update(
        &mut state,
        &Event::Mouse(mouse::Event::WheelScrolled {
            delta: mouse::ScrollDelta::Lines { x: 1.0, y: -2.0 },
        }),
        bounds,
        mouse::Cursor::Available(Point::new(10.0, 10.0)),
    );
    assert!(wheel.is_some());

    let zero_wheel = canvas.update(
        &mut state,
        &Event::Mouse(mouse::Event::WheelScrolled {
            delta: mouse::ScrollDelta::Pixels { x: 0.0, y: 0.0 },
        }),
        bounds,
        mouse::Cursor::Available(Point::new(10.0, 10.0)),
    );
    assert!(zero_wheel.is_none());
}

#[test]
fn mouse_interaction_reflects_drag_and_hover_targets() {
    let document = document();
    let canvas = canvas(&document);
    let bounds = Rectangle::new(Point::ORIGIN, Size::new(640.0, 480.0));
    let mut state = WorkflowCanvasState::default();

    assert_eq!(
        canvas.mouse_interaction(&state, bounds, mouse::Cursor::Unavailable),
        mouse::Interaction::Grab
    );
    assert_eq!(
        canvas.mouse_interaction(&state, bounds, mouse::Cursor::Available(Point::new(30.0, 30.0)),),
        mouse::Interaction::Pointer
    );

    state.drag_mode = DragMode::Pan;
    assert_eq!(
        canvas.mouse_interaction(
            &state,
            bounds,
            mouse::Cursor::Available(Point::new(500.0, 400.0)),
        ),
        mouse::Interaction::Grabbing
    );

    state.drag_mode = DragMode::Connection(WorkflowConnectionEndpoint {
        node_id: "a".to_string(),
        handle_id: "source".to_string(),
        kind: WorkflowHandleKind::Source,
    });
    assert_eq!(
        canvas.mouse_interaction(
            &state,
            bounds,
            mouse::Cursor::Available(Point::new(500.0, 400.0)),
        ),
        mouse::Interaction::Pointer
    );
}
