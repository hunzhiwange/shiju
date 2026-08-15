//! 轨迹详情使用的无状态 JSON 树投影与视图。

use std::collections::HashSet;

use iced::widget::scrollable::{Direction, Scrollbar};
use iced::widget::{Space, button, column, container, row, scrollable, text};
use iced::{Element, Font, Length, Theme};
use serde_json::Value;

use super::styles;

pub(crate) const JSON_TREE_SCROLLBAR_WIDTH: f32 = 4.0;
const JSON_TREE_INDENT: f32 = 16.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum JsonTreeLineKind {
    Open,
    Leaf,
    Close,
    Collapsed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum JsonTreeTokenKind {
    Punctuation,
    Key,
    String,
    Number,
    Boolean,
    Null,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct JsonTreeToken {
    pub(crate) kind: JsonTreeTokenKind,
    pub(crate) text: String,
    pub(crate) font: Font,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct JsonTreeRow {
    pub(crate) path: String,
    pub(crate) depth: usize,
    pub(crate) kind: JsonTreeLineKind,
    pub(crate) expandable: bool,
    pub(crate) expanded: bool,
    pub(crate) tokens: Vec<JsonTreeToken>,
    pub(crate) copy_value: Value,
}

pub(crate) fn project_json_tree(
    value: &Value,
    collapsed_paths: &HashSet<String>,
) -> Vec<JsonTreeRow> {
    let mut rows = Vec::new();
    project_value(
        value,
        "",
        None,
        0,
        true,
        collapsed_paths,
        &mut rows,
    );
    rows
}

#[allow(clippy::too_many_arguments)]
fn project_value(
    value: &Value,
    path: &str,
    key: Option<&str>,
    depth: usize,
    is_last: bool,
    collapsed_paths: &HashSet<String>,
    rows: &mut Vec<JsonTreeRow>,
) {
    let container_delimiters = match value {
        Value::Object(_) => Some(("{", "}")),
        Value::Array(_) => Some(("[", "]")),
        _ => None,
    };
    let Some((open, close)) = container_delimiters else {
        rows.push(JsonTreeRow {
            path: path.to_string(),
            depth,
            kind: JsonTreeLineKind::Leaf,
            expandable: false,
            expanded: false,
            tokens: leaf_tokens(key, value, is_last),
            copy_value: value.clone(),
        });
        return;
    };

    let empty = match value {
        Value::Object(object) => object.is_empty(),
        Value::Array(array) => array.is_empty(),
        _ => false,
    };
    if empty {
        let mut tokens = key_tokens(key);
        tokens.push(token(format!("{open}{close}"), JsonTreeTokenKind::Punctuation));
        push_comma(&mut tokens, is_last);
        rows.push(JsonTreeRow {
            path: path.to_string(),
            depth,
            kind: JsonTreeLineKind::Leaf,
            expandable: false,
            expanded: false,
            tokens,
            copy_value: value.clone(),
        });
        return;
    }

    if collapsed_paths.contains(path) {
        let mut tokens = key_tokens(key);
        tokens.push(token(format!("{open}…{close}"), JsonTreeTokenKind::Punctuation));
        push_comma(&mut tokens, is_last);
        rows.push(JsonTreeRow {
            path: path.to_string(),
            depth,
            kind: JsonTreeLineKind::Collapsed,
            expandable: true,
            expanded: false,
            tokens,
            copy_value: value.clone(),
        });
        return;
    }

    let mut tokens = key_tokens(key);
    tokens.push(token(open, JsonTreeTokenKind::Punctuation));
    rows.push(JsonTreeRow {
        path: path.to_string(),
        depth,
        kind: JsonTreeLineKind::Open,
        expandable: true,
        expanded: true,
        tokens,
        copy_value: value.clone(),
    });

    match value {
        Value::Object(object) => {
            let mut entries = object.iter().collect::<Vec<_>>();
            entries.sort_by(|(left, _), (right, _)| left.cmp(right));
            let last_index = entries.len().saturating_sub(1);
            for (index, (child_key, child)) in entries.into_iter().enumerate() {
                let child_path = format!("{path}/{}", escape_json_pointer(child_key));
                project_value(
                    child,
                    &child_path,
                    Some(child_key),
                    depth + 1,
                    index == last_index,
                    collapsed_paths,
                    rows,
                );
            }
        }
        Value::Array(array) => {
            let last_index = array.len().saturating_sub(1);
            for (index, child) in array.iter().enumerate() {
                let child_path = format!("{path}/{index}");
                project_value(
                    child,
                    &child_path,
                    None,
                    depth + 1,
                    index == last_index,
                    collapsed_paths,
                    rows,
                );
            }
        }
        _ => unreachable!("container delimiters require an object or array"),
    }

    let mut tokens = vec![token(close, JsonTreeTokenKind::Punctuation)];
    push_comma(&mut tokens, is_last);
    rows.push(JsonTreeRow {
        path: path.to_string(),
        depth,
        kind: JsonTreeLineKind::Close,
        expandable: false,
        expanded: false,
        tokens,
        copy_value: value.clone(),
    });
}

fn leaf_tokens(key: Option<&str>, value: &Value, is_last: bool) -> Vec<JsonTreeToken> {
    let mut tokens = key_tokens(key);
    let kind = match value {
        Value::String(_) => JsonTreeTokenKind::String,
        Value::Number(_) => JsonTreeTokenKind::Number,
        Value::Bool(_) => JsonTreeTokenKind::Boolean,
        Value::Null => JsonTreeTokenKind::Null,
        Value::Object(_) | Value::Array(_) => unreachable!("containers are projected separately"),
    };
    tokens.push(token(value.to_string(), kind));
    push_comma(&mut tokens, is_last);
    tokens
}

fn key_tokens(key: Option<&str>) -> Vec<JsonTreeToken> {
    let Some(key) = key else {
        return Vec::new();
    };
    vec![
        token(
            serde_json::to_string(key).unwrap_or_else(|_| format!("\"{key}\"")),
            JsonTreeTokenKind::Key,
        ),
        token(": ", JsonTreeTokenKind::Punctuation),
    ]
}

fn push_comma(tokens: &mut Vec<JsonTreeToken>, is_last: bool) {
    if !is_last {
        tokens.push(token(",", JsonTreeTokenKind::Punctuation));
    }
}

fn token(text: impl Into<String>, kind: JsonTreeTokenKind) -> JsonTreeToken {
    let text = text.into();
    JsonTreeToken { kind, font: json_token_font(&text), text }
}

fn escape_json_pointer(segment: &str) -> String {
    segment.replace('~', "~0").replace('/', "~1")
}

pub(crate) const fn json_token_font(token: &str) -> Font {
    if token.is_ascii() { Font::MONOSPACE } else { Font::DEFAULT }
}

pub(crate) fn json_tree_scroll_direction() -> Direction {
    Direction::Horizontal(
        Scrollbar::new()
            .width(JSON_TREE_SCROLLBAR_WIDTH)
            .scroller_width(JSON_TREE_SCROLLBAR_WIDTH),
    )
}

pub(crate) fn json_tree<'a, Message: Clone + 'a>(
    value: &Value,
    collapsed_paths: &HashSet<String>,
    on_toggle: impl Fn(String) -> Message + Clone + 'a,
    on_copy: impl Fn(Value) -> Message + Clone + 'a,
) -> Element<'a, Message> {
    let mut content = column![].spacing(2).width(Length::Shrink);
    for tree_row in project_json_tree(value, collapsed_paths) {
        let toggle: Element<'a, Message> = if tree_row.expandable {
            let marker = if tree_row.expanded { "▾" } else { "▸" };
            button(text(marker).font(Font::DEFAULT).size(11))
                .padding(0)
                .style(styles::json_tree_action_style)
                .on_press(on_toggle.clone()(tree_row.path.clone()))
                .into()
        } else {
            Space::new().width(Length::Fixed(13.0)).into()
        };
        let mut tokens = row![].spacing(0).align_y(iced::Alignment::Center);
        for tree_token in tree_row.tokens {
            let token_kind = tree_token.kind;
            tokens = tokens.push(
                text(tree_token.text)
                    .font(tree_token.font)
                    .size(11)
                    .style(move |theme: &Theme| iced::widget::text::Style {
                        color: Some(styles::json_tree_token_color(theme, token_kind)),
                    }),
            );
        }
        let copy: Element<'a, Message> = if tree_row.kind == JsonTreeLineKind::Close {
            Space::new().width(Length::Fixed(31.0)).into()
        } else {
            button(text("复制").font(Font::DEFAULT).size(10))
                .padding([1, 4])
                .style(styles::json_tree_action_style)
                .on_press(on_copy.clone()(tree_row.copy_value))
                .into()
        };
        let line = row![
            Space::new().width(Length::Fixed(tree_row.depth as f32 * JSON_TREE_INDENT)),
            toggle,
            tokens,
            Space::new().width(Length::Fixed(8.0)),
            copy,
        ]
        .spacing(3)
        .align_y(iced::Alignment::Center);
        content = content.push(line);
    }

    scrollable(container(content).padding([4, 6]))
        .direction(json_tree_scroll_direction())
        .width(Length::Fill)
        .into()
}
