use super::*;

#[test]
fn agcollab_delivery_results_are_once_per_turn_and_parent_scoped() {
    let coordinator = ChildAgentCoordinator::new(128);
    coordinator.register("root", "child", "worker", "task").unwrap();
    coordinator.register("other", "foreign", "worker", "task").unwrap();
    coordinator.mark_succeeded("root", "child", "first").unwrap();
    coordinator.mark_succeeded("other", "foreign", "private").unwrap();
    let context = ToolUseContext::new("root", None);
    let mut messages = Vec::new();
    assert!(coordinator.deliver_model_events(&context, &mut messages).unwrap());
    assert_eq!(messages.len(), 1);
    assert_eq!(messages[0]["role"], "user");
    let content = messages[0]["content"].as_str().unwrap();
    assert!(content.contains("child") && content.contains("first"));
    assert!(!content.contains("private"));
    assert!(!coordinator.deliver_model_events(&context, &mut messages).unwrap());
    let _runtime = coordinator.attach_continuation("child");
    coordinator.begin_followup("root", "child", "next").unwrap();
    coordinator.mark_succeeded("root", "child", "second").unwrap();
    assert!(coordinator.deliver_model_events(&context, &mut messages).unwrap());
    assert_eq!(messages.len(), 2);
    let content = messages[1]["content"].as_str().unwrap();
    assert!(content.contains("second") && content.contains("\"turn\":2"));
}

#[test]
fn agcollab_delivery_unknown_session_has_no_team_data() {
    let coordinator = ChildAgentCoordinator::new(128);
    let mut messages = Vec::new();
    assert!(
        !coordinator
            .deliver_model_events(&ToolUseContext::new("unknown", None), &mut messages)
            .unwrap()
    );
    assert!(messages.is_empty());
}
