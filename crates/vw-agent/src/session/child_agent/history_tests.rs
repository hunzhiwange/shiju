use super::*;
use serde_json::json;

#[test]
fn agcollab_history_inherits_effective_context() {
    let source = vec![
        json!({"role":"system","content":"parent identity"}),
        json!({"role":"user","content":"constraint"}),
        json!({"role":"assistant","content":"<think>secret</think>conclusion"}),
        json!({"role":"tool","content":"orphan"}),
        json!({"role":"assistant","tool_calls":[{"id":"unpaired"}]}),
        json!({"role":"user","content":"compressed summary"}),
    ];
    let inherited = snapshot(&source);
    assert_eq!(
        inherited,
        vec![
            json!({"role":"user","content":"constraint"}),
            json!({"role":"assistant","content":"conclusion"}),
            json!({"role":"user","content":"compressed summary"})
        ]
    );
}

#[test]
fn agcollab_fork_turns_defaults_all_and_rejects_unknown() {
    assert_eq!(ForkTurns::default(), ForkTurns::All);
    assert_eq!(serde_json::from_value::<ForkTurns>(json!("none")).unwrap(), ForkTurns::None);
    assert!(serde_json::from_value::<ForkTurns>(json!("invalid")).is_err());
}
