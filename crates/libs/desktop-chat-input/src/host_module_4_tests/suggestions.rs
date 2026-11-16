use crate::input_panel::suggestions::State;
use iced::widget::text_editor;
use std::sync::Arc;
fn paste(state: &mut State, editor: &mut text_editor::Content, value: String) {
    state.editor_action(
        editor,
        text_editor::Action::Edit(text_editor::Edit::Paste(std::sync::Arc::new(value))),
        true,
        false,
    );
}
#[test]
fn file_search_input_changed_toggles_visibility_and_resets_selection() {
    let mut app = State::default();
    app.file_search_selected_index = 4;

    app.change_files(" src ".to_string());

    assert!(app.show_file_search);
    assert_eq!(app.file_search_query, " src ");
    assert_eq!(app.file_search_selected_index, 0);

    app.file_search_selected_index = 3;
    app.change_files("   ".to_string());

    assert!(!app.show_file_search);
    assert_eq!(app.file_search_selected_index, 0);
}

#[test]
fn input_editor_invalid_search_character_clears_active_search() {
    let mut app = State::default();
    let mut editor = text_editor::Content::new();

    paste(&mut app, &mut editor, "@src".to_string());
    assert!(app.show_file_search);

    paste(&mut app, &mut editor, "!".to_string());

    assert!(!app.show_file_search);
    assert!(app.file_search_query.is_empty());
    assert_eq!(app.file_search_selected_index, 0);
}

#[test]
fn file_search_navigation_limits_selection_to_twenty_visible_results() {
    let mut app = State::default();

    for _ in 0..30 {
        app.navigate_file(true, 25);
    }

    assert_eq!(app.file_search_selected_index, 19);
}

fn set_text(editor: &mut text_editor::Content, text: &str) {
    *editor = text_editor::Content::with_text(text);
    editor.perform(text_editor::Action::Move(text_editor::Motion::DocumentEnd));
}
fn edit(state: &mut State, editor: &mut text_editor::Content, action: text_editor::Action) {
    state.editor_action(editor, action, true, false);
}
fn select(state: &mut State, editor: &mut text_editor::Content, root: Option<&str>, path: String) {
    state.select_file(editor, root, &path);
}
#[test]
fn selecting_file_search_path_replaces_active_mention_or_appends() {
    let mut state = State::default();
    let mut editor = text_editor::Content::new();
    let root = Some("/workspace".to_string());

    set_text(&mut editor, "open @sr");
    select(&mut state, &mut editor, root.as_deref(), "/workspace/src/main.rs".to_string());

    assert_eq!(editor.text(), "open @src/main.rs ");
    assert!(!state.show_file_search);
    assert!(state.file_search_query.is_empty());

    set_text(&mut editor, "plain text");
    select(&mut state, &mut editor, root.as_deref(), "tests/input.rs".to_string());

    assert_eq!(editor.text(), "plain text @tests/input.rs ");
}

#[test]
fn selecting_file_search_path_without_project_root_normalizes_backslashes() {
    let mut state = State::default();
    let mut editor = text_editor::Content::new();
    let root: Option<String> = None;
    set_text(&mut editor, "open @C");

    select(&mut state, &mut editor, root.as_deref(), "C:\\workspace\\src\\main.rs".to_string());

    assert_eq!(editor.text(), "open @C:/workspace/src/main.rs ");
    assert!(!state.show_file_search);
}

#[test]
fn selecting_and_removing_spaced_chinese_file_preserves_other_mentions() {
    let mut state = State::default();
    let mut editor = text_editor::Content::new();
    let root: Option<String> = None;
    let path = "已粘贴 2026-11-03 下午3.06.58.textClipping";
    set_text(&mut editor, "@已");
    select(&mut state, &mut editor, root.as_deref(), path.to_string());
    let input = editor.text();
    assert_eq!(
        input,
        format!("{} ", desktop_editor::components::input_mention_highlighter::file_mention(path))
    );
    crate::input_panel::file_references::remove_file_reference(&mut editor, path);
    assert!(editor.text().is_empty());
}

#[test]
fn input_editor_actions_open_and_clear_file_search() {
    let mut state = State::default();
    let mut editor = text_editor::Content::new();

    paste(&mut state, &mut editor, "@src".to_string());

    assert!(state.show_file_search);
    assert_eq!(state.file_search_query, "src");

    edit(
        &mut state,
        &mut editor,
        text_editor::Action::Edit(text_editor::Edit::Paste(Arc::new(" ".to_string()))),
    );

    assert!(!state.show_file_search);
    assert!(state.file_search_query.is_empty());

    paste(&mut state, &mut editor, String::new());
    edit(&mut state, &mut editor, text_editor::Action::Edit(text_editor::Edit::Backspace));
    edit(&mut state, &mut editor, text_editor::Action::Edit(text_editor::Edit::Delete));
    edit(&mut state, &mut editor, text_editor::Action::Move(text_editor::Motion::DocumentEnd));
    edit(&mut state, &mut editor, text_editor::Action::Select(text_editor::Motion::DocumentStart));
}

#[test]
fn slash_palette_opens_on_trailing_slash_and_closes_on_whitespace() {
    let mut state = State::default();
    let mut editor = text_editor::Content::new();

    paste(&mut state, &mut editor, "/".to_string());

    assert!(state.show_slash_palette);
    assert_eq!(state.slash_palette_query, "");
    assert!(!state.show_file_search);

    edit(
        &mut state,
        &mut editor,
        text_editor::Action::Edit(text_editor::Edit::Paste(Arc::new(" ".to_string()))),
    );

    assert!(!state.show_slash_palette);
    assert!(state.slash_palette_query.is_empty());
}

#[test]
fn slash_palette_and_file_search_are_mutually_exclusive() {
    let mut state = State::default();
    let mut editor = text_editor::Content::new();

    paste(&mut state, &mut editor, "@src".to_string());
    assert!(state.show_file_search);
    assert!(!state.show_slash_palette);

    // Replace input with slash query.
    {
        set_text(&mut editor, "");
    }
    paste(&mut state, &mut editor, "/gpt".to_string());
    assert!(state.show_slash_palette);
    assert!(!state.show_file_search);

    {
        set_text(&mut editor, "");
    }
    paste(&mut state, &mut editor, "@src".to_string());
    assert!(state.show_file_search);
    assert!(!state.show_slash_palette);
}
