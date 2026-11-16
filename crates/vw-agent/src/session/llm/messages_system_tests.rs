//! Real host system prompt composition and unified marker contracts.

use super::build_system_messages;
use super::messages_tests::{test_input, test_model};

#[test]
fn build_system_messages_merges_non_empty_sources_in_order() {
    let model = test_model("test-provider", "custom-model", "openai-compatible");
    let input = test_input(model);

    let system = build_system_messages(&input, false);

    llm_runtime::system_message_contract::merged_sources(&system);
}

#[test]
fn build_system_messages_does_not_duplicate_unified_prompt_for_codex() {
    // Stable marker from unified.txt; must appear once regardless of is_codex.
    const UNIFIED_MARKER: &str = crate::app::agent::session::system::UNIFIED_PROMPT_MARKER;

    let model = test_model("openai", "gpt-5-test", "openai");
    let input = test_input(model);

    let without_codex = build_system_messages(&input, false).join("\n");
    let with_codex = build_system_messages(&input, true).join("\n");

    llm_runtime::system_message_contract::unified_once(&without_codex, &with_codex, UNIFIED_MARKER);
}

#[test]
fn build_system_messages_keeps_pre_rendered_runtime_prompt_once() {
    let model = test_model("deepseek", "deepseek-v4-flash", "openai-compatible");
    let mut input = test_input(model);
    let rendered = crate::app::agent::session::system::render_unified_prompt(
        "deepseek/deepseek-v4-flash",
        "/workspace/project",
        &["TodoRead".into(), "TodoWrite".into(), "AgentTool".into()],
    );
    input.system = vec![rendered, "session system".to_string()];

    let joined = build_system_messages(&input, false).join("\n");

    llm_runtime::system_message_contract::rendered_runtime_once(&joined, crate::session::system::UNIFIED_PROMPT_MARKER);
}

#[test]
// 空的可选上下文不生成占位消息，供应商系统提示仍保持原顺序。
fn build_system_messages_filters_blank_optional_sources_and_keeps_provider_prompt() {
    let model = test_model("test-provider", "custom-model", "openai-compatible");
    let provider_prompt = crate::app::agent::session::system::provider(&model).join("\n");
    let mut input = test_input(model);
    input.agent.prompt = Some(" \t".to_string());
    input.system = vec![String::new(), " \n".to_string()];
    input.user.system = Some("\t".to_string());

    let system = build_system_messages(&input, false);

    llm_runtime::system_message_contract::provider_only(system, provider_prompt);
}
