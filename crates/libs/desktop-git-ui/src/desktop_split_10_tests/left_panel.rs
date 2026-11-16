use crate::git_panel::{left_panel, types::ConventionalCommitType};
fn input(editor: &iced::widget::text_editor::Content) -> left_panel::Input<'_> {
    left_panel::Input {
        desktop_locale: config_ui::DesktopLocale::default(),
        show_git_diff_summary: false,
        git_commit_type: None,
        git_commit_scope: "",
        git_commit_message: "",
        git_commit_description_editor: editor,
        git_commit_in_progress: false,
        file_manager_refresh_frame: 0,
        any_selected: false,
    }
}
#[test]
fn view_builds_disabled_state_when_nothing_selected() {
    let editor = iced::widget::text_editor::Content::new();
    let mut app = input(&editor);
    app.git_commit_message = "add panel tests";
    app.git_commit_type = Some(ConventionalCommitType::Feat);

    let _element = left_panel::view::<()>(&app, iced::widget::text("提交所选变更").into());
}

#[test]
fn view_builds_simple_mode_with_missing_summary() {
    let editor = iced::widget::text_editor::Content::new();
    let mut app = input(&editor);
    app.show_git_diff_summary = false;
    app.git_commit_message = "   ";
    app.any_selected = true;

    let _element = left_panel::view::<()>(&app, iced::widget::text("提交所选变更").into());
}

#[test]
fn view_builds_summary_mode_with_missing_summary() {
    let editor = iced::widget::text_editor::Content::new();
    let mut app = input(&editor);
    app.show_git_diff_summary = true;
    app.git_commit_type = None;
    app.git_commit_message = "   ";
    app.any_selected = true;

    let _element = left_panel::view::<()>(&app, iced::widget::text("提交所选变更").into());
}

#[test]
fn view_builds_summary_mode_with_missing_type() {
    let editor = iced::widget::text_editor::Content::new();
    let mut app = input(&editor);
    app.show_git_diff_summary = true;
    app.git_commit_type = None;
    app.git_commit_message = "cover left panel";
    app.any_selected = true;

    let _element = left_panel::view::<()>(&app, iced::widget::text("提交所选变更").into());
}

#[test]
fn view_builds_ready_state_from_hunk_selection() {
    let editor = iced::widget::text_editor::Content::new();
    let mut app = input(&editor);
    app.show_git_diff_summary = true;
    app.git_commit_type = Some(ConventionalCommitType::Fix);
    app.git_commit_scope = "git-panel";
    app.git_commit_message = "cover left panel";
    app.any_selected = true;

    let _element = left_panel::view::<()>(&app, iced::widget::text("提交所选变更").into());
}

#[test]
fn view_builds_ready_state_from_new_line_selection() {
    let editor = iced::widget::text_editor::Content::new();
    let mut app = input(&editor);
    app.show_git_diff_summary = false;
    app.git_commit_message = "cover selected line";
    app.any_selected = true;

    let _element = left_panel::view::<()>(&app, iced::widget::text("提交所选变更").into());
}

#[test]
fn view_builds_ready_state_from_old_line_selection() {
    let editor = iced::widget::text_editor::Content::new();
    let mut app = input(&editor);
    app.show_git_diff_summary = false;
    app.git_commit_message = "cover selected old line";
    app.any_selected = true;

    let _element = left_panel::view::<()>(&app, iced::widget::text("提交所选变更").into());
}

#[test]
fn view_builds_in_progress_commit_state() {
    let editor = iced::widget::text_editor::Content::new();
    let mut app = input(&editor);
    app.show_git_diff_summary = true;
    app.git_commit_type = Some(ConventionalCommitType::Fix);
    app.git_commit_scope = "git-panel";
    app.git_commit_message = "cover left panel";
    app.any_selected = true;

    app.git_commit_in_progress = true;
    app.file_manager_refresh_frame = 2;
    let _element = left_panel::view::<()>(&app, iced::widget::text("提交所选变更").into());
}
