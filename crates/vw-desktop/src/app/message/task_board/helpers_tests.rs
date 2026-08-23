// 验证各导入格式的任务提示模板与默认字段选择。
// 测试夹具围绕目标分支构造可控状态；跨模块行为由对应集成测试负责。

// 编译接线：引用父模块符号，确保测试片段仍由真实模块包含。
// 该冒烟用例只承担接线职责，具体行为由同领域测试覆盖。
#[test]
fn test_module_is_wired() {
    let module = std::hint::black_box(module_path!());

    assert!(module.ends_with("helpers_tests"));
}

// 数据边界：覆盖规范化、默认值以及相关状态字段的既定规则。
// 这里固定可观察结果，避免依赖处理过程中的临时实现细节。
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

// 数据边界：覆盖规范化、默认值以及相关状态字段的既定规则。
// 这里固定可观察结果，避免依赖处理过程中的临时实现细节。
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

// 数据边界：覆盖规范化、默认值以及相关状态字段的既定规则。
// 这里固定可观察结果，避免依赖处理过程中的临时实现细节。
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
