use super::*;

#[test]
fn historical_diff_blocks_worktree_actions_and_allows_browsing() {
    assert!(matches!(
        history::history_event(
            0,
            project_event(Message::Git(GitMessage::DiscardFile("file.rs".into())))
        ),
        HistoryAction::None
    ));
    assert!(matches!(
        history::history_event(
            0,
            project_event(Message::Git(GitMessage::ToggleStageLine("file.rs".into(), 0, true)))
        ),
        HistoryAction::None
    ));
    assert!(matches!(
        history::history_event(
            0,
            project_event(Message::Git(GitMessage::OpenDiffContextMenu {
                file: "file.rs".into(),
                line: 0,
                is_old: false,
                text: "text".into(),
                x: 0.0,
                y: 0.0
            }))
        ),
        HistoryAction::None
    ));
    assert!(matches!(
        history::history_event(
            0,
            project_event(Message::Git(GitMessage::ExpandContext(
                "history/file.rs".into(),
                0,
                crate::message_git::messages::ExpandDirection::All
            )))
        ),
        HistoryAction::Browse(Message::Git(GitMessage::ExpandContext(..)))
    ));
}
