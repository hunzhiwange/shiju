use super::*;
use tui_contracts::action::{V3ApprovalChoice, V3ApprovalKind, V3ApprovalRequest};

fn request() -> V3ApprovalRequest {
    V3ApprovalRequest::new(
        "permission-1",
        V3ApprovalKind::Command,
        "Would you like to run this command?",
        "$ git status --short",
        vec![
            V3ApprovalChoice::new("allow", "Allow", V3ApprovalDecision::Allow),
            V3ApprovalChoice::new(
                "custom",
                "Allow with custom rule",
                V3ApprovalDecision::Custom("prefix:git".into()),
            ),
            V3ApprovalChoice::new("deny", "Deny", V3ApprovalDecision::Deny),
            V3ApprovalChoice::new("cancel", "Cancel", V3ApprovalDecision::Cancel),
        ],
    )
}

// 验证 `v3_approval_actions_preserve_reject_and_custom_paths` 场景的触发条件与预期结果。
#[test]
fn v3_approval_actions_preserve_reject_and_custom_paths() {
    let mut overlay = ApprovalOverlay::new(request());
    assert_eq!(overlay.handle_key_event(KeyEvent::from(KeyCode::Down)), V3Action::None);
    assert_eq!(
        overlay.handle_key_event(KeyEvent::from(KeyCode::Enter)),
        V3Action::ResolveApproval {
            request_id: "permission-1".into(),
            decision: V3ApprovalDecision::Custom("prefix:git".into()),
        }
    );

    let mut cancelled = ApprovalOverlay::new(request());
    assert_eq!(
        cancelled.handle_key_event(KeyEvent::from(KeyCode::Esc)),
        V3Action::ResolveApproval {
            request_id: "permission-1".into(),
            decision: V3ApprovalDecision::Cancel,
        }
    );
}

// 验证 `approval_choices_keep_explicit_deny_and_session_options` 场景的触发条件与预期结果。
#[test]
fn approval_choices_keep_explicit_deny_and_session_options() {
    let request = V3ApprovalRequest::new(
        "file-1",
        V3ApprovalKind::FileChange,
        "Apply edits?",
        "src/main.rs",
        vec![
            V3ApprovalChoice::new(
                "session",
                "Allow for session",
                V3ApprovalDecision::AllowForSession,
            ),
            V3ApprovalChoice::new("deny", "Reject edits", V3ApprovalDecision::Deny),
        ],
    );
    assert_eq!(request.choices.len(), 2);
    assert_eq!(request.choices[1].decision, V3ApprovalDecision::Deny);
}
