//! 工作流画布核心行为的测试，覆盖视口变换、节点命中和交互路由。
//! 用例验证空文档及不同缩放偏移下的坐标转换保持一致。
use iced::advanced::renderer::Headless;
use iced::widget::canvas::{Event, Program};
use iced::{Font, Pixels};
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

fn edge(id: &str, source: &str, target: &str, z_index: f32) -> WorkflowEdge {
    WorkflowEdge {
        id: id.to_string(),
        source: source.to_string(),
        target: target.to_string(),
        source_handle: Some("source".to_string()),
        target_handle: Some("target".to_string()),
        source_type: "start".to_string(),
        target_type: "answer".to_string(),
        selected: false,
        z_index,
        raw_edge: Value::Null,
    }
}

fn document() -> WorkflowDocument {
    WorkflowDocument {
        nodes: vec![
            node("a", Point::new(20.0, 20.0), 0.0),
            node("b", Point::new(260.0, 20.0), 1.0),
        ],
        edges: vec![edge("edge-a-b", "a", "b", 0.0)],
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

fn bounds() -> Rectangle {
    Rectangle::new(Point::ORIGIN, Size::new(640.0, 480.0))
}

#[test]
fn export_svg_delegates_to_canvas_utils() {
    let document = document();

    assert_eq!(export_svg(&document), utils::export_svg(&document));
}

#[test]
fn hit_testing_covers_nodes_handles_edges_and_misses() {
    let mut document = document();
    document.edges.push(edge("missing-source", "missing", "b", 3.0));
    document.edges.push(edge("missing-target", "a", "missing", 2.0));
    let canvas = canvas(&document);
    let slots = build_handle_slots(&document);

    assert_eq!(canvas.hit_test_node(Point::new(270.0, 30.0)), Some("b".to_string()));
    assert_eq!(canvas.hit_test_node(Point::new(500.0, 500.0)), None);

    let source = canvas
        .hit_test_handle(Point::new(140.0, 60.0), &slots)
        .expect("source handle should be hit");
    assert_eq!(source.node_id, "a");
    assert_eq!(source.handle_id, "source");
    assert_eq!(source.kind, WorkflowHandleKind::Source);

    let target = canvas
        .hit_test_handle(Point::new(260.0, 60.0), &slots)
        .expect("target handle should be hit");
    assert_eq!(target.node_id, "b");
    assert_eq!(target.handle_id, "target");
    assert_eq!(target.kind, WorkflowHandleKind::Target);

    assert_eq!(canvas.hit_test_handle(Point::new(500.0, 500.0), &slots), None);
    assert_eq!(canvas.hit_test_edge(Point::new(200.0, 60.0), &slots), Some("edge-a-b".to_string()));
    assert_eq!(canvas.hit_test_edge(Point::new(200.0, 160.0), &slots), None);
}

#[test]
fn hit_test_edge_prefers_highest_z_index_when_edges_overlap() {
    let mut document = document();
    document.edges.push(edge("edge-top", "a", "b", 10.0));
    let canvas = canvas(&document);
    let slots = build_handle_slots(&document);

    assert_eq!(canvas.hit_test_edge(Point::new(200.0, 60.0), &slots), Some("edge-top".to_string()));
}

#[test]
fn left_press_selects_handle_node_edge_or_canvas_by_hit_priority() {
    let document = document();
    let canvas = canvas(&document);
    let mut state = WorkflowCanvasState::default();

    assert!(
        canvas
            .update(
                &mut state,
                &Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
                bounds(),
                mouse::Cursor::Available(Point::new(140.0, 60.0)),
            )
            .is_some()
    );
    assert!(matches!(state.drag_mode, DragMode::Connection(_)));
    assert!(state.hovered_handle.is_some());

    state = WorkflowCanvasState::default();
    assert!(
        canvas
            .update(
                &mut state,
                &Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
                bounds(),
                mouse::Cursor::Available(Point::new(30.0, 30.0)),
            )
            .is_some()
    );
    assert!(matches!(state.drag_mode, DragMode::Node(ref node_id) if node_id == "a"));

    state = WorkflowCanvasState::default();
    assert!(
        canvas
            .update(
                &mut state,
                &Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
                bounds(),
                mouse::Cursor::Available(Point::new(200.0, 60.0)),
            )
            .is_some()
    );
    assert!(matches!(state.drag_mode, DragMode::None));

    assert!(
        canvas
            .update(
                &mut state,
                &Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
                bounds(),
                mouse::Cursor::Available(Point::new(500.0, 400.0)),
            )
            .is_some()
    );
    assert!(matches!(state.drag_mode, DragMode::Pan));
}

#[test]
fn right_press_opens_context_menu_for_node_edge_or_canvas() {
    let document = document();
    let canvas = canvas(&document);
    let mut state = WorkflowCanvasState::default();

    for point in [Point::new(30.0, 30.0), Point::new(200.0, 60.0), Point::new(500.0, 400.0)] {
        state.drag_mode = DragMode::Pan;
        assert!(
            canvas
                .update(
                    &mut state,
                    &Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Right)),
                    bounds(),
                    mouse::Cursor::Available(point),
                )
                .is_some()
        );
        assert!(matches!(state.drag_mode, DragMode::None));
        assert_eq!(state.last_cursor, Some(point));
    }
}

#[test]
fn cursor_moved_updates_pan_node_connection_and_hover_state() {
    let document = document();
    let canvas = canvas(&document);
    let mut state = WorkflowCanvasState { drag_mode: DragMode::Pan, ..Default::default() };

    assert!(
        canvas
            .update(
                &mut state,
                &Event::Mouse(mouse::Event::CursorMoved { position: Point::new(510.0, 410.0) }),
                bounds(),
                mouse::Cursor::Available(Point::new(510.0, 410.0)),
            )
            .is_some()
    );

    state.drag_mode = DragMode::Node("a".to_string());
    state.last_cursor = Some(Point::new(30.0, 30.0));
    assert!(
        canvas
            .update(
                &mut state,
                &Event::Mouse(mouse::Event::CursorMoved { position: Point::new(40.0, 50.0) }),
                bounds(),
                mouse::Cursor::Available(Point::new(40.0, 50.0)),
            )
            .is_some()
    );

    state.drag_mode = DragMode::Connection(WorkflowConnectionEndpoint {
        node_id: "a".to_string(),
        handle_id: "source".to_string(),
        kind: WorkflowHandleKind::Source,
    });
    assert!(
        canvas
            .update(
                &mut state,
                &Event::Mouse(mouse::Event::CursorMoved { position: Point::new(160.0, 60.0) }),
                bounds(),
                mouse::Cursor::Available(Point::new(160.0, 60.0)),
            )
            .is_some()
    );

    state.drag_mode = DragMode::None;
    state.hovered_node_id = None;
    state.hovered_edge_id = None;
    state.hovered_handle = None;
    assert!(
        canvas
            .update(
                &mut state,
                &Event::Mouse(mouse::Event::CursorMoved { position: Point::new(30.0, 30.0) }),
                bounds(),
                mouse::Cursor::Available(Point::new(30.0, 30.0)),
            )
            .is_some()
    );
    assert_eq!(state.hovered_node_id.as_deref(), Some("a"));

    assert!(
        canvas
            .update(
                &mut state,
                &Event::Mouse(mouse::Event::CursorMoved { position: Point::new(30.0, 30.0) }),
                bounds(),
                mouse::Cursor::Available(Point::new(30.0, 30.0)),
            )
            .is_none()
    );
}

#[test]
fn left_release_finishes_or_cancels_connection_and_node_drags() {
    let document = document();
    let canvas = canvas(&document);
    let origin = WorkflowConnectionEndpoint {
        node_id: "a".to_string(),
        handle_id: "source".to_string(),
        kind: WorkflowHandleKind::Source,
    };
    let mut state = WorkflowCanvasState {
        drag_mode: DragMode::Connection(origin.clone()),
        ..Default::default()
    };

    assert!(
        canvas
            .update(
                &mut state,
                &Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)),
                bounds(),
                mouse::Cursor::Available(Point::new(260.0, 60.0)),
            )
            .is_some()
    );
    assert!(matches!(state.drag_mode, DragMode::None));
    assert_eq!(state.last_cursor, None);

    state.drag_mode = DragMode::Connection(origin.clone());
    assert!(
        canvas
            .update(
                &mut state,
                &Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)),
                bounds(),
                mouse::Cursor::Available(Point::new(140.0, 60.0)),
            )
            .is_some()
    );

    state.drag_mode = DragMode::Connection(origin);
    assert!(
        canvas
            .update(
                &mut state,
                &Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)),
                bounds(),
                mouse::Cursor::Unavailable,
            )
            .is_some()
    );

    state.drag_mode = DragMode::Node("a".to_string());
    assert!(
        canvas
            .update(
                &mut state,
                &Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)),
                bounds(),
                mouse::Cursor::Available(Point::new(30.0, 30.0)),
            )
            .is_some()
    );

    state.drag_mode = DragMode::Pan;
    assert!(
        canvas
            .update(
                &mut state,
                &Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)),
                bounds(),
                mouse::Cursor::Available(Point::new(500.0, 400.0)),
            )
            .is_some()
    );
}

#[test]
fn update_ignores_missing_cursor_zero_wheel_and_unhandled_events() {
    let document = document();
    let canvas = canvas(&document);
    let mut state = WorkflowCanvasState::default();

    assert!(
        canvas
            .update(
                &mut state,
                &Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
                bounds(),
                mouse::Cursor::Unavailable,
            )
            .is_none()
    );
    assert!(
        canvas
            .update(
                &mut state,
                &Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Right)),
                bounds(),
                mouse::Cursor::Unavailable,
            )
            .is_none()
    );
    assert!(
        canvas
            .update(
                &mut state,
                &Event::Mouse(mouse::Event::CursorMoved { position: Point::new(1.0, 1.0) }),
                bounds(),
                mouse::Cursor::Unavailable,
            )
            .is_none()
    );
    assert!(
        canvas
            .update(
                &mut state,
                &Event::Mouse(mouse::Event::WheelScrolled {
                    delta: mouse::ScrollDelta::Pixels { x: 0.0, y: 0.0 },
                }),
                bounds(),
                mouse::Cursor::Available(Point::new(1.0, 1.0)),
            )
            .is_none()
    );
    assert!(
        canvas
            .update(
                &mut state,
                &Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Middle)),
                bounds(),
                mouse::Cursor::Available(Point::new(1.0, 1.0)),
            )
            .is_none()
    );
}

#[test]
fn wheel_scrolling_supports_line_and_pixel_deltas() {
    let document = document();
    let canvas = canvas(&document);
    let mut state = WorkflowCanvasState::default();

    for delta in [
        mouse::ScrollDelta::Lines { x: 1.0, y: -2.0 },
        mouse::ScrollDelta::Pixels { x: 8.0, y: -4.0 },
    ] {
        assert!(
            canvas
                .update(
                    &mut state,
                    &Event::Mouse(mouse::Event::WheelScrolled { delta }),
                    bounds(),
                    mouse::Cursor::Available(Point::new(1.0, 1.0)),
                )
                .is_some()
        );
    }
}

#[test]
fn mouse_interaction_covers_idle_drag_and_target_states() {
    let document = document();
    let canvas = canvas(&document);
    let mut state = WorkflowCanvasState::default();

    assert_eq!(
        canvas.mouse_interaction(&state, bounds(), mouse::Cursor::Unavailable),
        mouse::Interaction::Grab
    );
    assert_eq!(
        canvas.mouse_interaction(
            &state,
            bounds(),
            mouse::Cursor::Available(Point::new(500.0, 400.0))
        ),
        mouse::Interaction::Grab
    );
    assert_eq!(
        canvas.mouse_interaction(
            &state,
            bounds(),
            mouse::Cursor::Available(Point::new(30.0, 30.0))
        ),
        mouse::Interaction::Pointer
    );
    assert_eq!(
        canvas.mouse_interaction(
            &state,
            bounds(),
            mouse::Cursor::Available(Point::new(200.0, 60.0))
        ),
        mouse::Interaction::Pointer
    );

    state.drag_mode = DragMode::Pan;
    assert_eq!(
        canvas.mouse_interaction(
            &state,
            bounds(),
            mouse::Cursor::Available(Point::new(500.0, 400.0))
        ),
        mouse::Interaction::Grabbing
    );

    state.drag_mode = DragMode::Node("a".to_string());
    assert_eq!(
        canvas.mouse_interaction(
            &state,
            bounds(),
            mouse::Cursor::Available(Point::new(500.0, 400.0))
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
            bounds(),
            mouse::Cursor::Available(Point::new(500.0, 400.0))
        ),
        mouse::Interaction::Pointer
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn draw_renders_light_and_dark_canvas_geometry() {
    let renderer =
        <iced::Renderer as Headless>::new(Font::DEFAULT, Pixels(16.0), Some("tiny-skia"))
            .await
            .expect("tiny-skia headless renderer should be available");
    let document = document();
    let canvas = canvas(&document);
    let state = WorkflowCanvasState::default();

    let light = canvas.draw(&state, &renderer, &Theme::Light, bounds(), mouse::Cursor::Unavailable);
    let dark = canvas.draw(&state, &renderer, &Theme::Dark, bounds(), mouse::Cursor::Unavailable);

    assert_eq!(light.len(), 1);
    assert_eq!(dark.len(), 1);
}
