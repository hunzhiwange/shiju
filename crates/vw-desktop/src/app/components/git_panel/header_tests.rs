//! 验证 Git 面板标题栏的工作树标签、切换消息与视图组合。
//! 覆盖字符级截断、空分支名和未匹配工作树时的显示回退。


use crate::app::state::GitWorktreeOption;
use crate::app::{Message, message};

fn app() -> crate::app::App {
    crate::app::App::new().0
}

fn worktree(directory: &str, label: &str, branch: Option<&str>) -> GitWorktreeOption {
    GitWorktreeOption {
        directory: directory.to_string(),
        label: label.to_string(),
        branch: branch.map(str::to_string),
    }
}


#[test]
fn selected_worktree_trigger_label_uses_matching_option_or_fallback() {
    let mut app = app();
    app.selected_git_worktree_directory = Some("/repo/beta".to_string());
    app.git_worktree_options = vec![
        worktree("/repo/alpha", "alpha", Some("main")),
        worktree("/repo/beta", "beta worktree · metadata", Some("feature")),
    ];

    assert_eq!(super::header::selected_worktree_trigger_label(&app), "beta w…");

    app.selected_git_worktree_directory = Some("/repo/missing".to_string());
    assert_eq!(super::header::selected_worktree_trigger_label(&app), "工作区");
}

#[test]
fn header_icon_tip_helpers_reflect_summary_and_fullscreen_state() {
    assert_eq!(
        super::header::summary_toggle_icon_and_tip(false),
        (crate::app::assets::Icon::Eye, "显示摘要输入框".to_string())
    );
    assert_eq!(
        super::header::summary_toggle_icon_and_tip(true),
        (crate::app::assets::Icon::EyeSlash, "隐藏摘要输入框".to_string())
    );
    assert_eq!(
        super::header::fullscreen_icon_and_tip(false),
        (crate::app::assets::Icon::Fullscreen, "全屏".to_string())
    );
    assert_eq!(
        super::header::fullscreen_icon_and_tip(true),
        (crate::app::assets::Icon::FullscreenExit, "退出全屏".to_string())
    );
}

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

#[test]
fn view_builds_with_default_branch_and_no_worktree_picker() {
    let app = app();

    let _element = super::header::view(&app, vec!["src/lib.rs".to_string()]);
}

#[test]
fn view_builds_with_long_branch_fullscreen_and_open_worktree_menu() {
    let mut app = app();
    app.selected_branch = Some("feature/super-long-branch-name".to_string());
    app.git_diff_fullscreen = true;
    app.show_git_diff_summary = true;
    app.show_git_filter_options = true;
    app.git_worktree_menu_open = true;
    app.selected_git_worktree_directory = Some("/repo/wt-two".to_string());
    app.git_worktree_options = vec![
        worktree("/repo/main", "main worktree · extra metadata", Some("main")),
        worktree(
            "/repo/wt-two",
            "secondary worktree with very long label · detail",
            Some("feature/extremely-long-branch"),
        ),
    ];

    let _element =
        super::header::view(&app, vec!["src/lib.rs".to_string(), "src/main.rs".to_string()]);
}

#[test]
fn view_builds_worktree_picker_without_matching_selected_directory_or_branch() {
    let mut app = app();
    app.selected_git_worktree_directory = Some("/repo/missing".to_string());
    app.git_worktree_options = vec![
        worktree("/repo/a", "alpha", None),
        worktree("/repo/b", "beta · ignored suffix", Some("")),
    ];

    let _element = super::header::view(&app, Vec::new());
}
