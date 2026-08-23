//! 验证提交侧栏的选择来源、校验状态、帮助内容与按钮样式。
//! 通过文件、区块和新旧行选择验证提交就绪状态的优先级。

use iced::widget::{button, svg};
use iced::{Background, Color, Theme};

use crate::app::state::ConventionalCommitType;

#[test]
fn task_719_test_module_is_wired() {
    let path = std::path::Path::new(file!());
    assert!(path.ends_with("left_panel_tests.rs"));
}

fn test_app() -> crate::app::App {
    crate::app::App::new().0
}

fn assert_panel_state(
    app: &crate::app::App,
    hint: &'static str,
    button: super::left_panel::CommitButtonState,
) {
    let state = super::left_panel::commit_panel_state(app);
    assert_eq!(state.hint, hint);
    assert_eq!(state.button, button);
}

#[test]
fn help_copy_lists_commit_format_and_common_types() {
    assert_eq!(super::left_panel::COMMIT_HELP_TITLE, "约定式提交");

    for expected in [
        "约定式提交格式",
        "<类型>[可选作用域]: <摘要>",
        "[可选详细描述]",
        "核心提交类型",
        "feat:",
        "fix:",
        "docs:",
        "style:",
        "refactor:",
        "perf:",
        "test:",
        "build:",
        "ci:",
        "chore:",
        "revert:",
        "扩展与特殊类型",
        "init:",
        "config:",
        "release:",
        "deploy:",
        "merge:",
        "wip:",
        "typo:",
        "locale:",
    ] {
        assert!(super::left_panel::COMMIT_HELP_TEXT.contains(expected), "{expected}");
    }
}

#[test]
fn selected_changes_detects_each_selection_source() {
    let initial_app = test_app();
    assert!(!super::left_panel::has_selected_changes(&initial_app));

    let mut files_app = test_app();
    files_app.staged_files_selected.push("src/lib.rs".to_string());
    assert!(super::left_panel::has_selected_changes(&files_app));

    let mut hunks_app = test_app();
    hunks_app.staged_hunks_selected.push(("src/lib.rs".to_string(), 1));
    assert!(super::left_panel::has_selected_changes(&hunks_app));

    let mut lines_app = test_app();
    lines_app.staged_lines_selected.push(("src/lib.rs".to_string(), 7));
    assert!(super::left_panel::has_selected_changes(&lines_app));

    let mut app = test_app();
    app.staged_old_lines_selected.push(("src/lib.rs".to_string(), 3));
    assert!(super::left_panel::has_selected_changes(&app));
}

#[test]
fn commit_panel_state_covers_priority_and_validation_paths() {
    let mut app = test_app();
    app.show_git_diff_summary = true;
    app.git_commit_in_progress = true;
    app.git_commit_message = "   ".to_string();
    assert_panel_state(&app, "正在提交已选变更", super::left_panel::CommitButtonState::InProgress);

    let mut no_selection_app = test_app();
    no_selection_app.git_commit_message = "cover left panel".to_string();
    no_selection_app.git_commit_type = Some(ConventionalCommitType::Feat);
    assert_panel_state(
        &no_selection_app,
        "请选择至少一项变更后再提交",
        super::left_panel::CommitButtonState::NoSelection,
    );

    let mut app = test_app();
    app.show_git_diff_summary = false;
    app.git_commit_message = "   ".to_string();
    app.staged_files_selected.push("src/lib.rs".to_string());
    assert_panel_state(
        &app,
        "填写摘要后即可提交",
        super::left_panel::CommitButtonState::MissingSummary,
    );

    let mut app = test_app();
    app.show_git_diff_summary = true;
    app.git_commit_type = None;
    app.git_commit_message = "cover left panel".to_string();
    app.staged_files_selected.push("src/lib.rs".to_string());
    assert_panel_state(
        &app,
        "选择提交类型后即可提交",
        super::left_panel::CommitButtonState::MissingType,
    );

    let mut app = test_app();
    app.show_git_diff_summary = false;
    app.git_commit_message = "cover left panel".to_string();
    app.staged_files_selected.push("src/lib.rs".to_string());
    assert_panel_state(
        &app,
        "已满足提交条件，可直接提交",
        super::left_panel::CommitButtonState::Ready,
    );
}

#[test]
fn commit_panel_state_is_ready_for_complete_conventional_commit() {
    let mut app = test_app();
    app.show_git_diff_summary = true;
    app.git_commit_type = Some(ConventionalCommitType::Fix);
    app.git_commit_message = "  cover complete commit form  ".to_string();
    app.staged_old_lines_selected.push(("src/lib.rs".to_string(), 3));

    assert_panel_state(
        &app,
        "已满足提交条件，可直接提交",
        super::left_panel::CommitButtonState::Ready,
    );
}

#[test]
fn help_icon_style_uses_muted_text_color_for_all_statuses() {
    let theme = Theme::Dark;
    let expected = Some(theme.palette().text.scale_alpha(0.78));

    assert_eq!(super::left_panel::help_icon_style(&theme, svg::Status::Idle).color, expected);
    assert_eq!(super::left_panel::help_icon_style(&theme, svg::Status::Hovered).color, expected);
}

#[test]
fn help_button_style_covers_idle_hovered_pressed_and_disabled_statuses() {
    let theme = Theme::Dark;
    let active = super::left_panel::help_button_style(&theme, button::Status::Active);
    let hovered = super::left_panel::help_button_style(&theme, button::Status::Hovered);
    let pressed = super::left_panel::help_button_style(&theme, button::Status::Pressed);
    let disabled = super::left_panel::help_button_style(&theme, button::Status::Disabled);

    assert!(active.background.is_none());
    assert!(disabled.background.is_none());
    assert_eq!(
        hovered.background,
        Some(Background::Color(theme.extended_palette().background.weak.color.scale_alpha(0.72)))
    );
    assert_eq!(
        pressed.background,
        Some(Background::Color(theme.extended_palette().background.weak.color.scale_alpha(0.72)))
    );
    assert_eq!(pressed.text_color, theme.palette().text);
    assert_eq!(pressed.border.color, Color::TRANSPARENT);
}

#[test]
fn view_builds_disabled_state_when_nothing_selected() {
    let mut app = test_app();
    app.git_commit_message = "add panel tests".to_string();
    app.git_commit_type = Some(ConventionalCommitType::Feat);

    let _element = super::left_panel::view(&app);
}

#[test]
fn view_builds_simple_mode_with_missing_summary() {
    let mut app = test_app();
    app.show_git_diff_summary = false;
    app.git_commit_message = "   ".to_string();
    app.staged_files_selected.push("src/lib.rs".to_string());

    let _element = super::left_panel::view(&app);
}

#[test]
fn view_builds_summary_mode_with_missing_summary() {
    let mut app = test_app();
    app.show_git_diff_summary = true;
    app.git_commit_type = None;
    app.git_commit_message = "   ".to_string();
    app.staged_files_selected.push("src/lib.rs".to_string());

    let _element = super::left_panel::view(&app);
}

#[test]
fn view_builds_summary_mode_with_missing_type() {
    let mut app = test_app();
    app.show_git_diff_summary = true;
    app.git_commit_type = None;
    app.git_commit_message = "cover left panel".to_string();
    app.staged_files_selected.push("src/lib.rs".to_string());

    let _element = super::left_panel::view(&app);
}

#[test]
fn view_builds_ready_state_from_hunk_selection() {
    let mut app = test_app();
    app.show_git_diff_summary = true;
    app.git_commit_type = Some(ConventionalCommitType::Fix);
    app.git_commit_scope = "git-panel".to_string();
    app.git_commit_message = "cover left panel".to_string();
    app.staged_hunks_selected.push(("src/lib.rs".to_string(), 1));

    let _element = super::left_panel::view(&app);
}

#[test]
fn view_builds_ready_state_from_new_line_selection() {
    let mut app = test_app();
    app.show_git_diff_summary = false;
    app.git_commit_message = "cover selected line".to_string();
    app.staged_lines_selected.push(("src/lib.rs".to_string(), 7));

    let _element = super::left_panel::view(&app);
}

#[test]
fn view_builds_ready_state_from_old_line_selection() {
    let mut app = test_app();
    app.show_git_diff_summary = false;
    app.git_commit_message = "cover selected old line".to_string();
    app.staged_old_lines_selected.push(("src/lib.rs".to_string(), 3));

    let _element = super::left_panel::view(&app);
}

#[test]
fn view_builds_in_progress_commit_state() {
    let mut app = test_app();
    app.show_git_diff_summary = true;
    app.git_commit_type = Some(ConventionalCommitType::Fix);
    app.git_commit_scope = "git-panel".to_string();
    app.git_commit_message = "cover left panel".to_string();
    app.staged_hunks_selected.push(("src/lib.rs".to_string(), 1));

    app.git_commit_in_progress = true;
    app.file_manager_refresh_frame = 2;
    let _element = super::left_panel::view(&app);
}
