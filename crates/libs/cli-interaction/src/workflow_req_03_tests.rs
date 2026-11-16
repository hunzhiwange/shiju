use crate::{app::markdown_export::{export, markdown}, commands::{route_slash_submission, CommandRoute}};
use cli_interaction_model::{UiAssistantMessage, UiAttachment, UiAttachmentKind, UiMessage, UiMessageBase, UiMessageId, UiTurnTerminal};
use cli_interaction_state::TuiState;

fn state() -> TuiState {
    let mut state = TuiState::default();
    state.messages.push(UiMessage::Assistant(UiAssistantMessage {
        base: UiMessageBase::new(UiMessageId::new("answer")),
        text: "你好🙂\n```rust\nlet x = 1;\n```".into(),
        usage: Default::default(), step_count: 1,
        terminal: UiTurnTerminal::Done { finish_reason: None }, model: None,
    }));
    state
}
#[test]
fn workflow_req_03_routes_path_and_primes_empty_command() {
    assert_eq!(route_slash_submission("/export 结果 文件.md").unwrap(), Some(CommandRoute::ExportSession { path: "结果 文件.md".into() }));
    assert_eq!(route_slash_submission("/export").unwrap(), Some(CommandRoute::ExportSession { path: String::new() }));
}
#[test]
fn workflow_req_03_preserves_raw_unicode_and_fences() {
    let text = markdown(&state()).unwrap();
    assert!(text.contains("## Assistant\n\n你好🙂\n```rust\nlet x = 1;\n```"));
    assert!(text.contains("未加载历史未导出"));
}
#[test]
fn workflow_req_03_omits_attachment_metadata() {
    let mut state = state();
    state.messages.push(UiMessage::Attachment(UiAttachment::new(
        UiMessageBase::new(UiMessageId::new("image")), UiAttachmentKind::Image,
        "secret-name", "file:///secret/path",
    ).with_preview("secret-preview")));
    let text = markdown(&state).unwrap();
    assert!(text.contains("附件已省略"));
    assert!(!text.contains("secret"));
}
#[test]
fn workflow_req_03_refuses_foreign_messages() {
    let mut state = state();
    state.session.session_id = Some("current".into());
    if let UiMessage::Assistant(m) = &mut state.messages[0] { m.base.session_id = Some("foreign".into()); }
    assert!(markdown(&state).is_err());
}
#[test]
fn workflow_req_03_refuses_traversal_and_directory() {
    let root = tempfile::tempdir().unwrap();
    for path in ["../escape.md", "/absolute.md", ".", ""] {
        assert!(export(&state(), root.path(), path).is_err());
    }
}
#[cfg(unix)]
#[test]
fn workflow_req_03_writes_once_without_overwrite() {
    let root = tempfile::tempdir().unwrap();
    let path = export(&state(), root.path(), "你好.md").unwrap();
    assert_eq!(std::fs::read_to_string(&path).unwrap(), markdown(&state()).unwrap());
    assert!(export(&TuiState::default(), root.path(), "你好.md").is_err());
    assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 1);
}
#[cfg(unix)]
#[test]
fn workflow_req_03_refuses_symlink_parent_and_destination() {
    use std::os::unix::fs::symlink;
    let root = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    symlink(outside.path(), root.path().join("linked")).unwrap();
    assert!(export(&state(), root.path(), "linked/escape.md").is_err());
    symlink(outside.path().join("absent"), root.path().join("target.md")).unwrap();
    assert!(export(&state(), root.path(), "target.md").is_err());
    assert_eq!(std::fs::read_dir(outside.path()).unwrap().count(), 0);
}
