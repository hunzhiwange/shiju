use super::tool_presentation::{ToolVisualState, ToolVisualTone, tool_inline_summary, tool_inline_summary_unbounded, tool_verb, tool_visual_state};
use desktop_assets::assets::Icon;
use iced::Theme;
#[test]
fn unbounded_inline_summary_preserves_full_tool_detail() {
    let path = format!("/workspace/{}target.rs", "nested/".repeat(20));
    let input = serde_json::json!({ "filePath": path }).to_string();

    let bounded = tool_inline_summary("read", &input).expect("bounded summary");
    let unbounded = tool_inline_summary_unbounded("read", &input).expect("unbounded summary");

    assert!(bounded.ends_with('…'));
    assert_eq!(unbounded, path);
}

#[test]
fn unknown_tool_visual_state_is_neutral() {
    for raw in ["", "mystery", " complete "] {
        let visual = tool_visual_state(raw);
        assert_eq!(visual.state, ToolVisualState::Unknown, "{raw:?}");
        assert_eq!(visual.icon, Icon::Clock, "{raw:?}");
        assert_eq!(visual.label, "状态未知", "{raw:?}");
        assert!(!visual.is_animated, "{raw:?}");
        assert_eq!(visual.tone, ToolVisualTone::Neutral, "{raw:?}");
        assert_ne!(visual.icon, Icon::Check, "{raw:?}");
    }

    for theme in [Theme::Dark, Theme::Light] {
        assert_eq!(
            tool_visual_state("unknown").color(&theme),
            crate::card_style::chat_secondary_text_color(&theme)
        );
    }
}

/// 生成 ls summary uses path，用于工具卡片或状态行的简短说明。
///
/// # 参数
///
/// 参数沿用调用点中的应用状态、工具输入或渲染上下文，不在这里扩大权限或补造状态。
///
/// # 返回值
///
/// 无返回值时，函数通过发布消息或更新局部状态完成交互。
///
/// # 错误处理
///
/// 本函数不吞掉底层错误；没有显式错误通道时，会用空集合、`None` 或现有 UI 状态表达不可用结果。
#[test]
fn ls_summary_uses_path() {
    let input = serde_json::json!({
        "path": "docs/agents"
    })
    .to_string();

    assert_eq!(tool_inline_summary("ls", &input).as_deref(), Some("docs/agents"));
}

/// 生成 image info summary uses path，用于工具卡片或状态行的简短说明。
///
/// # 参数
///
/// 参数沿用调用点中的应用状态、工具输入或渲染上下文，不在这里扩大权限或补造状态。
///
/// # 返回值
///
/// 无返回值时，函数通过发布消息或更新局部状态完成交互。
///
/// # 错误处理
///
/// 本函数不吞掉底层错误；没有显式错误通道时，会用空集合、`None` 或现有 UI 状态表达不可用结果。
/// 生成 file tool icons use activity semantics，用于工具卡片或状态行的简短说明。
///
/// # 参数
///
/// 参数沿用调用点中的应用状态、工具输入或渲染上下文，不在这里扩大权限或补造状态。
///
/// # 返回值
///
/// 无返回值时，函数通过发布消息或更新局部状态完成交互。
///
/// # 错误处理
///
/// 本函数不吞掉底层错误；没有显式错误通道时，会用空集合、`None` 或现有 UI 状态表达不可用结果。
#[test]
fn config_and_read_summaries_include_key_details() {
    let config_input = serde_json::json!({
        "setting": "model",
        "value": "gpt-5"
    })
    .to_string();
    let read_input = serde_json::json!({
        "filePath": "file:///tmp/demo.rs",
        "offset": 0,
        "limit": 20
    })
    .to_string();

    assert_eq!(tool_inline_summary("config", &config_input).as_deref(), Some("model = gpt-5"));
    assert_eq!(
        tool_inline_summary("read", &read_input).as_deref(),
        Some("tmp/demo.rs (offset=1, limit=20)")
    );
}

#[test]
fn search_web_and_agent_summaries_handle_multiple_shapes() {
    let search_input = serde_json::json!({
        "pattern": "App::new",
        "path": "crates/vw-desktop/src/app"
    })
    .to_string();
    let web_input = serde_json::json!({
        "urls": ["https://example.test"],
        "query": "release notes"
    })
    .to_string();
    let agent_input = serde_json::json!({
        "agent": "code-reviewer",
        "prompt": "Check the diff for regressions"
    })
    .to_string();

    assert_eq!(
        tool_inline_summary("grep", &search_input).as_deref(),
        Some("App::new in crates/vw-desktop/src/app")
    );
    assert_eq!(
        tool_inline_summary("web_fetch", &web_input).as_deref(),
        Some("release notes in https://example.test")
    );
    assert_eq!(
        tool_inline_summary("agent", &agent_input).as_deref(),
        Some("code-reviewer · Check the diff for regressions")
    );
}

#[test]
fn verb_mappings_cover_specialized_tools_mcp_and_fallback() {
    for (tool_name, expected) in [
        ("config", "配置"),
        ("read", "读取"),
        ("write", "写入"),
        ("file_edit", "编辑"),
        ("lsp", "搜索"),
        ("bash", "运行"),
        ("ls", "列出"),
        ("web_fetch", "请求"),
        ("browser", "浏览"),
        ("skill", "技能"),
        ("task", "任务"),
        ("question", "提问"),
        ("screenshot", "截图"),
        ("agent", "AgentTool"),
        ("mcp", "MCP"),
        ("todowrite", "写任务"),
        ("todoread", "读任务"),
        ("schedule", "调度"),
        ("memory_store", "存储记忆"),
        ("memory_recall", "回忆记忆"),
        ("memory_forget", "遗忘记忆"),
        ("git_diff", "Git Diff"),
        ("view_image", "图片信息"),
        ("get_errors", "诊断"),
        ("process", "进程"),
        ("workflow_node", "工作流"),
        ("mcp_linear", "MCP"),
        ("unknown_tool", "工具"),
    ] {
        assert_eq!(tool_verb(tool_name), expected, "{tool_name}");
    }
}

#[test]
fn inline_summary_handles_plain_empty_and_malformed_inputs() {
    assert_eq!(tool_inline_summary("bash", "  cargo check  ").as_deref(), Some("cargo check"));
    assert_eq!(tool_inline_summary("bash", "  "), None);
    assert_eq!(tool_inline_summary("bash", "{not-json"), None);
    assert_eq!(tool_inline_summary("bash", "[1,2]").as_deref(), Some("[1,2]"));
}

#[test]
fn config_summary_covers_values_sections_and_missing_selection() {
    assert_eq!(tool_inline_summary("config", r#"{"setting":" theme "}"#).as_deref(), Some("theme"));
    assert_eq!(
        tool_inline_summary("config", r#"{"setting":"limits","value":{"max":3}}"#).as_deref(),
        Some(r#"limits = {"max":3}"#)
    );
    assert_eq!(
        tool_inline_summary("config", r#"{"section":" providers "}"#).as_deref(),
        Some("providers")
    );
    assert_eq!(tool_inline_summary("config", r#"{"section":"all"}"#), None);
    assert_eq!(tool_inline_summary("config", "{}"), None);
}

#[test]
fn file_summaries_cover_field_fallbacks_partial_ranges_and_missing_paths() {
    assert_eq!(
        tool_inline_summary("read", r#"{"file_path":"src/lib.rs"}"#).as_deref(),
        Some("src/lib.rs")
    );
    assert_eq!(tool_inline_summary("read", r#"{"offset":0}"#).as_deref(), Some("offset=1"));
    assert_eq!(tool_inline_summary("read", "{}"), None);
    assert_eq!(
        tool_inline_summary("file_write", r#"{"path":" notes.txt "}"#).as_deref(),
        Some("notes.txt")
    );
    assert_eq!(tool_inline_summary("file_edit", r#"{"path":" "}"#), None);
    assert_eq!(
        tool_inline_summary("apply_patch", r#"{"path":"src/app.rs"}"#).as_deref(),
        Some("src/app.rs")
    );
    assert_eq!(tool_inline_summary("apply_patch", "{}"), None);
}

#[test]
fn search_summaries_cover_pattern_path_and_empty_combinations() {
    assert_eq!(tool_inline_summary("grep", r#"{"query":"needle"}"#).as_deref(), Some("needle"));
    assert_eq!(
        tool_inline_summary("glob", r#"{"includePattern":"src/**/*.rs"}"#).as_deref(),
        Some("src/**/*.rs")
    );
    assert_eq!(
        tool_inline_summary("lsp", r#"{"symbol":"App","filePath":"src/app.rs"}"#).as_deref(),
        Some("App in src/app.rs")
    );
    assert_eq!(tool_inline_summary("web_search", "{}"), None);
}

#[test]
fn list_and_shell_summaries_cover_alias_fields_and_missing_values() {
    assert_eq!(tool_inline_summary("list_dir", r#"{"file_path":"docs"}"#).as_deref(), Some("docs"));
    assert_eq!(tool_inline_summary("ls", "{}"), None);
    assert_eq!(
        tool_inline_summary("shell", r#"{"cmd":"  printf ok  "}"#).as_deref(),
        Some("printf ok")
    );
    assert_eq!(tool_inline_summary("bash", r#"{"command":" "}"#), None);
}

#[test]
fn web_fetch_summary_covers_url_query_and_empty_combinations() {
    assert_eq!(
        tool_inline_summary("web_fetch", r#"{"urls":["https://example.test"]}"#).as_deref(),
        Some("https://example.test")
    );
    assert_eq!(
        tool_inline_summary("web_fetch", r#"{"query":"release notes"}"#).as_deref(),
        Some("release notes")
    );
    assert_eq!(
        tool_inline_summary(
            "web_fetch",
            r#"{"urls":["https://example.test"],"query":"release notes"}"#
        )
        .as_deref(),
        Some("release notes in https://example.test")
    );
    assert_eq!(tool_inline_summary("web_fetch", "{}"), None);
}

#[test]
fn question_summary_covers_counts_labels_and_fallback_question_text() {
    assert_eq!(tool_inline_summary("question", r#"{"questions":[]}"#), None);
    assert_eq!(
        tool_inline_summary("question", r#"{"questions":[{}]}"#).as_deref(),
        Some("1 个问题")
    );
    assert_eq!(
        tool_inline_summary("question", r#"{"questions":[{"question":"Proceed?"}]}"#).as_deref(),
        Some("Proceed?")
    );
    assert_eq!(
        tool_inline_summary("question", r#"{"questions":[{"header":"Scope"},{"header":"Risk"}]}"#)
            .as_deref(),
        Some("Scope 等 2 个问题")
    );
    assert_eq!(
        tool_inline_summary("question", r#"{"questions":[{},{}]}"#).as_deref(),
        Some("2 个问题")
    );
}

#[test]
fn diagnostic_repository_image_and_open_page_summaries_use_defaults_and_paths() {
    assert_eq!(
        tool_inline_summary("get_errors", r#"{"filePaths":["src/main.rs"]}"#).as_deref(),
        Some("src/main.rs")
    );
    assert_eq!(tool_inline_summary("get_errors", "{}").as_deref(), Some("当前诊断"));
    assert_eq!(tool_inline_summary("get_changed_files", "{}").as_deref(), Some("当前仓库"));
    assert_eq!(
        tool_inline_summary("get_changed_files", r#"{"repositoryPath":"workspace/project"}"#)
            .as_deref(),
        Some("workspace/project")
    );
    assert_eq!(
        tool_inline_summary("view_image", r#"{"filePath":"assets/chart.png"}"#).as_deref(),
        Some("assets/chart.png")
    );
    assert_eq!(
        tool_inline_summary("browser_open", r#"{"url":"https://example.test"}"#).as_deref(),
        Some("https://example.test")
    );
    assert_eq!(tool_inline_summary("open_browser_page", "{}"), None);
}

#[test]
fn agent_summary_covers_agent_prompt_and_empty_combinations() {
    assert_eq!(
        tool_inline_summary("agent", r#"{"subagent_type":"reviewer"}"#).as_deref(),
        Some("调用 reviewer")
    );
    assert_eq!(
        tool_inline_summary("agent", r#"{"task":"Review the change"}"#).as_deref(),
        Some("Review the change")
    );
    assert_eq!(tool_inline_summary("agent", "{}"), None);
}

#[test]
fn browser_summary_prioritizes_url_then_selector_then_action() {
    assert_eq!(
        tool_inline_summary(
            "browser",
            r##"{"action":"open","url":"https://example.test","selector":"#ignored"}"##
        )
        .as_deref(),
        Some("open · https://example.test")
    );
    assert_eq!(
        tool_inline_summary("browser", r##"{"action":"click","selector":"#submit"}"##).as_deref(),
        Some("click · #submit")
    );
    assert_eq!(tool_inline_summary("browser", r#"{"action":"back"}"#).as_deref(), Some("back"));
    assert_eq!(tool_inline_summary("browser", r#"{"action":" "}"#), None);
    assert_eq!(tool_inline_summary("browser", "{}").as_deref(), Some("browser"));
}

#[test]
fn git_summaries_cover_required_operation_diff_options_and_other_operations() {
    assert_eq!(tool_inline_summary("git_operations", "{}"), None);
    assert_eq!(
        tool_inline_summary("git_operations", r#"{"operation":"status"}"#).as_deref(),
        Some("git status")
    );
    assert_eq!(
        tool_inline_summary("git_operations", r#"{"operation":"diff"}"#).as_deref(),
        Some("git diff")
    );
    assert_eq!(
        tool_inline_summary("git_diff", r#"{"files":"src/lib.rs","cached":true}"#).as_deref(),
        Some("git diff src/lib.rs --cached")
    );
    assert_eq!(
        tool_inline_summary("git_diff", r#"{"operation":"status","files":"."}"#).as_deref(),
        Some("git diff")
    );
}

#[test]
fn json_for_unknown_tool_has_no_inline_summary() {
    assert_eq!(tool_inline_summary("unknown_tool", r#"{"value":"ignored"}"#), None);
}
