//! 助手输出分段解析与行渲染的测试。
//! 覆盖普通文本、完整或未闭合思考块、工具 JSON 标记以及 Todo 卡片的分流规则。
//! 渲染时还校验展开状态、预览内容与逐行元数据始终对齐。

use super::assistant_segments::{
    AssistantSegment, assistant_segments_to_lines, assistant_segments_to_lines_with_meta,
    parse_assistant_segments,
};
use ratatui::text::Line;
use serde_json::json;
use std::collections::BTreeSet;

fn line_text(line: &Line<'_>) -> String {
    line.spans.iter().map(|span| span.content.as_ref()).collect::<String>()
}

#[test]
fn parses_empty_input_and_preserves_plain_text_lines() {
    assert!(parse_assistant_segments("").is_empty());

    let segments = parse_assistant_segments("first\n\n second ");

    assert_eq!(segments.len(), 3);
    let AssistantSegment::Text(first) = &segments[0] else {
        panic!("expected first line to remain text");
    };
    let AssistantSegment::Text(blank) = &segments[1] else {
        panic!("expected blank line to remain text");
    };
    let AssistantSegment::Text(second) = &segments[2] else {
        panic!("expected final line to remain text");
    };
    assert_eq!(first, "first");
    assert_eq!(blank, "");
    assert_eq!(second, " second ");
}

#[test]
fn parses_closed_and_unclosed_think_blocks() {
    let segments = parse_assistant_segments(
        "before\n  <think mode=\"deep\">\nreason\n</think>\nafter\n<think>\nstill working",
    );

    assert_eq!(segments.len(), 4);
    let AssistantSegment::Think { content, open } = &segments[1] else {
        panic!("expected a closed think block");
    };
    assert_eq!(content, "  <think mode=\"deep\">\nreason\n</think>");
    assert!(!open);

    let AssistantSegment::Think { content, open } = &segments[3] else {
        panic!("expected an unclosed think block");
    };
    assert_eq!(content, "<think>\nstill working");
    assert!(open);
}

#[test]
fn parses_multiline_tool_json_and_leaves_following_text_separate() {
    let segments = parse_assistant_segments(
        "tool read\n{\n  \"filePath\": \"/tmp/demo.rs\",\n  \"offset\": 3,\n  \"limit\": 2\n}\ntail",
    );

    assert_eq!(segments.len(), 2);
    let AssistantSegment::Tool { name, summary, input_raw } = &segments[0] else {
        panic!("expected a parsed tool segment");
    };
    assert_eq!(name, "read");
    assert!(summary.contains("demo.rs"));
    assert!(summary.contains("line 3-4"));
    assert_eq!(
        input_raw,
        "{\n  \"filePath\": \"/tmp/demo.rs\",\n  \"offset\": 3,\n  \"limit\": 2\n}"
    );
    let AssistantSegment::Text(tail) = &segments[1] else {
        panic!("expected trailing text after the tool payload");
    };
    assert_eq!(tail, "tail");
}

#[test]
fn keeps_tool_markers_as_text_when_name_or_json_is_invalid() {
    let segments = parse_assistant_segments("tool   \ntool bash\nnot json\nplain");

    assert_eq!(segments.len(), 4);
    for (segment, expected) in segments.iter().zip(["tool   ", "tool bash", "not json", "plain"]) {
        let AssistantSegment::Text(text) = segment else {
            panic!("invalid tool syntax must fall back to text");
        };
        assert_eq!(text, expected);
    }
}

#[test]
fn accepts_tool_json_at_the_line_limit_and_rejects_payloads_beyond_it() {
    let mut accepted_input = vec!["tool custom".to_string(), "{".to_string()];
    accepted_input.extend(std::iter::repeat_n(" ".to_string(), 62));
    accepted_input.push("}".to_string());

    let accepted = parse_assistant_segments(&accepted_input.join("\n"));
    assert_eq!(accepted.len(), 1);
    let AssistantSegment::Tool { input_raw, .. } = &accepted[0] else {
        panic!("a JSON payload of 64 lines must be accepted");
    };
    assert_eq!(input_raw.lines().count(), 64);

    let mut rejected_input = vec!["tool custom".to_string(), "{".to_string()];
    rejected_input.extend(std::iter::repeat_n(" ".to_string(), 63));
    rejected_input.push("}".to_string());

    let rejected = parse_assistant_segments(&rejected_input.join("\n"));
    assert_eq!(rejected.len(), 66);
    assert!(rejected.iter().all(|segment| matches!(segment, AssistantSegment::Text(_))));
}

#[test]
fn renders_text_and_collapsed_or_expanded_think_cards() {
    let content = "hello\n<think>\nalpha\n\nbeta\n</think>";
    let collapsed = assistant_segments_to_lines(parse_assistant_segments(content), false, false);
    let expanded = assistant_segments_to_lines(parse_assistant_segments(content), false, true);

    assert_eq!(line_text(&collapsed[0]), "hello");
    assert_eq!(collapsed.len(), 3);
    assert!(line_text(&collapsed[1]).contains("思考 · 5 行"));
    assert_eq!(line_text(&collapsed[2]), "  alpha");

    assert_eq!(expanded.len(), 5);
    assert_eq!(line_text(&expanded[2]), "  alpha");
    assert_eq!(line_text(&expanded[3]), "  alpha");
    assert_eq!(line_text(&expanded[4]), "  beta");
}

#[test]
fn renders_malformed_open_think_status_and_ignores_empty_tag_only_lines() {
    let lines = assistant_segments_to_lines(
        parse_assistant_segments("<think mode=\"deep\"\n\nworking"),
        false,
        true,
    );

    assert_eq!(lines.len(), 3);
    assert!(line_text(&lines[0]).starts_with("思考中 · 3 行"));
    assert_eq!(line_text(&lines[1]), "  working");
    assert_eq!(line_text(&lines[2]), "  working");
}

#[test]
fn renders_tag_only_think_without_an_empty_preview() {
    let lines =
        assistant_segments_to_lines(parse_assistant_segments("<think></think>"), false, true);

    assert_eq!(lines.len(), 1);
    assert_eq!(line_text(&lines[0]), "思考 · 1 行");
}

#[test]
fn renders_todowrite_preview_counts_and_todoread_fallback() {
    let todos = json!({
        "input": {
            "todos": [
                {"status": "completed", "content": "done"},
                {"status": "in_progress", "content": "running"},
                {"status": "pending", "content": "next"},
                {"status": "pending", "content": "later"}
            ]
        }
    })
    .to_string();
    let write_lines = assistant_segments_to_lines(
        vec![AssistantSegment::Tool {
            name: "todowrite".to_string(),
            summary: "sync plan".to_string(),
            input_raw: todos,
        }],
        false,
        false,
    );

    assert_eq!(write_lines.len(), 7);
    assert!(line_text(&write_lines[0]).contains("任务更新 · 4 项 · sync plan"));
    assert!(line_text(&write_lines[1]).contains("✓1"));
    assert!(line_text(&write_lines[1]).contains("·1"));
    assert!(line_text(&write_lines[1]).contains("○2"));
    assert_eq!(line_text(&write_lines[5]), "  ... +1 more");
    assert_eq!(line_text(&write_lines[6]), "  Ctrl+T 展开详情");

    let read_lines = assistant_segments_to_lines(
        vec![AssistantSegment::Tool {
            name: "todoread".to_string(),
            summary: String::new(),
            input_raw: "{}".to_string(),
        }],
        true,
        false,
    );
    assert_eq!(read_lines.len(), 3);
    assert_eq!(line_text(&read_lines[0]), "[T] 任务列表 · 0 项");
    assert_eq!(line_text(&read_lines[2]), "  Ctrl+T 收起详情");
}

#[test]
fn routes_non_todo_tools_to_the_standard_tool_card() {
    let lines = assistant_segments_to_lines(
        vec![AssistantSegment::Tool {
            name: "bash".to_string(),
            summary: "run command".to_string(),
            input_raw: json!({
                "status": "completed",
                "command": "echo ok",
                "workdir": "/tmp/project"
            })
            .to_string(),
        }],
        false,
        false,
    );

    assert_eq!(lines.len(), 2);
    assert!(line_text(&lines[0]).contains("运行"));
    assert!(line_text(&lines[0]).contains("已完成"));
    assert!(line_text(&lines[1]).contains("命令 echo ok"));
}

#[test]
fn keeps_text_and_tool_metadata_empty_and_aligned() {
    let segments = vec![
        AssistantSegment::Text("lead".to_string()),
        AssistantSegment::Tool {
            name: "todowrite".to_string(),
            summary: String::new(),
            input_raw: json!({
                "input": {"todos": [{"status": "completed", "content": "done"}]}
            })
            .to_string(),
        },
        AssistantSegment::Tool {
            name: "todoread".to_string(),
            summary: String::new(),
            input_raw: json!({
                "output": [{"status": "pending", "content": "next"}]
            })
            .to_string(),
        },
        AssistantSegment::Tool {
            name: "bash".to_string(),
            summary: "run command".to_string(),
            input_raw: json!({"status": "completed", "command": "echo ok"}).to_string(),
        },
    ];

    let (lines, meta) =
        assistant_segments_to_lines_with_meta(segments, false, false, &BTreeSet::new(), None, 3);

    assert_eq!(lines.len(), meta.len());
    assert!(meta.iter().all(Option::is_none));
    assert_eq!(line_text(&lines[0]), "lead");
    assert!(lines.iter().any(|line| line_text(line).contains("任务更新 · 1 项")));
    assert!(lines.iter().any(|line| line_text(line).contains("任务列表 · 1 项")));
    assert!(lines.iter().any(|line| line_text(line).contains("命令 echo ok")));
}

#[test]
fn aligns_think_metadata_and_applies_per_block_overrides() {
    let content = "plain\n<think>closed</think>\n<think>\nopen";
    let (default_lines, default_meta) = assistant_segments_to_lines_with_meta(
        parse_assistant_segments(content),
        false,
        false,
        &BTreeSet::new(),
        Some(12),
        41,
    );

    assert_eq!(default_lines.len(), default_meta.len());
    assert_eq!(line_text(&default_lines[0]), "plain");
    assert!(default_meta[0].is_none());
    assert!(line_text(&default_lines[1]).contains("12 秒"));

    let closed_meta = default_meta[1].expect("closed think block must have metadata");
    assert!(!closed_meta.open);
    assert_eq!(default_meta[2], Some(closed_meta));

    let open_meta = default_meta[3].expect("open think block must have metadata");
    assert!(open_meta.open);
    assert_ne!(closed_meta.id, open_meta.id);
    assert!(default_meta[3..].iter().all(|meta| *meta == Some(open_meta)));

    let overrides = BTreeSet::from([closed_meta.id, open_meta.id]);
    let (overridden_lines, overridden_meta) = assistant_segments_to_lines_with_meta(
        parse_assistant_segments(content),
        false,
        false,
        &overrides,
        None,
        41,
    );

    assert_eq!(overridden_lines.len(), overridden_meta.len());
    assert_eq!(overridden_lines.len(), default_lines.len());
    assert_eq!(line_text(&overridden_lines[4]), "思考中 · 2 行");
    assert_eq!(overridden_meta[1], Some(closed_meta));
    assert_eq!(overridden_meta[4], Some(open_meta));
}

#[test]
fn global_think_expansion_overrides_closed_default_state() {
    let (lines, meta) = assistant_segments_to_lines_with_meta(
        parse_assistant_segments("<think>detail</think>"),
        false,
        true,
        &BTreeSet::new(),
        None,
        7,
    );

    assert_eq!(lines.len(), 3);
    assert_eq!(line_text(&lines[1]), "  detail");
    assert_eq!(line_text(&lines[2]), "  detail");
    assert_eq!(meta.len(), lines.len());
    assert!(meta.iter().all(Option::is_some));
}
