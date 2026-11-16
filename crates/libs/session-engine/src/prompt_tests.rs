#[test]
fn runtime_parts_exclude_agents_context_from_system_messages() {
    let prompt = super::SystemPrompt {
        recursive_policy: "",
        content: "base".to_string(),
        visible_instruction_paths: Vec::new(),
        base_content: "base".to_string(),
        instruction_context: Vec::new(),
    };

    assert_eq!(prompt.runtime_parts(Some("custom"), false), vec!["base", "custom"]);

    let parts = prompt.runtime_parts(Some("custom"), true);
    assert_eq!(&parts[..2], ["base", "custom"]);
    assert_eq!(parts.last().map(String::as_str), Some(super::max_steps_text()));
}

#[test]
fn max_steps_text_matches_trimmed_embedded_constraint() {
    let text = super::max_steps_text();

    assert!(!text.is_empty());
    assert_eq!(text, include_str!("prompt/max-steps.txt").trim());
    assert!(text.contains("工具已禁用"));
}

#[test]
fn block_on_runs_future_without_existing_runtime() {
    let value = super::block_on(async { 42 });

    assert_eq!(value, 42);
}

#[test]
fn block_on_uses_current_runtime_handle() {
    let runtime =
        tokio::runtime::Builder::new_current_thread().build().expect("test runtime should build");
    let _guard = runtime.enter();

    let value = super::block_on(async { 42 });

    assert_eq!(value, 42);
}

#[test]
fn join_non_empty_parts_omits_blank_entries_and_preserves_order() {
    let prompt = super::join_non_empty_parts(vec![
        "environment".to_string(),
        String::new(),
        " \t ".to_string(),
        "instruction".to_string(),
    ]);

    assert_eq!(prompt, "environment\n\ninstruction");
}

#[test]
fn join_non_empty_parts_keeps_non_blank_content_unchanged() {
    let prompt =
        super::join_non_empty_parts(vec![" environment ".to_string(), "instruction\n".to_string()]);

    assert_eq!(prompt, " environment \n\ninstruction\n");
}
