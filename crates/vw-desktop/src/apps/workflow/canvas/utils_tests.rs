//! 工作流画布辅助函数测试，验证坐标、尺寸和布局计算的稳定性。

use super::*;
use serde_yaml::Value;

fn code_node(error_strategy: Option<&str>) -> WorkflowNode {
    let raw_node = match error_strategy {
        Some(strategy) => {
            serde_yaml::from_str::<Value>(&format!("data:\n  error_strategy: {strategy}\n"))
                .expect("node yaml should parse")
        }
        None => serde_yaml::from_str::<Value>("data: {}\n").expect("node yaml should parse"),
    };

    WorkflowNode {
        id: "code_1".to_string(),
        block_type: "code".to_string(),
        title: "初始化变量".to_string(),
        description: String::new(),
        position: Point::ORIGIN,
        size: Size::new(180.0, 84.0),
        parent_id: None,
        selected: false,
        source_side: WorkflowHandleSide::Right,
        target_side: WorkflowHandleSide::Left,
        source_handles: Vec::new(),
        target_handles: Vec::new(),
        z_index: 0.0,
        raw_node,
    }
}

fn basic_node(id: &str, block_type: &str, title: &str, description: &str) -> WorkflowNode {
    WorkflowNode {
        id: id.to_string(),
        block_type: block_type.to_string(),
        title: title.to_string(),
        description: description.to_string(),
        position: Point::new(10.0, 20.0),
        size: Size::new(180.0, 84.0),
        parent_id: None,
        selected: false,
        source_side: WorkflowHandleSide::Right,
        target_side: WorkflowHandleSide::Left,
        source_handles: vec![WorkflowHandle {
            id: "source".to_string(),
            label: "输出".to_string(),
            kind: WorkflowHandleKind::Source,
        }],
        target_handles: vec![WorkflowHandle {
            id: "target".to_string(),
            label: "输入".to_string(),
            kind: WorkflowHandleKind::Target,
        }],
        z_index: 0.0,
        raw_node: Value::Null,
    }
}

fn edge(source: &str, target: &str, label: Option<&str>) -> WorkflowEdge {
    WorkflowEdge {
        id: format!("{source}-{target}"),
        source: source.to_string(),
        target: target.to_string(),
        source_handle: label.map(str::to_string),
        target_handle: Some("target".to_string()),
        source_type: "llm".to_string(),
        target_type: "answer".to_string(),
        selected: false,
        z_index: 0.0,
        raw_edge: Value::Null,
    }
}

#[test]
fn code_node_description_hides_when_error_strategy_is_none() {
    let document = WorkflowDocument::default();

    assert!(node_description_text(&document, &code_node(None)).is_empty());
    assert!(node_description_text(&document, &code_node(Some("none"))).is_empty());
}

#[test]
fn code_node_description_shows_error_strategy_summary() {
    let document = WorkflowDocument::default();

    assert_eq!(
        node_description_text(&document, &code_node(Some("default-value"))),
        "异常时 输出默认值"
    );
    assert_eq!(
        node_description_text(&document, &code_node(Some("fail-branch"))),
        "异常时 异常分支"
    );
}

#[test]
fn distance_to_segment_handles_degenerate_and_projected_segments() {
    assert_eq!(distance_to_segment(Point::new(3.0, 4.0), Point::ORIGIN, Point::ORIGIN), 5.0);
    assert_eq!(
        distance_to_segment(Point::new(5.0, 5.0), Point::ORIGIN, Point::new(10.0, 0.0)),
        5.0
    );
    assert_eq!(
        distance_to_segment(Point::new(-3.0, 4.0), Point::ORIGIN, Point::new(10.0, 0.0)),
        5.0
    );
    assert_eq!(
        distance_to_segment(Point::new(13.0, 4.0), Point::ORIGIN, Point::new(10.0, 0.0)),
        5.0
    );
}

#[test]
fn wrap_text_lines_trims_wraps_and_truncates() {
    assert_eq!(wrap_text_lines("   ", 8, 2), vec!["".to_string()]);
    assert_eq!(wrap_text_lines("abcdef", 3, 3), vec!["abcd".to_string(), "ef".to_string()]);
    assert_eq!(wrap_text_lines("一二三四五", 4, 1), vec!["一二…".to_string()]);
    assert_eq!(wrap_text_lines("one\n\ntwo", 10, 3), vec!["one".to_string(), "two".to_string()]);
}

#[test]
fn wrap_text_lines_clamps_line_limit_and_preserves_existing_ellipsis() {
    assert_eq!(wrap_text_lines("one\ntwo", 10, 0), vec!["one…".to_string()]);
    assert_eq!(wrap_text_lines("abc…d", 4, 1), vec!["abc…".to_string()]);
    assert_eq!(wrap_text_lines("abcd", 0, 0), vec!["abcd".to_string()]);
}

#[test]
fn wrap_text_lines_skips_blank_raw_lines_and_marks_multiline_truncation() {
    assert_eq!(
        wrap_text_lines(" first \n\n second \n third ", 12, 2),
        vec!["first".to_string(), "second…".to_string()]
    );
}

#[test]
fn display_width_counts_wide_and_unknown_characters() {
    assert_eq!(display_width("abc"), 3);
    assert_eq!(display_width("你好"), 4);
}

#[test]
fn display_width_treats_zero_width_codepoints_as_one_column() {
    assert_eq!(display_width("\u{0301}"), 1);
    assert_eq!(display_width("a\u{0301}"), 2);
}

#[test]
fn glyph_badge_and_line_height_cover_known_and_fallback_values() {
    assert_eq!(node_glyph("start"), "S");
    assert_eq!(node_glyph("end"), "E");
    assert_eq!(node_glyph("answer"), "A");
    assert_eq!(node_glyph("llm"), "AI");
    assert_eq!(node_glyph("if-else"), "IF");
    assert_eq!(node_glyph("code"), "C");
    assert_eq!(node_glyph("tool"), "T");
    assert_eq!(node_glyph("knowledge-retrieval"), "K");
    assert_eq!(node_glyph("question-classifier"), "Q");
    assert_eq!(node_glyph("http-request"), "H");
    assert_eq!(node_glyph("iteration"), "IT");
    assert_eq!(node_glyph("loop"), "L");
    assert_eq!(node_glyph("unknown"), "•");

    assert_eq!(start_variable_badge_text("number"), "#");
    assert_eq!(start_variable_badge_text("boolean"), "B");
    assert_eq!(start_variable_badge_text("file"), "F");
    assert_eq!(start_variable_badge_text("array[file]"), "F");
    assert_eq!(start_variable_badge_text("string"), "T");

    assert_eq!(line_block_height(0, 12.0, 16.0), 0.0);
    assert_eq!(line_block_height(3, 12.0, 16.0), 44.0);
}

#[test]
fn group_and_plain_node_description_text_follow_state() {
    let mut group = basic_node("group", "iteration", "Group", "");
    group.size = Size::new(220.0, 160.0);
    let child = WorkflowNode {
        parent_id: Some("group".to_string()),
        ..basic_node("child", "llm", "Child", "")
    };
    let document =
        WorkflowDocument { nodes: vec![group.clone(), child], ..WorkflowDocument::default() };

    assert_eq!(node_description_text(&document, &group), "包含 1 个子节点");

    let empty_group = basic_node("empty-group", "loop", "Loop", "");
    assert_eq!(
        node_description_text(&WorkflowDocument::default(), &empty_group),
        "容器节点，可拖动带走内部节点"
    );

    let described_group = basic_node("described", "loop", "Loop", "自定义描述");
    assert_eq!(node_description_text(&WorkflowDocument::default(), &described_group), "自定义描述");

    assert_eq!(
        node_description_text(
            &WorkflowDocument::default(),
            &basic_node("plain", "llm", "LLM", "节点描述")
        ),
        "节点描述"
    );
    assert!(
        node_description_text(&WorkflowDocument::default(), &basic_node("plain", "llm", "LLM", ""))
            .is_empty()
    );
}

#[test]
fn code_node_description_falls_back_for_invalid_and_unknown_strategies() {
    let document = WorkflowDocument::default();
    let raw_nodes = [
        Value::Null,
        serde_yaml::from_str::<Value>("data: invalid\n").expect("yaml should parse"),
        serde_yaml::from_str::<Value>("data:\n  error_strategy: 42\n").expect("yaml should parse"),
        serde_yaml::from_str::<Value>("data:\n  error_strategy: '   '\n")
            .expect("yaml should parse"),
        serde_yaml::from_str::<Value>("data:\n  error_strategy: retry\n")
            .expect("yaml should parse"),
    ];

    for raw_node in raw_nodes {
        let mut node = code_node(None);
        node.description = "保留原始描述".to_string();
        node.raw_node = raw_node;

        assert_eq!(node_description_text(&document, &node), "保留原始描述");
    }

    let mut trimmed = code_node(None);
    trimmed.description = "不会显示".to_string();
    trimmed.raw_node =
        serde_yaml::from_str::<Value>("data:\n  error_strategy: '  fail-branch  '\n")
            .expect("yaml should parse");
    assert_eq!(node_description_text(&document, &trimmed), "异常时 异常分支");
}

#[test]
fn color_and_coordinate_helpers_clamp_and_transform() {
    let left = Color::from_rgba(0.0, 0.2, 0.4, 0.6);
    let right = Color::from_rgba(1.0, 0.8, 0.6, 0.4);

    assert_eq!(blend(left, right, -1.0), left);
    assert_eq!(blend(left, right, 2.0), right);
    assert_eq!(with_alpha(left, -1.0).a, 0.0);
    assert_eq!(with_alpha(left, 2.0).a, 1.0);
    assert_eq!(canvas_element_scale(0.1), 0.3);
    assert_eq!(canvas_element_scale(1.5), 1.5);

    let pan = Vector::new(10.0, -5.0);
    let screen = screen_from_world(Point::new(20.0, 10.0), pan, 2.0);
    assert_eq!(screen, Point::new(50.0, 15.0));
    assert_eq!(world_from_screen(screen, pan, 2.0), Point::new(20.0, 10.0));
    assert_eq!(world_from_screen(screen, pan, 0.0).x, 400000.0);

    let midpoint = cubic_bezier_point(
        Point::new(0.0, 0.0),
        Point::new(0.0, 10.0),
        Point::new(10.0, 10.0),
        Point::new(10.0, 0.0),
        0.5,
    );
    assert_eq!(midpoint, Point::new(5.0, 7.5));
}

#[test]
fn color_and_bezier_helpers_cover_intermediate_and_endpoint_values() {
    let left = Color::from_rgba(0.0, 0.2, 0.4, 0.6);
    let right = Color::from_rgba(1.0, 0.8, 0.6, 0.4);
    assert_eq!(blend(left, right, 0.5), Color::from_rgba(0.5, 0.5, 0.5, 0.5));
    assert_eq!(with_alpha(left, 0.25), Color::from_rgba(0.0, 0.2, 0.4, 0.25));
    assert_eq!(accent_color("llm"), workflow_node_accent_color("llm"));
    assert_eq!(accent_color("unknown"), workflow_node_accent_color("unknown"));

    let start = Point::new(1.0, 2.0);
    let end = Point::new(9.0, 10.0);
    let c1 = Point::new(3.0, 8.0);
    let c2 = Point::new(7.0, 4.0);
    assert_eq!(cubic_bezier_point(start, c1, c2, end, 0.0), start);
    assert_eq!(cubic_bezier_point(start, c1, c2, end, 1.0), end);
}

#[test]
fn theme_darkness_uses_palette_background() {
    assert!(theme_is_dark(&iced::Theme::Dark));
    assert!(!theme_is_dark(&iced::Theme::Light));
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn export_svg_handles_empty_documents() {
    let svg = export_svg(&WorkflowDocument::default());

    assert!(svg.contains("<svg"));
    assert!(svg.contains("width=\"960\""));
    assert!(svg.contains("rgba(248,250,253,1)"));
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn export_svg_renders_nodes_edges_labels_and_escapes_text() {
    let mut source = basic_node("a", "llm", "A <B> \"Q\" 'S'", "description & more");
    source.size = Size::new(320.0, 84.0);
    source.selected = true;
    let target = basic_node("b", "answer", "", "target");
    let document = WorkflowDocument {
        nodes: vec![source, WorkflowNode { position: Point::new(260.0, 20.0), ..target }],
        edges: vec![edge("a", "b", Some("true"))],
        ..WorkflowDocument::default()
    };

    let svg = export_svg(&document);

    assert!(svg.contains("A &lt;B&gt;"));
    assert!(svg.contains("&quot;Q&quot;"));
    assert!(svg.contains("&apos;S&apos;"));
    assert!(svg.contains("description &amp; more"));
    assert!(svg.contains(">是<"));
    assert!(svg.contains("node-shadow"));
    assert!(svg.ends_with("</svg>"));
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn export_svg_skips_dangling_edges_and_renders_selected_edge_style() {
    let mut later = basic_node("a", "llm", "Later", "");
    later.z_index = 2.0;
    let mut earlier = basic_node("b", "answer", "Earlier", "");
    earlier.position = Point::new(260.0, 20.0);
    earlier.z_index = -1.0;

    let plain_edge = edge("a", "b", None);
    let mut selected_edge = edge("a", "b", Some("false"));
    selected_edge.id = "selected".to_string();
    selected_edge.selected = true;
    selected_edge.z_index = 1.0;
    let document = WorkflowDocument {
        nodes: vec![later, earlier],
        edges: vec![
            edge("missing", "b", None),
            plain_edge,
            edge("a", "missing", None),
            selected_edge,
        ],
        ..WorkflowDocument::default()
    };

    let svg = export_svg(&document);

    assert_eq!(svg.matches("<path ").count(), 2);
    assert!(svg.contains("stroke-width=\"1.60\""));
    assert!(svg.contains("stroke-width=\"2.80\""));
    assert!(svg.contains(">否<"));
    assert!(
        svg.find(">Earlier<").expect("earlier node should render")
            < svg.find(">Later<").expect("later node should render")
    );
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn export_svg_renders_custom_edge_label_and_wrapped_compact_title() {
    let mut source = basic_node("source", "tool", "ABCDEFGHIJKLMNO", "");
    source.size = Size::new(120.0, 60.0);
    let mut target = basic_node("target", "answer", "Target", "");
    target.position = Point::new(240.0, 20.0);
    let document = WorkflowDocument {
        nodes: vec![source, target],
        edges: vec![edge("source", "target", Some("fallback"))],
        ..WorkflowDocument::default()
    };

    let svg = export_svg(&document);

    assert!(svg.contains(">fallback<"));
    assert!(svg.contains(">ABCDEFGH…<"));
    assert!(svg.contains("rgba(148,163,184,0.300)"));
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn export_svg_uses_default_title_and_respects_description_height() {
    let mut short = basic_node("short", "answer", "", "short description hidden");
    short.size = Size::new(180.0, 60.0);
    let mut tall = basic_node("tall", "answer", "Tall", "tall description visible");
    tall.position = Point::new(260.0, 20.0);
    tall.size = Size::new(220.0, 180.0);
    let document = WorkflowDocument { nodes: vec![short, tall], ..WorkflowDocument::default() };

    let svg = export_svg(&document);

    assert!(svg.contains(">回复<"));
    assert!(!svg.contains("short description hidden"));
    assert!(svg.contains("tall description visible"));
    let description_line = svg
        .lines()
        .find(|line| line.contains("tall description visible"))
        .expect("tall description should render");
    assert!(description_line.contains("x=\"310.00\""));
    assert!(svg.contains("font-size=\"13.00\""));
    assert!(svg.contains("font-size=\"12.00\""));
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn export_svg_renders_start_variable_rows_and_type_badges() {
    let raw_node = serde_yaml::from_str::<Value>(
        r#"
data:
  variables:
    - variable: amount
      type: number
    - variable: accepted
      type: checkbox
    - variable: avatar
      type: file
    - variable: attachments
      type: file-list
    - variable: "note<&"
      type: text-input
"#,
    )
    .expect("start variable yaml should parse");
    let mut start = basic_node("start", "start", "", "description should be hidden");
    start.size = Size::new(240.0, 300.0);
    start.raw_node = raw_node;
    let document = WorkflowDocument { nodes: vec![start], ..WorkflowDocument::default() };

    let svg = export_svg(&document);

    assert!(svg.contains(">开始<"));
    assert!(svg.contains(">amount<"));
    assert!(svg.contains(">accepted<"));
    assert!(svg.contains(">avatar<"));
    assert!(svg.contains(">attachments<"));
    assert!(svg.contains(">note&lt;&amp;<"));
    assert!(svg.contains(">#<"));
    assert!(svg.contains(">B<"));
    assert!(svg.contains(">F<"));
    assert!(svg.contains(">T<"));
    assert!(!svg.contains("description should be hidden"));
}

#[cfg(target_arch = "wasm32")]
#[test]
fn export_svg_returns_empty_on_wasm() {
    assert!(export_svg(&WorkflowDocument::default()).is_empty());
}
