use super::*;

#[test]
fn reasoning_effort_rejects_unsupported_models_and_levels() {
    for (model, effort) in [
        ("claude-sonnet-4-5", ReasoningEffort::High),
        ("claude-opus-4-5", ReasoningEffort::Max),
        ("claude-opus-4-6", ReasoningEffort::Xhigh),
        ("claude-opus-4-7", ReasoningEffort::Minimal),
        ("unknown", ReasoningEffort::High),
    ] {
        assert!(validate_effort(model, Some(effort)).is_err());
        assert!(validate_effort(model, None).is_ok());
    }
    assert!(validate_effort("claude-opus-4-7-20260101", Some(ReasoningEffort::Xhigh)).is_ok());
}
