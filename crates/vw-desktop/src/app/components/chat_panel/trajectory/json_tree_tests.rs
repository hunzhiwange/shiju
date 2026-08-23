//! 验证 JSON 树的稳定路径、折叠范围、复制操作与非 ASCII 字体安全。
//! 折叠只隐藏目标子树，悬停复制按钮不得改变相邻节点的投影和标识。

use std::collections::HashSet;

use iced::advanced::widget::Tree;
use iced::widget::scrollable::{Direction, Scrollbar};
use iced::{Font, Theme};
use serde_json::json;

use super::json_tree::{
    JSON_TREE_SCROLLBAR_WIDTH, JsonTreeLineKind, JsonTreeTokenKind, json_token_font, json_tree,
    json_tree_copy_visible_in_layer, json_tree_scroll_direction, project_json_tree,
};
use super::styles::json_tree_token_color;

#[derive(Debug, Clone, PartialEq, Eq)]
enum TestMessage {
    Toggle(String),
    Copy(String),
}

#[test]
fn json_tree_projects_stable_paths_and_tokens() {
    let value = json!({
        "z": true,
        "a": {"text": "hello", "number": 3.5},
        "items": [null, false],
    });

    let rows = project_json_tree(&value, &HashSet::new());
    assert_eq!(
        rows.iter().map(|row| (row.path.as_str(), row.kind)).collect::<Vec<_>>(),
        [
            ("", JsonTreeLineKind::Open),
            ("/a", JsonTreeLineKind::Open),
            ("/a/number", JsonTreeLineKind::Leaf),
            ("/a/text", JsonTreeLineKind::Leaf),
            ("/a", JsonTreeLineKind::Close),
            ("/items", JsonTreeLineKind::Open),
            ("/items/0", JsonTreeLineKind::Leaf),
            ("/items/1", JsonTreeLineKind::Leaf),
            ("/items", JsonTreeLineKind::Close),
            ("/z", JsonTreeLineKind::Leaf),
            ("", JsonTreeLineKind::Close),
        ]
    );
    assert!(rows[1].expandable);
    assert!(rows[2].tokens.iter().any(|token| token.kind == JsonTreeTokenKind::Number));
    assert!(rows[3].tokens.iter().any(|token| token.kind == JsonTreeTokenKind::String));
    assert!(rows[6].tokens.iter().any(|token| token.kind == JsonTreeTokenKind::Null));
    assert!(rows[7].tokens.iter().any(|token| token.kind == JsonTreeTokenKind::Boolean));
}

#[test]
fn json_tree_collapse_hides_only_selected_subtree() {
    let value = json!({"a": {"hidden": [1, 2]}, "b": {"visible": 3}});
    let collapsed = HashSet::from(["/a".to_string()]);

    let rows = project_json_tree(&value, &collapsed);
    assert_eq!(
        rows.iter().map(|row| (row.path.as_str(), row.kind)).collect::<Vec<_>>(),
        [
            ("", JsonTreeLineKind::Open),
            ("/a", JsonTreeLineKind::Collapsed),
            ("/b", JsonTreeLineKind::Open),
            ("/b/visible", JsonTreeLineKind::Leaf),
            ("/b", JsonTreeLineKind::Close),
            ("", JsonTreeLineKind::Close),
        ]
    );
    assert_eq!(rows[1].copy_value, json!({"hidden": [1, 2]}));

    let element = json_tree(&value, &collapsed, TestMessage::Toggle, |value| {
        TestMessage::Copy(value.to_string())
    });
    assert!(!Tree::new(element.as_widget()).children.is_empty());
}

#[test]
fn json_tree_uses_safe_fonts_for_non_ascii_values() {
    let rows = project_json_tree(&json!({"中文键": "中文值", "ascii": "value"}), &HashSet::new());
    let chinese = rows.iter().find(|row| row.path == "/中文键").unwrap();
    assert!(
        chinese
            .tokens
            .iter()
            .filter(|token| matches!(
                token.kind,
                JsonTreeTokenKind::Key | JsonTreeTokenKind::String
            ))
            .all(|token| token.font == Font::DEFAULT)
    );
    let ascii = rows.iter().find(|row| row.path == "/ascii").unwrap();
    assert_eq!(ascii.tokens.last().unwrap().font, Font::MONOSPACE);
    assert_eq!(json_token_font("纯中文"), Font::DEFAULT);
    assert_eq!(json_token_font("ascii"), Font::MONOSPACE);
    assert_eq!(JSON_TREE_SCROLLBAR_WIDTH, 4.0);
    assert_eq!(
        json_tree_scroll_direction(),
        Direction::Horizontal(
            Scrollbar::new()
                .width(JSON_TREE_SCROLLBAR_WIDTH)
                .scroller_width(JSON_TREE_SCROLLBAR_WIDTH)
        )
    );

    for theme in [Theme::Light, Theme::Dark] {
        let punctuation = json_tree_token_color(&theme, JsonTreeTokenKind::Punctuation);
        for kind in [
            JsonTreeTokenKind::Key,
            JsonTreeTokenKind::String,
            JsonTreeTokenKind::Number,
            JsonTreeTokenKind::Boolean,
            JsonTreeTokenKind::Null,
        ] {
            let color = json_tree_token_color(&theme, kind);
            assert!(color.a > 0.0);
            assert_ne!(color, punctuation);
        }
    }
}

#[test]
fn json_tree_copy_action_is_only_visible_on_hover() {
    assert!(!json_tree_copy_visible_in_layer(false, JsonTreeLineKind::Open));
    assert!(!json_tree_copy_visible_in_layer(false, JsonTreeLineKind::Leaf));
    assert!(json_tree_copy_visible_in_layer(true, JsonTreeLineKind::Open));
    assert!(json_tree_copy_visible_in_layer(true, JsonTreeLineKind::Leaf));
    assert!(!json_tree_copy_visible_in_layer(true, JsonTreeLineKind::Close));
}
