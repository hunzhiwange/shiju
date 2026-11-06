use super::*;

#[test]
fn historical_diff_blocks_worktree_actions_and_allows_browsing() {
    assert!(matches!(
        history_event(Message::Git(GitMessage::DiscardFile("file.rs".into()))),
        Message::None
    ));
    assert!(matches!(
        history_event(Message::Git(GitMessage::ToggleStageLine("file.rs".into(), 0, true))),
        Message::None
    ));
    assert!(matches!(
        history_event(Message::Git(GitMessage::OpenDiffContextMenu {
            file: "file.rs".into(),
            line: 0,
            is_old: false,
            text: "text".into(),
            x: 0.0,
            y: 0.0
        })),
        Message::None
    ));
    assert!(matches!(
        history_event(Message::Git(GitMessage::ExpandContext(
            "history/file.rs".into(),
            0,
            crate::app::message::git::ExpandDirection::All
        ))),
        Message::Git(GitMessage::ExpandContext(..))
    ));
}
