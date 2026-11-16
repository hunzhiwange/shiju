//! 规划文本清单解析的库内测试。

#[test]
fn planning_tests_module_is_wired() {
    let marker = String::from("planning_tests");
    assert_eq!(marker.as_str(), "planning_tests");
}

fn todo_contents(input: &str) -> Vec<String> {
    let value = serde_json::from_str::<serde_json::Value>(input)
        .expect("todowrite input should be valid JSON");
    value["todos"]
        .as_array()
        .expect("todos should be an array")
        .iter()
        .map(|todo| todo["content"].as_str().expect("todo content should be string").to_string())
        .collect()
}

#[test]
fn build_todowrite_from_text_extracts_common_list_formats() {
    let input = super::build_todowrite_from_text(
        r#"
        1. Read the source
        2) Add focused tests
        - [ ] Update docs
        - [x] Keep regression covered
        * Run formatter
        /todowrite {}
        ```
        - ignored code item
        ```
        "#,
    )
    .expect("list text should produce todowrite input");

    let value = serde_json::from_str::<serde_json::Value>(&input)
        .expect("todowrite input should be valid JSON");
    let todos = value["todos"].as_array().expect("todos should be an array");

    assert_eq!(
        todo_contents(&input),
        vec![
            "Read the source",
            "Add focused tests",
            "Update docs",
            "Keep regression covered",
            "Run formatter",
        ]
    );
    assert_eq!(value["merge"].as_bool(), Some(false));
    assert_eq!(todos[0]["id"].as_str(), Some("1"));
    assert_eq!(todos[0]["status"].as_str(), Some("pending"));
    assert_eq!(todos[0]["priority"].as_str(), Some("medium"));
}

#[test]
// 从自然语言提取的待办需规范化、去重和限量，避免生成失控任务列表。
fn build_todowrite_from_text_deduplicates_normalizes_and_limits_items() {
    let mut answer = String::new();
    answer.push_str("- Ship   feature\n");
    answer.push_str("- ship feature\n");
    for idx in 0..20 {
        answer.push_str(&format!("{}. item {idx}\n", idx + 1));
    }

    let input = super::build_todowrite_from_text(&answer)
        .expect("list text should produce todowrite input");
    let contents = todo_contents(&input);

    assert_eq!(contents.len(), 12);
    assert_eq!(contents[0], "Ship feature");
    assert_eq!(contents[1], "item 0");
    assert_eq!(contents[11], "item 10");
    assert_eq!(contents.iter().filter(|item| item.as_str() == "ship feature").count(), 0);
}

#[test]
fn build_todowrite_from_text_returns_none_for_empty_or_command_only_text() {
    assert!(super::build_todowrite_from_text("").is_none());
    assert!(super::build_todowrite_from_text("/todowrite {}").is_none());
    assert!(super::build_todowrite_from_text("```\\n- hidden\\n```").is_none());
    assert!(super::build_todowrite_from_text("- a").is_none());
}

#[test]
fn build_todowrite_from_text_handles_uppercase_checkbox_and_skips_empty_markers() {
    let input = super::build_todowrite_from_text(
        "- [X] Finished review\n- [ ]\n- [x]\n-\n*\n1.\n2)\n- /todowrite {}",
    )
    .expect("uppercase checkbox should produce a todowrite input");

    assert_eq!(todo_contents(&input), vec!["Finished review"]);
}

#[test]
fn build_todowrite_from_text_only_strips_numeric_list_prefixes() {
    let input = super::build_todowrite_from_text(
        "release.v1 stays intact\nphase) stays intact\n0. zero-based task\n007) padded task",
    )
    .expect("plain and numbered lines should produce a todowrite input");

    assert_eq!(
        todo_contents(&input),
        vec!["release.v1 stays intact", "phase) stays intact", "zero-based task", "padded task",]
    );
}

#[test]
fn build_todowrite_from_text_ignores_remainder_after_unclosed_code_fence() {
    let input = super::build_todowrite_from_text(
        "Visible task\n```rust\n- hidden task\n1. another hidden task",
    )
    .expect("text before an unclosed fence should remain available");

    assert_eq!(todo_contents(&input), vec!["Visible task"]);
}
