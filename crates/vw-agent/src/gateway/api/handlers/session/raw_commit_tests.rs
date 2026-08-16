use super::*;

fn artifact(turn_id: &str, step_index: u32) -> LlmRawArtifact {
    let raw = serde_json::json!({
        "session_id": "session-a",
        "step_index": step_index,
        "model": "provider/model",
        "system": ["system"],
        "messages": [{"role": "user", "content": "hello"}],
        "message_sources": [null],
        "request": {"config": {"model": "provider/model"}, "tools": []},
        "output": {"text": "answer"}
    });
    LlmRawArtifact::from_full_raw_rounds(turn_id, &[raw]).expect("valid raw artifact")
}

#[test]
fn failed_commit_retains_pending_raw() {
    let tenant = TenantScope::default();
    let mut registry = PendingRawCommitRegistry::default();
    registry.register(&tenant, Some("project-a"), artifact("turn-a", 0)).expect("register raw");

    let first = registry
        .claim(&tenant, Some("project-a"), "session-a", &[("turn-a".to_string(), 0)])
        .expect("claim raw");
    assert_eq!(first.raw_artifacts().len(), 1);

    let retry = registry
        .claim(&tenant, Some("project-a"), "session-a", &[("turn-a".to_string(), 0)])
        .expect("failed commit must remain claimable");
    assert_eq!(retry.raw_artifacts().len(), 1);
}

#[test]
fn successful_commit_acknowledges_only_claimed_version() {
    let tenant = TenantScope::default();
    let mut registry = PendingRawCommitRegistry::default();
    registry
        .register(&tenant, Some("project-a"), artifact("turn-a", 0))
        .expect("register first raw");
    let first = registry
        .claim(&tenant, Some("project-a"), "session-a", &[("turn-a".to_string(), 0)])
        .expect("claim first raw");
    registry
        .register(&tenant, Some("project-a"), artifact("turn-a", 1))
        .expect("register next raw");

    registry.acknowledge(&first);
    let remaining = registry
        .claim(
            &tenant,
            Some("project-a"),
            "session-a",
            &[("turn-a".to_string(), 0), ("turn-a".to_string(), 1)],
        )
        .expect("claim remaining raw");
    assert_eq!(remaining.raw_artifacts().len(), 1);
    assert_eq!(remaining.expected_steps()[0].step_index(), 1);
}
