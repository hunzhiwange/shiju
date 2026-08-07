use super::tree_list::{
    DirNode, build_changes_list, build_file_tree_list, change_list_expand_keys,
    changes_directory_button_style, changes_file_button_style, insert_path,
    tree_directory_button_style, tree_file_button_style,
};
use crate::app::App;
use crate::app::components::file_tree::model::build_file_tree_model;
use iced::widget::button;
use iced::{Background, Color, Theme};

fn background_color(style: button::Style) -> Option<Color> {
    match style.background {
        Some(Background::Color(color)) => Some(color),
        _ => None,
    }
}

fn primary_with_alpha(theme: &Theme, alpha: f32) -> Color {
    let primary = theme.palette().primary;
    Color::from_rgba(primary.r, primary.g, primary.b, alpha)
}

#[test]
fn insert_path_ignores_empty_paths_and_groups_root_and_nested_files() {
    let mut tree = DirNode::default();

    for path in ["", "/", "README.md", "src/main.rs", "src/bin/tool.rs", "src//lib.rs"] {
        insert_path(&mut tree, path);
    }

    assert_eq!(tree.files, vec!["README.md"]);
    let src = tree.children.get("src").expect("src directory should be created");
    assert_eq!(src.files, vec!["src/main.rs", "src//lib.rs"]);
    assert_eq!(src.children["bin"].files, vec!["src/bin/tool.rs"]);
}

#[test]
fn change_list_expand_keys_use_chg_prefix_and_all_ancestor_dirs() {
    let keys = change_list_expand_keys(["README.md", "src/main.rs", "src/nested/lib.rs", ""]);
    assert_eq!(
        keys.into_iter().collect::<Vec<_>>(),
        vec!["chg:src".to_string(), "chg:src/nested".to_string()]
    );
}

#[test]
fn insert_path_normalizes_leading_and_repeated_separators_for_directory_segments() {
    let mut tree = DirNode::default();

    insert_path(&mut tree, "/src/generated/schema.rs");
    insert_path(&mut tree, "src///generated/client.rs");

    let generated = &tree.children["src"].children["generated"];
    assert_eq!(generated.files, vec!["/src/generated/schema.rs", "src///generated/client.rs"]);
}

#[test]
fn tree_directory_style_distinguishes_active_hovered_and_pressed_states() {
    let theme = Theme::Dark;

    let active = tree_directory_button_style(&theme, button::Status::Active);
    let hovered = tree_directory_button_style(&theme, button::Status::Hovered);
    let pressed = tree_directory_button_style(&theme, button::Status::Pressed);

    assert_eq!(active.text_color, theme.palette().text);
    assert_eq!(active.border.width, 0.0);
    assert_eq!(active.border.color, Color::TRANSPARENT);
    assert_eq!(background_color(active), None);
    assert_eq!(background_color(hovered), Some(primary_with_alpha(&theme, 0.10)));
    assert_eq!(background_color(pressed), Some(primary_with_alpha(&theme, 0.18)));
}

#[test]
fn tree_file_style_preserves_selection_and_interaction_precedence() {
    let theme = Theme::Light;

    assert_eq!(
        background_color(tree_file_button_style(&theme, button::Status::Active, false)),
        None
    );
    assert_eq!(
        background_color(tree_file_button_style(&theme, button::Status::Active, true)),
        Some(primary_with_alpha(&theme, 0.18))
    );
    assert_eq!(
        background_color(tree_file_button_style(&theme, button::Status::Hovered, true)),
        Some(primary_with_alpha(&theme, 0.10))
    );
    assert_eq!(
        background_color(tree_file_button_style(&theme, button::Status::Pressed, false)),
        Some(primary_with_alpha(&theme, 0.18))
    );
    assert_eq!(
        background_color(tree_file_button_style(&theme, button::Status::Disabled, true)),
        Some(primary_with_alpha(&theme, 0.18))
    );
}

#[test]
fn changes_styles_use_hover_color_for_interaction_and_selection() {
    let theme = Theme::Dark;
    let expected = Some(primary_with_alpha(&theme, 0.10));

    assert_eq!(
        background_color(changes_directory_button_style(&theme, button::Status::Active)),
        None
    );
    assert_eq!(
        background_color(changes_directory_button_style(&theme, button::Status::Hovered)),
        expected
    );
    assert_eq!(
        background_color(changes_directory_button_style(&theme, button::Status::Pressed)),
        expected
    );
    assert_eq!(
        background_color(changes_file_button_style(&theme, button::Status::Active, false)),
        None
    );
    assert_eq!(
        background_color(changes_file_button_style(&theme, button::Status::Active, true)),
        expected
    );
    assert_eq!(
        background_color(changes_file_button_style(&theme, button::Status::Hovered, false)),
        expected
    );
    assert_eq!(
        background_color(changes_file_button_style(&theme, button::Status::Disabled, true)),
        expected
    );
}

#[test]
fn tree_list_builds_empty_states_with_and_without_project() {
    let app = App::new().0;
    let _ = build_file_tree_list(&app);

    let mut app = App::new().0;
    app.project_path = Some("/tmp/demo".to_string());
    app.file_tree_model_cache
        .insert("/tmp/demo".to_string(), build_file_tree_model("/tmp/demo", &[]));
    let _ = build_file_tree_list(&app);
}

#[test]
fn tree_list_builds_fallback_model_with_expanded_selection_and_file_menu() {
    let mut app = App::new().0;
    app.project_path = Some("/tmp/demo".to_string());
    app.file_tree_model_cache
        .insert("/tmp/demo".to_string(), build_file_tree_model("/tmp/demo", &[]));
    app.file_index_cache.insert(
        "/tmp/demo".to_string(),
        vec![
            "/tmp/demo/README.md".to_string(),
            "/tmp/demo/src/main.rs".to_string(),
            "/tmp/demo/src/nested/lib.rs".to_string(),
        ],
    );
    app.file_tree_expanded_set.insert("src".to_string());
    app.file_tree_expanded_set.insert("src/nested".to_string());
    app.active_preview_path = Some("/tmp/demo/src/main.rs".to_string());
    app.file_tree_menu_path = Some("/tmp/demo/src/main.rs".to_string());
    app.file_tree_menu_source = Some("tree-file".to_string());
    app.file_tree_menu_anchor = Some(iced::Point::new(12.0, 24.0));

    let _ = build_file_tree_list(&app);
}

#[test]
fn tree_list_builds_cached_model_with_directory_menu() {
    let mut app = App::new().0;
    app.project_path = Some("/tmp/demo".to_string());
    let files = vec!["/tmp/demo/src/main.rs".to_string()];
    app.file_tree_model_cache
        .insert("/tmp/demo".to_string(), build_file_tree_model("/tmp/demo", &files));
    app.file_tree_expanded_set.insert("src".to_string());
    app.file_tree_menu_path = Some("/tmp/demo/src".to_string());
    app.file_tree_menu_source = Some("tree-dir".to_string());

    let _ = build_file_tree_list(&app);
}

#[test]
fn changes_list_builds_loading_no_changes_and_empty_path_states() {
    let mut app = App::new().0;
    let _ = build_changes_list(&app);

    app.git_changed_files_loading = true;
    let _ = build_changes_list(&app);

    app.git_changed_files_loading = false;
    app.git_changed_files = vec![String::new(), "/".to_string()];
    let _ = build_changes_list(&app);
}

#[test]
fn changes_list_builds_root_and_nested_files_with_directory_menu() {
    let mut app = App::new().0;
    app.project_path = Some("/tmp/demo".to_string());
    app.git_changed_files =
        vec!["README.md".to_string(), "src/main.rs".to_string(), "src/nested/lib.rs".to_string()];
    app.file_tree_expanded_set.insert("chg:src".to_string());
    app.git_focused_file = Some("src/main.rs".to_string());
    app.file_tree_menu_path = Some("/tmp/demo/src/nested".to_string());
    app.file_tree_menu_source = Some("changes-dir".to_string());
    app.file_tree_menu_anchor = Some(iced::Point::new(8.0, 16.0));

    let _ = build_changes_list(&app);
}

#[test]
fn changes_list_builds_expanded_file_menu_for_selected_file() {
    let mut app = App::new().0;
    app.project_path = Some("/tmp/demo".to_string());
    app.git_changed_files = vec!["src/main.rs".to_string()];
    app.file_tree_expanded_set.insert("chg:src".to_string());
    app.git_focused_file = Some("src/main.rs".to_string());
    app.file_tree_menu_path = Some("/tmp/demo/src/main.rs".to_string());
    app.file_tree_menu_source = Some("changes-file".to_string());

    let _ = build_changes_list(&app);
}

#[test]
fn changes_list_builds_root_file_menu_without_project_path() {
    let mut app = App::new().0;
    app.git_changed_files = vec!["README.md".to_string()];
    app.git_focused_file = Some("README.md".to_string());
    app.file_tree_menu_path = Some("README.md".to_string());
    app.file_tree_menu_source = Some("changes-file".to_string());
    app.file_tree_menu_anchor = Some(iced::Point::new(4.0, 6.0));

    let element = build_changes_list(&app);

    assert_eq!(element.as_widget().size().width, iced::Length::Fill);
}

#[test]
fn tree_lists_build_collapsed_directories_and_ignore_mismatched_menu_sources() {
    let mut app = App::new().0;
    app.project_path = Some("/tmp/demo".to_string());
    let files = vec!["/tmp/demo/src/nested/lib.rs".to_string()];
    app.file_tree_model_cache
        .insert("/tmp/demo".to_string(), build_file_tree_model("/tmp/demo", &files));
    app.file_tree_menu_path = Some("/tmp/demo/src".to_string());
    app.file_tree_menu_source = Some("changes-dir".to_string());

    let _collapsed_tree = build_file_tree_list(&app);
    drop(_collapsed_tree);

    app.git_changed_files = vec!["src/nested/lib.rs".to_string()];
    app.file_tree_menu_source = Some("tree-dir".to_string());

    let _collapsed_changes = build_changes_list(&app);
}
