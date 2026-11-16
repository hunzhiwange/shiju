//! 保留桌面 Git 消息适配契约；标题栏局部行为见 desktop-git-ui。

use crate::message_git::messages as message;
use crate::page::Message;

#[test]
fn header_toggle_message_helpers_flip_current_state() {
    match super::header::worktree_menu_toggle_message(false) {
        Message::Git(message::GitMessage::ToggleGitWorktreeMenu(value)) => assert!(value),
        _ => panic!("expected worktree toggle message"),
    }
    match super::header::worktree_menu_toggle_message(true) {
        Message::Git(message::GitMessage::ToggleGitWorktreeMenu(value)) => assert!(!value),
        _ => panic!("expected worktree toggle message"),
    }
    match super::header::filter_options_toggle_message(false) {
        Message::Git(message::GitMessage::ToggleFilterOptions(value)) => assert!(value),
        _ => panic!("expected filter toggle message"),
    }
    match super::header::filter_options_toggle_message(true) {
        Message::Git(message::GitMessage::ToggleFilterOptions(value)) => assert!(!value),
        _ => panic!("expected filter toggle message"),
    }
}
