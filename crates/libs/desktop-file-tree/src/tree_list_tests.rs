use super::tree_list::*;
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
fn file_tree_drop_directory_highlight_supports_both_themes() {
    for theme in [Theme::Light, Theme::Dark] {
        let style =
            super::tree_list::file_tree_drop_directory_style(&theme, button::Status::Active, true);
        assert_eq!(style.text_color, theme.palette().text);
        assert_eq!(background_color(style), Some(primary_with_alpha(&theme, 0.24)));
        assert_eq!(
            background_color(super::tree_list::file_tree_drop_directory_style(
                &theme,
                button::Status::Active,
                false
            )),
            None
        );
    }
}
