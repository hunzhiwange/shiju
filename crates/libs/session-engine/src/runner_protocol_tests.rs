//! Context preflight uses the library protocol directly.

#[test]
fn context_preflight_runs_before_next_llm_step_after_real_input_usage() {
    let mut messages = vec![serde_json::json!({"role": "system", "content": "system"})];
    for index in 0..9 {
        let call_id = format!("call-{index}");
        messages.push(serde_json::json!({
            "role": "assistant",
            "tool_calls": [{"id": call_id, "type": "function"}]
        }));
        messages.push(serde_json::json!({
            "role": "tool",
            "tool_call_id": call_id,
            "content": if index == 0 { "old output ".repeat(80) } else { "recent".to_string() }
        }));
    }
    messages.push(serde_json::json!({"role": "user", "content": "latest question"}));

    let result = super::prepare_context_before_llm_step(
        &mut messages,
        &["system".to_string()],
        &[],
        &provider::ModelLimit {
            context: 500,
            input: Some(500),
            output: 0,
        },
        Some(490),
        false,
    )
    .expect("the next LLM step should report whether internal compaction is required");

    assert!(result.needs_internal_compaction);
    assert!(messages.iter().any(|message| message["role"] == "system"));
    assert!(messages.iter().any(|message| message["role"] == "user"));
    assert_ne!(
        messages
            .iter()
            .find(|message| message["tool_call_id"] == "call-0")
            .and_then(|message| message["content"].as_str()),
        Some(
            "old output old output old output old output old output old output old output old output old output old output old output old output old output old output old output old output old output old output old output old output old output old output old output old output old output old output old output old output old output old output old output old output old output old output old output old output old output old output old output old output old output old output old output old output old output old output old output old output old output old output old output old output old output old output old output old output old output old output old output old output old output old output old output old output old output old output old output old output old output old output "
        )
    );
}
