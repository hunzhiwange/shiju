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
