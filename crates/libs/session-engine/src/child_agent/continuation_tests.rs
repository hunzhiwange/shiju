use super::*;

#[tokio::test]
async fn agcollab_followup_reuses_session_with_new_turn() {
    let coordinator = ChildAgentCoordinator::new(128);
    coordinator.register("parent", "child", "worker", "first").unwrap();
    let mut receiver = coordinator.attach_continuation("child");
    coordinator.mark_succeeded("parent", "child", "first result").unwrap();
    coordinator.followup_task("parent", "child", "second".into()).unwrap();
    let prompt = receiver.recv().await.unwrap();
    let old_token = coordinator.turn_cancellation("child").unwrap();
    coordinator.begin_followup("parent", "child", &prompt).unwrap();
    assert!(old_token.is_cancelled());
    assert!(!coordinator.turn_cancellation("child").unwrap().is_cancelled());
    let tasks = coordinator.snapshots("parent", &["child".into()]).unwrap();
    assert_eq!(tasks[0].child_session_id, "child");
    assert_eq!(tasks[0].turn, 2);
    assert!(!coordinator.finish_turn("parent", "child", 1, Some("stale".into()), None).unwrap());
    assert_eq!(tasks[0].output, None);
    assert_eq!(tasks[0].state, ChildAgentTaskState::Running);
    assert!(coordinator.followup_task("stranger", "child", "bad".into()).is_err());
    coordinator.cancel("parent", "child").unwrap();
    assert!(coordinator.followup_task("parent", "child", "bad".into()).is_err());
}

#[test]
fn agcollab_followup_rejects_missing_runtime() {
    let coordinator = ChildAgentCoordinator::new(128);
    coordinator.register("parent", "child", "worker", "first").unwrap();
    assert!(coordinator.followup_task("parent", "child", "second".into()).is_err());
}
