#[test]
fn test_module_is_wired() {
    let module = std::hint::black_box(module_path!());

    assert!(module.ends_with("ids_tests"));
}

use super::*;
use crate::apps::workflow::model::{WorkflowHandleSide, WorkflowViewport};
use iced::Size;

fn assert_approx_eq(actual: f32, expected: f32) {
    assert!((actual - expected).abs() < 0.001, "expected {expected}, got {actual}");
}

fn assert_numeric_suffix(value: &str, prefix: &str) {
    let suffix = value.strip_prefix(prefix).expect("generated ID should use expected prefix");

    assert!(!suffix.is_empty());
    assert!(suffix.chars().all(|ch| ch.is_ascii_digit()));
}

fn node_at(id: &str, position: Point, size: Size) -> WorkflowNode {
    WorkflowNode {
        id: id.to_string(),
        block_type: "answer".to_string(),
        title: id.to_string(),
        description: String::new(),
        position,
        size,
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
fn fitted_viewport_returns_default_for_empty_document() {
    let (pan, zoom) = fitted_viewport(&WorkflowDocument::default(), (800.0, 600.0));

    assert_eq!(pan, Vector::new(120.0, 120.0));
    assert_eq!(zoom, 1.0);
}

#[test]
fn fitted_viewport_centers_document_and_clamps_zoom() {
    let document = WorkflowDocument {
        nodes: vec![node_at("small", Point::new(0.0, 0.0), Size::new(100.0, 100.0))],
        viewport: WorkflowViewport::default(),
        ..WorkflowDocument::default()
    };
    let (pan, zoom) = fitted_viewport(&document, (5000.0, 5000.0));

    assert_eq!(zoom, 4.0);
    assert_eq!(pan, Vector::new(2134.0, 2214.0));

    let large_document = WorkflowDocument {
        nodes: vec![node_at("large", Point::new(-5000.0, -5000.0), Size::new(10_000.0, 10_000.0))],
        viewport: WorkflowViewport::default(),
        ..WorkflowDocument::default()
    };
    let (_, small_zoom) = fitted_viewport(&large_document, (0.0, 0.0));
    assert_eq!(small_zoom, 0.1);
}

#[test]
fn fitted_viewport_uses_limiting_axis_and_world_center() {
    let width_limited = WorkflowDocument {
        nodes: vec![node_at("wide", Point::new(100.0, 50.0), Size::new(200.0, 100.0))],
        viewport: WorkflowViewport::default(),
        ..WorkflowDocument::default()
    };
    let (width_pan, width_zoom) = fitted_viewport(&width_limited, (1000.0, 800.0));

    assert_approx_eq(width_zoom, 2.852);
    assert_approx_eq(width_pan.x, -236.4);
    assert_approx_eq(width_pan.y, 28.8);

    let height_limited = WorkflowDocument {
        nodes: vec![node_at("tall", Point::new(-50.0, -100.0), Size::new(100.0, 400.0))],
        viewport: WorkflowViewport::default(),
        ..WorkflowDocument::default()
    };
    let (height_pan, height_zoom) = fitted_viewport(&height_limited, (800.0, 600.0));

    assert_approx_eq(height_zoom, 0.874);
    assert_approx_eq(height_pan.x, 234.0);
    assert_approx_eq(height_pan.y, 126.6);
}

#[test]
fn fitted_viewport_uses_minimum_canvas_size_for_small_windows() {
    let document = WorkflowDocument {
        nodes: vec![node_at("node", Point::new(0.0, 0.0), Size::new(100.0, 100.0))],
        viewport: WorkflowViewport::default(),
        ..WorkflowDocument::default()
    };

    let (pan, zoom) = fitted_viewport(&document, (0.0, 0.0));

    assert_approx_eq(zoom, 2.392);
    assert_approx_eq(pan.x, 64.4);
    assert_approx_eq(pan.y, 34.4);
}

#[test]
fn generated_runtime_ids_use_expected_segments_and_numeric_suffixes() {
    assert_numeric_suffix(&generate_app_id(), "workflow-app-");
    assert_numeric_suffix(&generate_node_id("if-else"), "if_else-node-");
    assert_numeric_suffix(&generate_variable_id("env secret"), "env_secret-var-");
    assert_numeric_suffix(&generate_start_variable_name(), "input_");
    assert_numeric_suffix(&generate_case_id(), "case-");
    assert_numeric_suffix(&generate_condition_id(), "condition-");
    assert_numeric_suffix(&generate_prompt_item_id("system-prompt"), "system_prompt-prompt-");
}

#[test]
fn normalize_connection_endpoints_accepts_either_order_and_rejects_same_kind() {
    let source = WorkflowConnectionEndpoint {
        node_id: "source".to_string(),
        handle_id: "out".to_string(),
        kind: WorkflowHandleKind::Source,
    };
    let target = WorkflowConnectionEndpoint {
        node_id: "target".to_string(),
        handle_id: "in".to_string(),
        kind: WorkflowHandleKind::Target,
    };

    let normalized = normalize_connection_endpoints(&source, &target).expect("source to target");
    assert_eq!(normalized, (source.clone(), target.clone()));

    let reversed = normalize_connection_endpoints(&target, &source).expect("target to source");
    assert_eq!(reversed, (source.clone(), target.clone()));

    assert!(normalize_connection_endpoints(&source, &source).is_none());
    assert!(normalize_connection_endpoints(&target, &target).is_none());
}

#[test]
fn generate_edge_id_and_sanitize_handle_id_replace_non_ascii_alphanumeric_chars() {
    let source = WorkflowConnectionEndpoint {
        node_id: "source-node".to_string(),
        handle_id: "out-1".to_string(),
        kind: WorkflowHandleKind::Source,
    };
    let target = WorkflowConnectionEndpoint {
        node_id: "target-node".to_string(),
        handle_id: "in.2".to_string(),
        kind: WorkflowHandleKind::Target,
    };

    let edge_id = generate_edge_id(&source, &target);

    assert_numeric_suffix(&edge_id, "manual-source-node-out_1-target-node-in_2-");
    assert_eq!(sanitize_handle_id("A-b_中 1"), "A_b___1");
}

#[test]
fn sanitize_handle_id_handles_empty_safe_punctuation_and_unicode_inputs() {
    assert_eq!(sanitize_handle_id(""), "");
    assert_eq!(sanitize_handle_id("abcXYZ012"), "abcXYZ012");
    assert_eq!(sanitize_handle_id("-./"), "___");
    assert_eq!(sanitize_handle_id("é中🙂"), "___");
}
