//! 验证原始 LLM 轮次按步骤重建及压缩前缀差量处理。

use serde_json::{Value, json};

use super::llm_raw::LlmRawArtifact;

fn raw_round(step_index: u32, messages: Vec<Value>, config: Value, output: Value) -> Value {
    let message_sources =
        messages.iter().map(|message| json!({"source": message["role"]})).collect::<Vec<_>>();
    json!({
        "session_id": "session-a",
        "step_index": step_index,
        "model": "requested-model",
        "system": ["System instruction"],
        "messages": messages,
        "message_sources": message_sources,
        "request": {
            "config": config,
            "tools": [{"id": "shell", "description": "Run a command"}],
        },
        "output": output,
    })
}

#[test]
// 压缩或重试可能复用步骤编号，轮次键还需区分消息内容。
fn raw_turn_key_distinguishes_reused_step_indices() {
    let raw = raw_round(
        0,
        vec![json!({"role": "user", "content": "hello"})],
        json!({"model": "provider/model-a"}),
        json!({"text": "first"}),
    );
    let first_turn = LlmRawArtifact::from_full_raw_rounds("turn-a", &[raw.clone()]).unwrap();
    let second_turn = LlmRawArtifact::from_full_raw_rounds("turn-b", &[raw]).unwrap();

    assert_eq!(first_turn.rounds[0].step_index, second_turn.rounds[0].step_index);
    assert_ne!(first_turn.rounds[0].turn_id, second_turn.rounds[0].turn_id);
}

#[test]
fn raw_rounds_reconstruct_provider_messages_after_tools() {
    let first = raw_round(
        0,
        vec![json!({"role": "user", "content": "inspect the workspace"})],
        json!({"model": "provider/model-a", "temperature": 0.1}),
        json!({"text": "", "tool_calls": [{"id": "call-1", "name": "shell"}]}),
    );
    let second = raw_round(
        1,
        vec![
            json!({"role": "user", "content": "inspect the workspace"}),
            json!({"role": "assistant", "tool_calls": [{"id": "call-1", "name": "shell"}]}),
            json!({"role": "tool", "tool_call_id": "call-1", "content": "Cargo.toml"}),
        ],
        json!({"model": "provider/model-a", "temperature": 0.4}),
        json!({"text": "done", "tool_calls": []}),
    );

    let artifact =
        LlmRawArtifact::from_full_raw_rounds("turn-a", &[first.clone(), second.clone()]).unwrap();

    assert_eq!(artifact.rounds[1].provider_message_prefix_len, 1);
    assert_eq!(artifact.rounds[1].provider_message_suffix.len(), 2);
    assert_eq!(artifact.reconstruct_round(1).unwrap(), second);
    assert_eq!(artifact.reconstruct_round(0).unwrap(), first);
}

#[test]
fn raw_rounds_reconstruct_final_step_system_and_tool_changes() {
    let first = raw_round(
        0,
        vec![json!({"role": "user", "content": "inspect the workspace"})],
        json!({"model": "provider/model-a"}),
        json!({"text": "", "tool_calls": [{"id": "call-1", "name": "shell"}]}),
    );
    let mut final_round = raw_round(
        1,
        vec![
            json!({"role": "user", "content": "inspect the workspace"}),
            json!({"role": "tool", "tool_call_id": "call-1", "content": "Cargo.toml"}),
        ],
        json!({"model": "provider/model-a"}),
        json!({"text": "done", "tool_calls": []}),
    );
    final_round["system"] = json!(["Final answer only"]);
    final_round["request"]["tools"] = json!([]);

    let artifact =
        LlmRawArtifact::from_full_raw_rounds("turn-a", &[first, final_round.clone()]).unwrap();

    assert_eq!(artifact.rounds[1].system_delta, Some(json!(["Final answer only"])));
    assert_eq!(artifact.rounds[1].tools_delta, Some(json!([])));
    assert_eq!(artifact.reconstruct_round(1).unwrap(), final_round);
}

#[test]
// 历史前缀压缩后只持久化新增差量，不能重复保存剩余消息。
fn raw_round_delta_handles_compacted_prefix() {
    let before_compaction = raw_round(
        0,
        vec![
            json!({"role": "system", "content": "keep"}),
            json!({"role": "user", "content": "old request"}),
            json!({"role": "assistant", "content": "old response"}),
        ],
        json!({"model": "provider/model-a"}),
        json!({"text": "tool call"}),
    );
    let after_compaction = raw_round(
        1,
        vec![
            json!({"role": "system", "content": "keep"}),
            json!({"role": "system", "content": "summary of old request and response"}),
            json!({"role": "user", "content": "new request"}),
        ],
        json!({"model": "provider/model-a"}),
        json!({"text": "final"}),
    );

    let artifact = LlmRawArtifact::from_full_raw_rounds(
        "turn-a",
        &[before_compaction, after_compaction.clone()],
    )
    .unwrap();

    assert_eq!(artifact.rounds[1].provider_message_prefix_len, 1);
    assert_eq!(artifact.rounds[1].provider_message_suffix.len(), 2);
    assert_eq!(artifact.reconstruct_round(1).unwrap(), after_compaction);
}
