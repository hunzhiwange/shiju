//! 通用工具卡片的摘要提取与渲染测试。
//! 覆盖路径压缩、搜索、Shell、浏览器、补丁和 Todo 等工具的主次详情选择。
//! 降级逻辑必须能处理稀疏或畸形输入，并忽略敏感或过度冗长的通用字段。

use super::{render_tool_card, tool_summary_cli, toolcards::compact_cli_path};
use ratatui::text::Line;
use serde_json::json;

fn line_text(line: &Line<'_>) -> String {
    line.spans.iter().map(|span| span.content.as_ref()).collect::<String>()
}

fn card_text(name: &str, summary: &str, input: serde_json::Value) -> Vec<String> {
    render_tool_card(name, summary, &input.to_string(), false).iter().map(line_text).collect()
}

#[test]
fn compact_cli_path_keeps_short_paths_and_retains_long_path_suffixes() {
    assert_eq!(compact_cli_path("/tmp/a.txt", 32), "/tmp/a.txt");

    let unix = compact_cli_path("/very/long/path/to/project/src/main.rs", 18);
    assert!(unix.starts_with(".../"));
    assert!(unix.ends_with("src/main.rs"));

    let windows = compact_cli_path(r"C:\very\long\project\src\lib.rs", 16);
    assert!(windows.starts_with(".../"));
    assert!(windows.ends_with("lib.rs"));
}

#[test]
fn compact_cli_path_handles_separator_only_and_zero_length_limits() {
    assert_eq!(compact_cli_path("//////", 3), "///…");
    assert_eq!(compact_cli_path("/long/path", 0), "");
}

#[test]
fn tool_summary_cli_truncates_plain_text_and_malformed_json() {
    let plain = "x".repeat(121);
    let malformed = format!("{{{}", "y".repeat(120));

    assert_eq!(tool_summary_cli("bash", &plain), format!("{}…", "x".repeat(120)));
    assert_eq!(tool_summary_cli("bash", &malformed), format!("{{{}…", "y".repeat(119)));
}

#[test]
fn tool_summary_cli_describes_read_ranges_and_clamps_invalid_bounds() {
    assert_eq!(
        tool_summary_cli("read", &json!({"filePath": "/tmp/demo.rs"}).to_string()),
        "/tmp/demo.rs  line 1-2000"
    );
    assert_eq!(
        tool_summary_cli("read", &json!({"offset": -3, "limit": 0}).to_string()),
        "line 1-1"
    );
    assert_eq!(
        tool_summary_cli("read", &json!({"offset": i64::MAX, "limit": 2}).to_string()),
        format!("line {}-{}", i64::MAX, i64::MAX)
    );
}

#[test]
fn tool_summary_cli_combines_search_patterns_and_paths() {
    assert_eq!(
        tool_summary_cli("grep", &json!({"path": "/workspace", "query": "needle"}).to_string()),
        "/workspace  pattern=needle"
    );
    assert_eq!(tool_summary_cli("content_search", "{}"), "grep");
    assert_eq!(
        tool_summary_cli("glob", &json!({"glob": "**/*.rs"}).to_string()),
        "pattern=**/*.rs"
    );
    assert_eq!(
        tool_summary_cli(
            "glob_search",
            &json!({"path": "/workspace", "pattern": "src/**"}).to_string()
        ),
        "/workspace  pattern=src/**"
    );
}

#[test]
fn tool_summary_cli_covers_all_code_search_query_path_combinations() {
    assert_eq!(tool_summary_cli("lsp", "{}"), "workspace search");
    assert_eq!(tool_summary_cli("lsp", &json!({"symbol": "Widget"}).to_string()), "Widget");
    assert_eq!(
        tool_summary_cli("codesearch", &json!({"includePattern": "src/**"}).to_string()),
        "src/**"
    );
    assert_eq!(
        tool_summary_cli(
            "codesearch",
            &json!({"lineContent": "render", "filePath": "src/view.rs"}).to_string()
        ),
        "src/view.rs  render"
    );
}

#[test]
fn tool_summary_cli_prioritizes_shell_description_and_workdir() {
    assert_eq!(
        tool_summary_cli(
            "bash",
            &json!({"description": "List files", "workdir": "/tmp/project"}).to_string()
        ),
        "List files  /tmp/project"
    );
    assert_eq!(
        tool_summary_cli("shell", &json!({"description": "List files"}).to_string()),
        "List files"
    );
    assert_eq!(
        tool_summary_cli("bash", &json!({"workdir": "/tmp/project"}).to_string()),
        "工作目录 /tmp/project"
    );
    assert_eq!(tool_summary_cli("shell", "{}"), "bash command");
}

#[test]
fn tool_summary_cli_covers_browser_action_target_combinations() {
    assert_eq!(
        tool_summary_cli("browser", &json!({"action": "click", "selector": "#submit"}).to_string()),
        "click #submit"
    );
    assert_eq!(
        tool_summary_cli("browser_open", &json!({"action": "reload"}).to_string()),
        "reload"
    );
    assert_eq!(
        tool_summary_cli("browser", &json!({"url": "https://example.com"}).to_string()),
        "https://example.com"
    );
    assert_eq!(tool_summary_cli("browser_open", "{}"), "browser");
}

#[test]
fn tool_summary_cli_counts_patch_operations_and_handles_empty_patches() {
    let patch = concat!(
        "*** Add File: src/new.rs\n",
        "*** Update File: src/lib.rs\n",
        "*** Delete File: src/old.rs\n",
        "*** End Patch"
    );

    assert_eq!(
        tool_summary_cli("apply_patch", &json!({"patchText": patch}).to_string()),
        "3 项 · 新增 1 · 更新 1 · 删除 1"
    );
    assert_eq!(tool_summary_cli("apply_patch", "{}"), "patch");
}

#[test]
fn tool_summary_cli_parses_nested_todo_updates_and_rejects_invalid_nested_input() {
    let write = json!({"input": json!({"todos": [{}, {}], "merge": false}).to_string()});
    let merge = json!({"input": json!({"todos": [{}], "merge": true}).to_string()});

    assert_eq!(tool_summary_cli("todowrite", &write.to_string()), "todos=2 action=write");
    assert_eq!(tool_summary_cli("todowrite", &merge.to_string()), "todos=1 action=merge");
    assert_eq!(tool_summary_cli("todowrite", &json!({"input": "not json"}).to_string()), "todos");
    assert_eq!(tool_summary_cli("todoread", "{}"), "todo list");
}

#[test]
fn tool_summary_cli_uses_generic_priority_path_and_shape_fallbacks() {
    assert_eq!(
        tool_summary_cli("custom", &json!({"selector": "  ", "target": "dialog"}).to_string()),
        "dialog"
    );
    assert_eq!(
        tool_summary_cli("custom", &json!({"filePath": " ", "path": "/tmp/a"}).to_string()),
        "/tmp/a"
    );
    assert_eq!(tool_summary_cli("custom", &json!({"alpha": 1, "beta": 2}).to_string()), "2 项参数");
    assert_eq!(tool_summary_cli("custom", "[]"), "工具");
}

#[test]
fn render_tool_card_maps_titles_emojis_and_statuses() {
    let cases = [
        ("read", "📖 读取已完成", "completed"),
        ("write", "✏️ 写入进行中", "running"),
        ("apply_patch", "📝 补丁进行中", "in_progress"),
        ("grep", "🔍 搜索失败", "error"),
        ("bash", "⚡ 运行已拒绝", "denied"),
        ("webfetch", "🌐 抓取", "unknown"),
        ("question", "❓ 提问", "unknown"),
        ("task", "📋 任务", "unknown"),
        ("todowrite", "✅ 工具", "unknown"),
        ("custom", "🔧 工具", "unknown"),
    ];

    for (name, expected, status) in cases {
        let card = card_text(name, "", json!({"status": status}));
        assert!(card[0].contains(expected), "unexpected header for {name}: {}", card[0]);
    }
}

#[test]
fn render_tool_card_omits_unknown_or_non_string_statuses() {
    let unknown = card_text("bash", "", json!({"status": "queued"}));
    let non_string = card_text("bash", "", json!({"status": 1}));

    assert_eq!(unknown[0], "⚡ 运行");
    assert_eq!(non_string[0], "⚡ 运行");
}

#[test]
fn render_tool_card_selects_each_tools_primary_detail() {
    let cases = [
        ("read", json!({"filePath": "/tmp/demo.rs", "offset": 3, "limit": 2}), "路径 /tmp/demo.rs"),
        ("read", json!({"offset": 3, "limit": 2}), "范围 line 3-4"),
        ("write", json!({"path": "/tmp/out.rs"}), "路径 /tmp/out.rs"),
        (
            "apply_patch",
            json!({"patchText": "*** Update File: src/lib.rs"}),
            "变更 1 项 · 新增 0 · 更新 1 · 删除 0",
        ),
        ("grep", json!({"path": "/workspace", "pattern": "needle"}), "模式 needle"),
        ("glob_search", json!({"path": "/workspace"}), "目录 /workspace"),
        ("lsp", json!({"query": "Widget", "filePath": "src/lib.rs"}), "查询 Widget"),
        ("shell", json!({"description": "List", "command": "ls", "workdir": "/tmp"}), "命令 ls"),
        ("browser", json!({"action": "click", "selector": "#ok"}), "目标 #ok"),
        (
            "webfetch",
            json!({"url": "https://example.com", "format": "text"}),
            "链接 https://example.com",
        ),
        (
            "question",
            json!({
                "questions": [{
                    "header": "Choice",
                    "question": "Continue?",
                    "options": [{}, {}]
                }]
            }),
            "问题 Continue?",
        ),
        (
            "task",
            json!({"description": "Inspect code", "command": "scan", "prompt": "first\nsecond"}),
            "任务 Inspect code",
        ),
    ];

    for (name, input, expected) in cases {
        let card = card_text(name, "summary", input);
        assert_eq!(card.len(), 2, "missing detail for {name}");
        assert!(card[1].contains(expected), "unexpected detail for {name}: {}", card[1]);
    }
}

#[test]
fn render_tool_card_uses_sparse_secondary_details_as_fallbacks() {
    let cases = [
        ("grep", json!({"include": "*.rs"}), "文件 *.rs"),
        ("bash", json!({"description": "List files"}), "说明 List files"),
        ("bash", json!({"workdir": "/tmp"}), "目录 /tmp"),
        ("bash", json!({"timeout": 250}), "超时 250 ms"),
        ("browser_open", json!({"action": "reload"}), "动作 reload"),
        ("webfetch", json!({"format": "markdown"}), "格式 markdown"),
        ("question", json!({"questions": [{"header": "Choice"}]}), "标题 Choice"),
        ("task", json!({"subagent_type": "explore"}), "代理 explore"),
    ];

    for (name, input, expected) in cases {
        let card = card_text(name, "", input);
        assert_eq!(card.len(), 2, "missing sparse detail for {name}");
        assert_eq!(card[1], format!("  {expected}"));
    }
}

#[test]
fn render_tool_card_uses_first_available_fallback_detail() {
    let task = card_text("task", "", json!({"prompt": "  first line  \nsecond line"}));
    let generic = card_text("custom", "", json!({"enabled": true}));

    assert_eq!(task[1], "  提示 first line");
    assert_eq!(generic[1], "  enabled true");
}

#[test]
fn render_tool_card_summarizes_generic_scalar_shapes() {
    let cases = [
        (json!({"value": " text "}), "value text"),
        (json!({"value": false}), "value false"),
        (json!({"value": 42}), "value 42"),
        (json!({"value": [1, 2]}), "value 2 项"),
        (json!({"value": {"a": 1}}), "value 1 个字段"),
    ];

    for (input, expected) in cases {
        let card = card_text("custom", "", input);
        assert_eq!(card.len(), 2);
        assert!(card[1].contains(expected), "unexpected generic detail: {}", card[1]);
    }

    assert_eq!(card_text("custom", "", json!({"value": null})).len(), 1);
}

#[test]
fn render_tool_card_ignores_sensitive_or_verbose_generic_fields() {
    let card = card_text(
        "custom",
        "",
        json!({
            "patchText": "secret patch",
            "command": "secret command",
            "prompt": "secret prompt",
            "input": "secret input",
            "operations": ["secret operation"]
        }),
    );

    assert_eq!(card, vec!["🔧 工具"]);
}

#[test]
fn render_tool_card_handles_malformed_input_without_details() {
    let empty_summary = render_tool_card("read", "", "{not json", false);
    let with_summary = render_tool_card("read", "overview", "{not json", false);

    assert_eq!(empty_summary.len(), 1);
    assert_eq!(with_summary.len(), 1);
    assert!(line_text(&with_summary[0]).contains(" · overview"));
}

#[test]
fn render_tool_card_currently_ignores_expand_tool_details() {
    let input = json!({"command": "cargo test", "workdir": "/tmp/project"}).to_string();
    let collapsed = render_tool_card("bash", "测试命令", &input, false);
    let expanded = render_tool_card("bash", "测试命令", &input, true);

    assert_eq!(
        collapsed.iter().map(line_text).collect::<Vec<_>>(),
        expanded.iter().map(line_text).collect::<Vec<_>>()
    );
}

#[test]
fn render_tool_card_reports_empty_question_collection() {
    let card = card_text("question", "", json!({"questions": []}));

    assert_eq!(card, vec!["❓ 提问", "  问题数 0 项"]);
}
