use super::tool_summaries::{
    collect_tool_card_texts, count_code_blocks, explore_summary_text_blocks,
    normalized_visible_text, should_hide_explore_link_box, should_hide_post_explore_tool_block,
    special_text_blocks, summarize_explore_items, tool_card_text_blocks,
    trailing_tool_tail_text_source_block_idx,
};
use crate::app::models::ParsedChatBlock;

fn tool_card(tool_name: &str, value: serde_json::Value) -> Vec<String> {
    collect_tool_card_texts(&format!("tool {tool_name}\n{value}")).expect("tool card text")
}

#[test]
fn count_code_blocks_counts_fenced_pairs() {
    assert_eq!(count_code_blocks("```rust\nfn main() {}\n```\n```text\nx\n```"), 2);
}

#[test]
fn normalized_visible_text_filters_empty_or_trace_like_text() {
    assert_eq!(normalized_visible_text("  visible  "), Some("visible".to_string()));
    assert_eq!(normalized_visible_text(""), None);
}

#[test]
fn special_text_blocks_keep_text_after_tool() {
    let raw = "tool bash\n{\"status\":\"completed\",\"output\":\"ok\"}\nfinal answer";

    assert_eq!(special_text_blocks(raw), vec!["final answer".to_string()]);
    assert_eq!(tool_card_text_blocks(raw).len(), 1);
}

#[test]
fn explore_link_box_hide_rules_are_explicit() {
    assert!(should_hide_explore_link_box("- src/main.rs\n- src/lib.rs"));
    assert!(should_hide_explore_link_box(
        "1. [main.rs](file:///tmp/main.rs)\nopen: file:///tmp/lib.rs"
    ));
    assert!(!should_hide_explore_link_box("summary text"));
    assert!(!should_hide_post_explore_tool_block("plain text"));
}

#[test]
fn collect_tool_card_texts_rejects_malformed_raw_blocks() {
    assert!(collect_tool_card_texts("plain text").is_none());
    assert!(collect_tool_card_texts("tool bash\nnot-json").is_none());
}

#[test]
fn tool_card_text_blocks_cover_todo_and_browser_previews() {
    let todo = tool_card_text_blocks(
        "tool todowrite\n{\"input\":\"{\\\"todos\\\":[{\\\"content\\\":\\\"done\\\",\\\"status\\\":\\\"completed\\\"},{\\\"content\\\":\\\"next\\\",\\\"status\\\":\\\"pending\\\"}]}\"}",
    );
    assert_eq!(todo, vec![vec!["✓ done\n○ next".to_string()]]);

    let browser = tool_card_text_blocks(
        "tool browser_open\n{\"result\":{\"data\":{\"browser\":\"Chrome\",\"url\":\"https://example.test\"}},\"status\":\"completed\",\"output\":\"opened\"}",
    );
    assert_eq!(browser, vec![vec!["Chrome · https://example.test".to_string()]]);
}

#[test]
fn tool_card_text_blocks_cover_advanced_tool_previews() {
    let blocks = tool_card_text_blocks(
        "tool tool_search\n{\"result\":{\"data\":{\"items\":[{\"display_name\":\"GitHub\",\"reason\":\"repo access\"},{\"id\":\"slack\",\"reason\":\"messages\"}]}},\"status\":\"completed\",\"output\":\"matches\"}",
    );

    assert!(blocks[0][0].starts_with("状态:"));
    assert!(blocks[0].contains(&"GitHub: repo access".to_string()));
    assert!(blocks[0].contains(&"slack: messages".to_string()));
}

#[test]
fn normalized_visible_text_strips_internal_traces() {
    assert_eq!(
        normalized_visible_text("tool bash({\"command\":\"pwd\"})\nfinal"),
        Some("final".to_string())
    );
}

#[test]
fn trailing_tool_tail_text_source_block_idx_requires_trailing_explore_tool() {
    let blocks = vec![
        ParsedChatBlock::Text { content: "summary".to_string() },
        ParsedChatBlock::Tool {
            raw: "tool read\n{\"input\":\"{\\\"filePath\\\":\\\"/tmp/a.rs\\\"}\",\"status\":\"completed\"}"
                .to_string(),
        },
        ParsedChatBlock::Text { content: "\n".to_string() },
    ];

    assert_eq!(trailing_tool_tail_text_source_block_idx(&blocks), Some(0));

    let no_explore = vec![
        ParsedChatBlock::Text { content: "summary".to_string() },
        ParsedChatBlock::Tool {
            raw: "tool bash\n{\"input\":\"pwd\",\"status\":\"completed\"}".to_string(),
        },
    ];
    assert_eq!(trailing_tool_tail_text_source_block_idx(&no_explore), None);

    let no_source_text = vec![ParsedChatBlock::Tool {
        raw: "tool read\n{\"input\":\"src/lib.rs\",\"status\":\"completed\"}".to_string(),
    }];
    assert_eq!(trailing_tool_tail_text_source_block_idx(&no_source_text), None);
}

#[test]
fn tool_card_texts_cover_bash_and_read_summaries() {
    assert_eq!(
        tool_card("bash", serde_json::json!({"input":"{\"command\":\"   \"}"})),
        vec!["运行命令".to_string()]
    );
    assert_eq!(
        tool_card(
            "read",
            serde_json::json!({
                "input":"{\"filePath\":\"file:///tmp/project/src/lib.rs\",\"offset\":2,\"limit\":3}"
            })
        ),
        vec!["lib.rs".to_string(), "offset=2 limit=3 (line 3-5)".to_string()]
    );
    assert_eq!(
        tool_card("pdf_read", serde_json::json!({"input":"docs/report.pdf"})),
        vec!["report.pdf".to_string()]
    );
}

#[test]
fn bash_tool_card_summary_stays_on_one_line() {
    let command = "cat > /tmp/a.json <<'EOF'\n{\n  \"key\": \"value\"\n}\nEOF";

    assert_eq!(
        tool_card("bash", serde_json::json!({"input": command})),
        vec!["cat > /tmp/a.json <<'EOF' { \"key\": \"value\" } EOF".to_string()]
    );
}

#[test]
fn read_summaries_describe_single_sided_ranges() {
    assert_eq!(
        tool_card("read", serde_json::json!({"input":"{\"path\":\"src/main.rs\",\"offset\":4}"})),
        vec!["main.rs".to_string(), "offset=4 (from line 5)".to_string()]
    );
    assert_eq!(
        tool_card(
            "file_read",
            serde_json::json!({"input":"{\"file_path\":\"src/lib.rs\",\"limit\":3}"})
        ),
        vec!["lib.rs".to_string(), "limit=3 (line 1-3)".to_string()]
    );
}

#[test]
fn post_explore_tool_block_hides_valid_file_link_output() {
    let raw = "tool read\n{\"output\":\"Files:\\n- [main.rs](file:///tmp/main.rs)\"}";

    assert!(should_hide_post_explore_tool_block(raw));
    assert!(!should_hide_post_explore_tool_block("tool read\nnot-json"));
}

#[test]
fn tool_card_texts_cover_file_preview_fallbacks() {
    let malformed_inner_json = "{\"content\":\"alpha\nbeta\"}";
    assert_eq!(
        tool_card(
            "file_write",
            serde_json::json!({"status":"completed","input":malformed_inner_json})
        ),
        vec!["alpha\nbeta".to_string()]
    );
    assert_eq!(
        tool_card(
            "notebook_edit",
            serde_json::json!({"input":"{\"newCode\":[\"print(1)\",\"print(2)\"]}"})
        ),
        vec!["print(1)\nprint(2)".to_string()]
    );
}

#[test]
fn tool_card_texts_cover_apply_patch_diff_sources() {
    let structured = serde_json::json!({
        "result": {
            "content": [
                {
                    "type": "structured_patch",
                    "hunks": [
                        {
                            "path": "src/lib.rs",
                            "header": "@@ -1 +1 @@",
                            "lines": ["-old", "+new"]
                        }
                    ]
                }
            ]
        }
    });
    let structured_texts = tool_card("apply_patch", structured);
    assert!(structured_texts[0].contains("--- a/src/lib.rs"));
    assert!(structured_texts[0].contains("+new"));

    let output_diff = "<diff>\n--- a/a.rs\n+++ b/a.rs\n@@ -1 +1 @@\n-old\n+new\n</diff>";
    assert_eq!(
        tool_card("apply_patch", serde_json::json!({"status":"completed","output":output_diff})),
        vec!["--- a/a.rs\n+++ b/a.rs\n@@ -1 +1 @@\n-old\n+new".to_string()]
    );
}

#[test]
fn tool_card_texts_cover_search_web_agent_and_unknown_branches() {
    assert_eq!(
        tool_card("grep", serde_json::json!({"status":"error","error":"pattern failed"})),
        vec!["pattern failed".to_string()]
    );
    assert_eq!(
        tool_card("web_search", serde_json::json!({"summary":"web summary"})),
        vec!["web summary".to_string()]
    );
    assert_eq!(
        tool_card("AgentTool", serde_json::json!({"output":"{\"message\":\"delegate done\"}"})),
        vec!["delegate done".to_string()]
    );
    assert_eq!(
        tool_card("unknown_tool", serde_json::json!({"status":"error","output":"bad path"})),
        vec!["bad path".to_string()]
    );
}

#[test]
fn tool_card_texts_cover_browser_and_advanced_tool_fallbacks() {
    assert_eq!(
        tool_card(
            "browser",
            serde_json::json!({
                "output":"fallback",
                "result":{"data":{"result":{"title":"Page","url":"https://example.test","text":"hello","path":"/tmp/page.html"}}}
            })
        ),
        vec![
            "Page".to_string(),
            "https://example.test".to_string(),
            "hello".to_string(),
            "/tmp/page.html".to_string()
        ]
    );

    let planned = tool_card("mcp_demo", serde_json::json!({}));
    assert_eq!(
        planned,
        vec!["状态: planned".to_string(), "mcp_* 集成 当前已明确标记为 planned。".to_string()]
    );
}

#[test]
fn tool_card_texts_cover_browser_open_sparse_fields() {
    assert_eq!(
        tool_card(
            "browser_open",
            serde_json::json!({"output":"opened","result":{"data":{"browser":"Chrome"}}})
        ),
        vec!["已在 Chrome 中打开页面".to_string()]
    );
    assert_eq!(
        tool_card(
            "browser_open",
            serde_json::json!({"output":"opened","result":{"data":{"url":"https://example.test"}}})
        ),
        vec!["https://example.test".to_string()]
    );
    assert_eq!(
        tool_card("browser_open", serde_json::json!({"output":"opened","result":{"data":{}}})),
        vec!["opened".to_string()]
    );

    let available = tool_card("tool_search", serde_json::json!({}));
    assert_eq!(available[0], "状态: available");
    assert!(available[1].contains("已接入当前会话工具面"));
}

#[test]
fn tool_card_texts_cover_todoread_and_question_edges() {
    assert_eq!(
        tool_card("todoread", serde_json::json!({"output":"[]"})),
        vec!["暂无任务".to_string()]
    );
    assert_eq!(
        tool_card("todoread", serde_json::json!({"output":"not json"})),
        vec!["not json".to_string()]
    );
    assert_eq!(
        tool_card(
            "question",
            serde_json::json!({"summary":"need input","status":"error","error":"declined"})
        ),
        vec!["need input".to_string(), "declined".to_string()]
    );
}

#[test]
fn explore_summaries_dedupe_and_mark_running_groups() {
    let first_read = r#"tool read
{"tool_call_id":"same","status":"completed","input":"{\"filePath\":\"src/old.rs\"}"}"#;
    let replacement_read = r#"tool read
{"tool_call_id":"same","status":"running","input":"{\"filePath\":\"src/new.rs\"}"}"#;
    let grep = r#"tool grep
{"status":"completed","input":"{\"pattern\":\"main\"}"}"#;
    let list = r#"tool ls
{"status":"completed","input":"src"}"#;
    let bash = r#"tool bash
{"status":"completed","input":"pwd"}"#;

    let summary = summarize_explore_items(
        [first_read, replacement_read, grep, list, bash].into_iter(),
        1,
        false,
    )
    .expect("explore summary");

    assert_eq!(summary.1, "1 次读取，1 次搜索，1 次列出，1 次其他");

    let forced =
        summarize_explore_items([grep].into_iter(), 0, true).expect("forced running summary");
    let completed =
        summarize_explore_items([grep].into_iter(), 0, false).expect("completed summary");
    assert_eq!(forced.1, "1 次搜索");
    assert!(forced.0 < completed.0);
}

#[test]
fn explore_summary_text_blocks_split_on_visible_boundaries() {
    let raw = "tool read\n{\"status\":\"completed\",\"input\":\"src/lib.rs\"}\n<think>hidden</think>\ntool grep\n{\"status\":\"running\",\"input\":\"main\"}\nvisible\ntool ls\n{\"status\":\"completed\",\"input\":\"src\"}";

    let hidden_think = explore_summary_text_blocks(raw, false);
    assert_eq!(hidden_think.len(), 2);
    assert_eq!(hidden_think[0].1, "1 次读取，1 次搜索");
    assert_eq!(hidden_think[1].1, "1 次列出");

    let shown_think = explore_summary_text_blocks(raw, true);
    assert_eq!(shown_think.len(), 3);
    assert_eq!(shown_think[0].1, "1 次读取");
    assert_eq!(shown_think[1].1, "1 次搜索");
    assert_eq!(shown_think[2].1, "1 次列出");
}
