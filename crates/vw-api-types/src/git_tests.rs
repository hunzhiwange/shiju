use crate::git::{GitCommitResponse, GitDiffRequest, GitDiffScope, GitFileStatus, GitStatusDto};
use serde_json::json;

#[test]
fn git_dtos_use_stable_wire_names_and_empty_lists() {
    assert_eq!(
        serde_json::to_value(GitFileStatus::TypeChanged).expect("serialize"),
        json!("type_changed")
    );

    let status: GitStatusDto =
        serde_json::from_value(json!({ "branch": "main" })).expect("valid status");
    assert!(status.staged.is_empty());
    assert!(status.unstaged.is_empty());
    assert!(status.untracked.is_empty());

    let diff: GitDiffRequest = serde_json::from_value(json!({
        "project_id": "p1",
        "scope": "working_tree"
    }))
    .expect("valid diff request");
    assert_eq!(diff.scope, GitDiffScope::WorkingTree);
    assert_eq!(diff.worktree_id, None);
}

#[test]
fn git_commit_response_defaults_created_without_overriding_explicit_false() {
    let defaulted: GitCommitResponse = serde_json::from_value(json!({
        "ok": true,
        "commit": {
            "sha": "abc123",
            "message": "initial commit"
        }
    }))
    .expect("response without created is valid");
    assert!(defaulted.created);

    let explicit: GitCommitResponse = serde_json::from_value(json!({
        "ok": true,
        "created": false,
        "commit": {
            "sha": "abc123",
            "message": "nothing to commit"
        }
    }))
    .expect("response with explicit created is valid");
    assert!(!explicit.created);
}
