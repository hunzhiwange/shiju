#[test]
fn test_module_is_wired() {
    let module = std::hint::black_box(module_path!());

    assert!(module.ends_with("helpers_tests"));
}

#[test]
fn import_prompt_template_json_includes_selected_values_and_format_guidance() {
    let template = super::import_prompt_template(
        crate::app::task::TaskImportPromptFormat::Json,
        321,
        "gpt-test",
        Some("codex"),
    );

    assert!(
        template
            .starts_with("你是任务拆解助手。请把我的需求拆成任务列表，并严格只输出 JSON 数组。")
    );
    assert!(template.contains("\"priority\": 321,"));
    assert!(template.contains("\"model\": \"gpt-test\","));
    assert!(template.contains("\"acp_agent\": \"codex\","));
    assert!(template.ends_with(
        "reasoning_effort 可留空表示默认，或使用 none、low、medium、high、xhigh、max；非法值必须拒绝。"
    ));
}

#[test]
fn import_prompt_template_csv_uses_defaults_for_blank_model_and_missing_executor() {
    let template = super::import_prompt_template(
        crate::app::task::TaskImportPromptFormat::Csv,
        808,
        "   ",
        None,
    );

    assert!(template.starts_with("你是任务拆解助手。请把我的需求拆成任务列表，并严格只输出 CSV。"));
    assert!(
        template
            .contains("priority,prompt,model,acp_agent,reasoning_effort,task_split_mode\n808,\"")
    );
    assert!(template.contains(",auto,default,,follow_system"));
}

#[test]
fn import_prompt_template_tsv_trims_model_and_includes_reasoning_effort_column() {
    let template = super::import_prompt_template(
        crate::app::task::TaskImportPromptFormat::Tsv,
        144,
        "  model-x  ",
        Some("opencode"),
    );

    assert!(template.starts_with("你是任务拆解助手。请把我的需求拆成任务列表，并严格只输出 TSV。"));
    assert!(template.contains(
        "priority\tprompt\tmodel\tacp_agent\treasoning_effort\ttask_split_mode\n144\t\""
    ));
    assert!(template.contains("\tmodel-x\topencode\t\tfollow_system"));
    assert!(template.ends_with(
        "reasoning_effort 可留空表示默认，或使用 none、low、medium、high、xhigh、max；非法值必须拒绝。"
    ));
}
