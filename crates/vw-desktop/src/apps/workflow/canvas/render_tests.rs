//! 工作流画布渲染的测试，覆盖节点、连线、选中态和临时连接。
//! 记录型绘制检查用于核对层级顺序、裁剪及主题颜色。
use super::*;
use serde_yaml::Value;
use std::collections::{HashMap, HashSet};

fn handle(kind: WorkflowHandleKind, id: &str, label: &str) -> WorkflowHandle {
    WorkflowHandle { id: id.to_string(), label: label.to_string(), kind }
}

fn node(id: &str, block_type: &str, position: Point) -> WorkflowNode {
    WorkflowNode {
        id: id.to_string(),
        block_type: block_type.to_string(),
        title: id.to_string(),
        description: String::new(),
        position,
        size: Size::new(160.0, 96.0),
        parent_id: None,
        selected: false,
        source_side: WorkflowHandleSide::Right,
        target_side: WorkflowHandleSide::Left,
        source_handles: vec![handle(WorkflowHandleKind::Source, "source", "输出")],
        target_handles: vec![handle(WorkflowHandleKind::Target, "target", "输入")],
        z_index: 0.0,
        raw_node: Value::Null,
    }
}

fn edge(id: &str, source: &str, target: &str, source_handle: Option<&str>) -> WorkflowEdge {
    WorkflowEdge {
        id: id.to_string(),
        source: source.to_string(),
        target: target.to_string(),
        source_handle: source_handle.map(str::to_string),
        target_handle: Some("target".to_string()),
        source_type: "llm".to_string(),
        target_type: "answer".to_string(),
        selected: false,
        z_index: 0.0,
        raw_edge: Value::Null,
    }
}

fn document() -> WorkflowDocument {
    WorkflowDocument {
        nodes: vec![
            node("source", "llm", Point::new(20.0, 30.0)),
            node("target", "answer", Point::new(300.0, 60.0)),
        ],
        edges: vec![edge("edge", "source", "target", Some("true"))],
        ..WorkflowDocument::default()
    }
}

#[test]
fn render_helpers_are_wired_through_canvas_module() {
    let _grid: fn(&mut Frame, Size, Vector, f32, &Theme) = draw_grid;
    let _edges: fn(
        &mut Frame,
        &WorkflowDocument,
        Vector,
        f32,
        &Theme,
        Option<&str>,
        Option<&str>,
        Option<&str>,
        &HandleSlots,
    ) = draw_edges;
    let _draft: fn(
        &mut Frame,
        &WorkflowDocument,
        Vector,
        f32,
        Option<&WorkflowConnectionDraft>,
        &HandleSlots,
    ) = draw_connection_draft;
    let _nodes: fn(
        &mut Frame,
        &WorkflowDocument,
        Vector,
        f32,
        Option<&str>,
        Option<&str>,
        Option<&str>,
        Option<&WorkflowConnectionEndpoint>,
        &Theme,
        Color,
        &HandleSlots,
    ) = draw_nodes;
}

#[test]
fn grid_plan_covers_zoom_buckets_and_theme_colors() {
    assert_eq!(grid_spacing(0.2), 144.0);
    assert_eq!(grid_spacing(0.35), 72.0);
    assert_eq!(grid_spacing(0.6), 48.0);
    assert_eq!(grid_spacing(1.0), 32.0);

    let light = grid_plan(Size::new(96.0, 64.0), Vector::new(16.0, -8.0), 0.5, &Theme::Light);
    assert_eq!(light.spacing, 72.0);
    assert_eq!(light.start_x, -2);
    assert_eq!(light.start_y, -1);
    assert!(light.end_x > light.start_x);
    assert!(light.end_y > light.start_y);
    assert!((light.dot_color.a - 0.32).abs() < f32::EPSILON);
    assert!((light.major_dot_color.a - 0.28).abs() < f32::EPSILON);

    let dark = grid_plan(Size::new(10.0, 10.0), Vector::new(0.0, 0.0), 1.5, &Theme::Dark);
    assert_eq!(dark.spacing, 32.0);
    assert!((dark.dot_color.a - 0.12).abs() < f32::EPSILON);
    assert!((dark.major_dot_color.a - 0.20).abs() < f32::EPSILON);
}

#[test]
fn edge_plan_skips_missing_nodes_and_marks_selected_or_hovered_edges() {
    let document = document();
    let slots = build_handle_slots(&document);
    let node_map =
        document.nodes.iter().map(|node| (node.id.as_str(), node)).collect::<HashMap<_, _>>();
    let missing = edge("missing", "missing", "target", Some("true"));

    assert!(
        edge_plan(
            &missing,
            EdgePlanInput {
                node_map: &node_map,
                pan: Vector::new(0.0, 0.0),
                zoom: 1.0,
                theme: &Theme::Light,
                selected_node_id: None,
                selected_edge_id: None,
                hovered_edge_id: None,
                handle_slots: &slots,
            },
        )
        .is_none()
    );

    let selected = edge_plan(
        &document.edges[0],
        EdgePlanInput {
            node_map: &node_map,
            pan: Vector::new(4.0, -2.0),
            zoom: 2.0,
            theme: &Theme::Light,
            selected_node_id: Some("source"),
            selected_edge_id: None,
            hovered_edge_id: None,
            handle_slots: &slots,
        },
    )
    .expect("edge should have endpoints");
    assert!(selected.selected);
    assert!(!selected.hovered);
    assert!((selected.width - 5.6).abs() < f32::EPSILON);
    assert_eq!(selected.start, Point::new(364.0, 186.0));

    let hovered = edge_plan(
        &document.edges[0],
        EdgePlanInput {
            node_map: &node_map,
            pan: Vector::new(0.0, 0.0),
            zoom: 0.5,
            theme: &Theme::Dark,
            selected_node_id: None,
            selected_edge_id: None,
            hovered_edge_id: Some("edge"),
            handle_slots: &slots,
        },
    )
    .expect("edge should have endpoints");
    assert!(!hovered.selected);
    assert!(hovered.hovered);
    assert!((hovered.width - 1.1).abs() < f32::EPSILON);
    assert!((hovered.color.a - 0.82).abs() < f32::EPSILON);
}

#[test]
fn draft_plan_handles_absent_drafts_and_endpoint_direction() {
    let document = document();
    let slots = build_handle_slots(&document);

    assert!(draft_plan(&document, Vector::new(0.0, 0.0), 1.0, None, &slots).is_none());

    let source_draft = WorkflowConnectionDraft {
        from: WorkflowConnectionEndpoint {
            node_id: "source".to_string(),
            handle_id: "source".to_string(),
            kind: WorkflowHandleKind::Source,
        },
        cursor_world: Point::new(260.0, 140.0),
    };
    let source_plan =
        draft_plan(&document, Vector::new(0.0, 0.0), 1.0, Some(&source_draft), &slots)
            .expect("source draft should resolve");
    assert_eq!(source_plan.start, Point::new(180.0, 62.0));
    assert!(source_plan.c1.x > source_plan.start.x);
    assert!(source_plan.c2.x < source_plan.end.x);
    assert!((source_plan.width - 2.2).abs() < f32::EPSILON);

    let target_draft = WorkflowConnectionDraft {
        from: WorkflowConnectionEndpoint {
            node_id: "target".to_string(),
            handle_id: "target".to_string(),
            kind: WorkflowHandleKind::Target,
        },
        cursor_world: Point::new(220.0, 10.0),
    };
    let target_plan =
        draft_plan(&document, Vector::new(10.0, 0.0), 0.5, Some(&target_draft), &slots)
            .expect("target draft should resolve");
    assert!(target_plan.c1.x < target_plan.start.x);
    assert!(target_plan.c2.x > target_plan.end.x);
    assert!((target_plan.color.a - 0.86).abs() < f32::EPSILON);
}

#[test]
fn node_plan_wraps_descriptions_and_tracks_selection_state() {
    let mut parent = node("parent", "iteration", Point::new(0.0, 0.0));
    parent.size = Size::new(220.0, 180.0);
    let child = WorkflowNode {
        parent_id: Some("parent".to_string()),
        ..node("child", "llm", Point::new(20.0, 30.0))
    };
    let document =
        WorkflowDocument { nodes: vec![parent.clone(), child], ..WorkflowDocument::default() };
    let selected_group_ids = HashSet::from(["parent"]);

    let plan = node_plan(
        &document,
        &parent,
        Vector::new(5.0, 10.0),
        0.5,
        Some("child"),
        Some("parent"),
        &selected_group_ids,
    );

    assert_eq!(plan.rect, Rectangle::new(Point::new(5.0, 10.0), Size::new(110.0, 90.0)));
    assert!(!plan.is_selected);
    assert!(plan.is_hovered);
    assert!(plan.descendant_selected);
    assert!(!plan.show_start_variables);
    assert_eq!(plan.desc_lines, vec!["包含 1 个子节点".to_string()]);
    assert_eq!(plan.description_x, plan.title_x);
    assert_eq!(plan.content_scale, 0.5);
}

#[test]
fn node_plan_prefers_pretty_titles_and_start_variable_rows() {
    let raw_node = serde_yaml::from_str::<Value>(
        "data:\n  variables:\n    - variable: city\n      type: text-input\n    - label: retries\n      type: number\n    - variable: attachment\n      type: file\n",
    )
    .expect("start node yaml should parse");
    let mut start = node("start", "start", Point::new(12.0, 18.0));
    start.title = "   ".to_string();
    start.size = Size::new(260.0, 180.0);
    start.raw_node = raw_node;
    let document = WorkflowDocument { nodes: vec![start.clone()], ..WorkflowDocument::default() };
    let plan = node_plan(
        &document,
        &start,
        Vector::new(0.0, 0.0),
        1.0,
        Some("start"),
        None,
        &HashSet::new(),
    );

    assert!(plan.is_selected);
    assert!(plan.show_start_variables);
    assert_eq!(plan.title_lines, vec![pretty_block_type("start")]);
    assert_eq!(
        plan.start_variable_rows,
        vec![
            ("city".to_string(), "T".to_string()),
            ("retries".to_string(), "#".to_string()),
            ("attachment".to_string(), "F".to_string()),
        ]
    );
    assert!(plan.desc_lines.is_empty());
    assert_eq!(plan.content_top, 36.0);
}

#[test]
fn edge_plan_uses_base_color_when_not_selected_or_hovered() {
    let document = document();
    let slots = build_handle_slots(&document);
    let node_map =
        document.nodes.iter().map(|node| (node.id.as_str(), node)).collect::<HashMap<_, _>>();

    let plan = edge_plan(
        &document.edges[0],
        EdgePlanInput {
            node_map: &node_map,
            pan: Vector::new(0.0, 0.0),
            zoom: 1.0,
            theme: &Theme::Light,
            selected_node_id: None,
            selected_edge_id: None,
            hovered_edge_id: None,
            handle_slots: &slots,
        },
    )
    .expect("edge should have endpoints");

    assert!(!plan.selected);
    assert!(!plan.hovered);
    assert_eq!(plan.color, edge_base_color(&Theme::Light));
    assert!((plan.width - 1.6).abs() < f32::EPSILON);

    let missing_target = edge("missing-target", "source", "absent", None);
    assert!(
        edge_plan(
            &missing_target,
            EdgePlanInput {
                node_map: &node_map,
                pan: Vector::new(0.0, 0.0),
                zoom: 1.0,
                theme: &Theme::Dark,
                selected_node_id: None,
                selected_edge_id: None,
                hovered_edge_id: None,
                handle_slots: &slots,
            },
        )
        .is_none()
    );

    let dark_base = edge_base_color(&Theme::Dark);
    assert_eq!(dark_base, Theme::Dark.palette().text.scale_alpha(0.22));
}

#[test]
fn edge_plan_marks_explicit_selected_edge_without_selected_nodes() {
    let document = document();
    let slots = build_handle_slots(&document);
    let node_map =
        document.nodes.iter().map(|node| (node.id.as_str(), node)).collect::<HashMap<_, _>>();

    let plan = edge_plan(
        &document.edges[0],
        EdgePlanInput {
            node_map: &node_map,
            pan: Vector::new(0.0, 0.0),
            zoom: 0.25,
            theme: &Theme::Light,
            selected_node_id: None,
            selected_edge_id: Some("edge"),
            hovered_edge_id: None,
            handle_slots: &slots,
        },
    )
    .expect("edge should have endpoints");

    assert!(plan.selected);
    assert!(!plan.hovered);
    assert!((plan.width - 0.84).abs() < f32::EPSILON);
}

#[test]
fn draft_plan_rejects_unknown_origin_node() {
    let document = document();
    let slots = build_handle_slots(&document);
    let draft = WorkflowConnectionDraft {
        from: WorkflowConnectionEndpoint {
            node_id: "absent".to_string(),
            handle_id: "source".to_string(),
            kind: WorkflowHandleKind::Source,
        },
        cursor_world: Point::new(50.0, 60.0),
    };

    assert!(draft_plan(&document, Vector::new(0.0, 0.0), 1.0, Some(&draft), &slots).is_none());
}

#[test]
fn node_plan_hides_description_when_card_is_too_short() {
    let mut compact = node("compact", "llm", Point::new(4.0, 8.0));
    compact.title = "A deliberately long title for wrapping".to_string();
    compact.description = "This description should not be rendered".to_string();
    compact.size = Size::new(120.0, 60.0);
    let document = WorkflowDocument { nodes: vec![compact.clone()], ..WorkflowDocument::default() };

    let plan =
        node_plan(&document, &compact, Vector::new(0.0, 0.0), 1.0, None, None, &HashSet::new());

    assert_eq!(plan.title_lines.len(), 1);
    assert!(plan.desc_lines.is_empty());
    assert_eq!(plan.title_font_size, 13.0);
    assert_eq!(plan.content_scale, 1.0);
}

#[test]
fn node_plan_uses_tall_card_description_layout_when_space_allows() {
    let mut tall = node("tall", "llm", Point::new(0.0, 0.0));
    tall.title = "Long enough title to wrap across two lines".to_string();
    tall.description =
        "Detailed description that should remain visible on a tall workflow node".to_string();
    tall.size = Size::new(180.0, 180.0);
    let document = WorkflowDocument { nodes: vec![tall.clone()], ..WorkflowDocument::default() };

    let plan = node_plan(&document, &tall, Vector::new(0.0, 0.0), 1.0, None, None, &HashSet::new());

    assert_eq!(plan.title_lines.len(), 2);
    assert!(!plan.desc_lines.is_empty());
    assert_eq!(plan.desc_font_size, 12.0);
    assert!(plan.content_top > 0.0);
}
