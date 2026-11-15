//! 权限模式测试，覆盖各模式下命令自动放行的边界。

use super::mode::PermissionMode;

#[test]
fn accept_edits_auto_allows_expected_commands() {
    assert!(PermissionMode::AcceptEdits.auto_allows_command("touch"));
    assert!(PermissionMode::AcceptEdits.auto_allows_command("mkdir"));
    assert!(!PermissionMode::AcceptEdits.auto_allows_command("git"));
}

#[test]
fn auto_accept_allows_everything() {
    assert!(PermissionMode::AutoAccept.auto_allows_command("anything"));
}

#[test]
fn normal_mode_does_not_auto_allow() {
    assert!(!PermissionMode::Normal.auto_allows_command("touch"));
}
