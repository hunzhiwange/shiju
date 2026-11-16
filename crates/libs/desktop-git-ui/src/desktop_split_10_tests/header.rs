use crate::git_panel::{header, types::GitWorktreeOption};
use desktop_assets::assets::Icon;
fn app<'a>() -> header::Input<'a> {
    header::Input {
        desktop_locale: config_ui::DesktopLocale::default(),
        selected_branch: &None,
        git_worktree_options: &[],
        selected_git_worktree_directory: &None,
        git_worktree_menu_open: false,
        show_git_diff_summary: false,
        show_git_filter_options: false,
        git_diff_fullscreen: false,
        code_mode: false,
    }
}
fn worktree(directory: &str, label: &str, branch: Option<&str>) -> GitWorktreeOption {
    GitWorktreeOption {
        directory: directory.into(),
        label: label.into(),
        branch: branch.map(str::to_owned),
    }
}
#[test]
fn selected_worktree_trigger_label_uses_matching_option_or_fallback() {
    let mut app = app();
    let selected_git_worktree_directory = Some("/repo/beta".to_string());
    app.selected_git_worktree_directory = &selected_git_worktree_directory;
    let options = [
        worktree("/repo/alpha", "alpha", Some("main")),
        worktree("/repo/beta", "beta worktree · metadata", Some("feature")),
    ];
    app.git_worktree_options = &options;

    assert_eq!(header::selected_worktree_trigger_label(&app), "beta w…");

    let selected_git_worktree_directory = Some("/repo/missing".to_string());
    app.selected_git_worktree_directory = &selected_git_worktree_directory;
    assert_eq!(header::selected_worktree_trigger_label(&app), "工作区");
}

#[test]
fn header_icon_tip_helpers_reflect_summary_and_fullscreen_state() {
    assert_eq!(
        header::summary_toggle_icon_and_tip(false),
        (Icon::Eye, "显示摘要输入框".to_string())
    );
    assert_eq!(
        header::summary_toggle_icon_and_tip(true),
        (Icon::EyeSlash, "隐藏摘要输入框".to_string())
    );
    assert_eq!(header::fullscreen_icon_and_tip(false), (Icon::Fullscreen, "全屏".to_string()));
    assert_eq!(
        header::fullscreen_icon_and_tip(true),
        (Icon::FullscreenExit, "退出全屏".to_string())
    );
}

#[test]
fn view_builds_with_default_branch_and_no_worktree_picker() {
    let app = app();

    let _element = header::view(&app, vec!["src/lib.rs".to_string()]);
}

#[test]
fn view_builds_with_long_branch_fullscreen_and_open_worktree_menu() {
    let mut app = app();
    let selected_branch = Some("feature/super-long-branch-name".to_string());
    app.selected_branch = &selected_branch;
    app.git_diff_fullscreen = true;
    app.show_git_diff_summary = true;
    app.show_git_filter_options = true;
    app.git_worktree_menu_open = true;
    let selected_git_worktree_directory = Some("/repo/wt-two".to_string());
    app.selected_git_worktree_directory = &selected_git_worktree_directory;
    let options = [
        worktree("/repo/main", "main worktree · extra metadata", Some("main")),
        worktree(
            "/repo/wt-two",
            "secondary worktree with very long label · detail",
            Some("feature/extremely-long-branch"),
        ),
    ];
    app.git_worktree_options = &options;

    let _element = header::view(&app, vec!["src/lib.rs".to_string(), "src/main.rs".to_string()]);
}

#[test]
fn view_builds_worktree_picker_without_matching_selected_directory_or_branch() {
    let mut app = app();
    let selected_git_worktree_directory = Some("/repo/missing".to_string());
    app.selected_git_worktree_directory = &selected_git_worktree_directory;
    let options = [
        worktree("/repo/a", "alpha", None),
        worktree("/repo/b", "beta · ignored suffix", Some("")),
    ];
    app.git_worktree_options = &options;

    let _element = header::view(&app, Vec::new());
}
