//! markdown 模块的单元测试。
//!
//! 覆盖块级解析（标题/代码块/列表/引用）与行内标记清理，并确认链接点击
//! 会映射为外部打开消息。

use super::markdown::{
    MarkdownBlock, MarkdownBlockKind, link_click_message, markdown_block_font, markdown_block_key,
    markdown_body_view, markdown_visual_contract, parse_markdown_blocks, selection_actions,
    strip_inline_markdown,
};
use crate::app::message::ViewMessage;
use crate::app::{App, Message};
use iced::Element;
use iced::advanced::widget::Tree;
use iced::widget::text_editor;

fn element_child_count(element: Element<'_, Message>) -> usize {
    let tree = Tree::new(element.as_widget());
    tree.children.first().map_or(0, |content| content.children.len())
}

#[test]
fn markdown_visual_contract_preserves_readable_block_hierarchy() {
    let markdown = concat!(
        "# 中文标题\n\n",
        "A long English paragraph with **emphasis**.\n\n",
        "- first\n  - nested\n\n",
        "> 引用内容\n\n",
        "```rust\n// 中文注释\nlet value = 1;\n```",
    );
    let blocks = parse_markdown_blocks(markdown);
    let (app, _) = App::new();
    assert_eq!(element_child_count(markdown_body_view(&app, 7, 0, &blocks)), blocks.len());

    let contract = markdown_visual_contract();
    assert_eq!(contract.heading_sizes, [21.0, 19.0, 17.0, 16.0, 16.0, 16.0]);
    assert_eq!(contract.block_spacing, 8.0);
    assert_eq!(contract.list_base_indent, 18.0);
    assert_eq!(contract.list_nested_indent, 14.0);
    assert_eq!(contract.quote_bar_width, 2.0);
    assert_eq!(contract.quote_content_inset, 12.0);
    assert_eq!(contract.code_padding, iced::Padding::from([10, 12]));
    assert_eq!(contract.code_corner_radius, 8.0);
    assert_eq!(contract.code_border_width, 1.0);
}

#[test]
fn parse_blocks_splits_heading_paragraph_list() {
    let blocks = parse_markdown_blocks("# 标题\n\n正文段落\n\n- 列表项");
    assert_eq!(blocks.len(), 3);
    assert_eq!(blocks[0].kind, MarkdownBlockKind::Heading(1));
    assert_eq!(blocks[0].text, "标题");
    assert_eq!(blocks[1].kind, MarkdownBlockKind::Paragraph);
    assert_eq!(blocks[2].kind, MarkdownBlockKind::BulletItem(0));
    assert_eq!(blocks[2].text, "列表项");
}

#[test]
fn parse_blocks_keeps_code_block_verbatim() {
    let blocks = parse_markdown_blocks("说明\n\n```rust\nfn main() {}\n```");
    assert_eq!(blocks.len(), 2);
    assert_eq!(blocks[1].kind, MarkdownBlockKind::CodeBlock(Some("rust".to_string())));
    assert_eq!(blocks[1].text, "fn main() {}");
}

#[test]
fn code_block_font_uses_monospace_only_for_ascii_content() {
    let ascii =
        MarkdownBlock { kind: MarkdownBlockKind::CodeBlock(None), text: "let x = 1;".to_string() };
    let mixed = MarkdownBlock {
        kind: MarkdownBlockKind::CodeBlock(None),
        text: "// 中文注释\nlet x = 1;".to_string(),
    };

    assert_eq!(markdown_block_font(&ascii), iced::Font::MONOSPACE);
    assert_eq!(markdown_block_font(&mixed), super::super::tool_text_support::chat_text_font());
}

#[test]
fn parse_blocks_ordered_and_quote() {
    let blocks = parse_markdown_blocks("1. 第一\n2. 第二\n\n> 引用行");
    assert_eq!(blocks[0].kind, MarkdownBlockKind::OrderedItem("1.".to_string()));
    assert_eq!(blocks[1].kind, MarkdownBlockKind::OrderedItem("2.".to_string()));
    assert_eq!(blocks[2].kind, MarkdownBlockKind::Quote);
    assert_eq!(blocks[2].text, "引用行");
}

#[test]
fn parse_blocks_empty_yields_nothing() {
    assert!(parse_markdown_blocks("").is_empty());
    assert!(parse_markdown_blocks("\n\n").is_empty());
}

#[test]
fn strip_inline_removes_emphasis_and_code() {
    assert_eq!(strip_inline_markdown("这是 **粗体** 和 `代码`"), "这是 粗体 和 代码");
    assert_eq!(strip_inline_markdown("__也是粗__ 与 _斜_"), "也是粗 与 斜");
    assert_eq!(strip_inline_markdown("~~删除~~ 文本"), "删除 文本");
}

#[test]
fn strip_inline_keeps_link_text() {
    assert_eq!(strip_inline_markdown("见 [文档](https://example.com)"), "见 文档");
    assert_eq!(strip_inline_markdown("裸链 <https://example.com>"), "裸链 https://example.com");
}

#[test]
fn strip_inline_keeps_lonely_markers() {
    // 乘法等非强调用法（标记后紧跟空白）应原样保留。
    assert_eq!(strip_inline_markdown("2 * 3 = 6"), "2 * 3 = 6");
}

#[test]
fn block_key_is_unique_per_index_triplet() {
    assert_ne!(markdown_block_key(1, 0, 0), markdown_block_key(1, 0, 1));
    assert_ne!(markdown_block_key(1, 0, 0), markdown_block_key(1, 1, 0));
    assert_ne!(markdown_block_key(1, 0, 0), markdown_block_key(2, 0, 0));
}

#[test]
fn cross_block_drag_selects_boundaries_and_intermediate_blocks() {
    let actions = selection_actions(
        0,
        2,
        iced::Point::new(40.0, 12.0),
        iced::Rectangle::new(iced::Point::new(10.0, 10.0), iced::Size::new(100.0, 30.0)),
        4,
    );

    assert!(actions.contains(&(1, text_editor::Action::SelectAll)));
    assert!(actions.contains(&(3, text_editor::Action::Click(iced::Point::ORIGIN))));
    assert!(
        actions.contains(&(0, text_editor::Action::Drag(iced::Point::new(f32::MAX, f32::MAX))))
    );
    assert!(actions.contains(&(2, text_editor::Action::Click(iced::Point::ORIGIN))));
    assert!(actions.contains(&(2, text_editor::Action::Drag(iced::Point::new(30.0, 2.0)))));
}

#[test]
fn reverse_cross_block_drag_selects_from_current_end() {
    let actions = selection_actions(
        2,
        0,
        iced::Point::new(25.0, 18.0),
        iced::Rectangle::new(iced::Point::new(5.0, 8.0), iced::Size::new(100.0, 30.0)),
        3,
    );

    assert!(actions.contains(&(1, text_editor::Action::SelectAll)));
    assert!(actions.contains(&(2, text_editor::Action::Drag(iced::Point::ORIGIN))));
    assert!(
        actions.contains(&(0, text_editor::Action::Click(iced::Point::new(f32::MAX, f32::MAX))))
    );
    assert!(actions.contains(&(0, text_editor::Action::Drag(iced::Point::new(20.0, 10.0)))));
}

#[test]
fn link_click_maps_to_external_open() {
    let message = link_click_message("https://example.com".to_string());
    assert!(matches!(
        message,
        Message::View(ViewMessage::OpenUrlExternal(url)) if url == "https://example.com"
    ));
}

#[test]
fn strip_inline_preserves_unclosed_and_invalid_constructs() {
    assert_eq!(strip_inline_markdown("before `unterminated"), "before `unterminated");
    assert_eq!(strip_inline_markdown("[label] without target"), "[label] without target");
    assert_eq!(strip_inline_markdown("<mailto:user@example.com>"), "<mailto:user@example.com>");
    assert_eq!(strip_inline_markdown("****"), "****");
}

#[test]
fn strip_inline_handles_nested_link_labels_multitick_code_and_plain_angles() {
    assert_eq!(strip_inline_markdown("见 [**文档**](https://example.com)"), "见 文档");
    assert_eq!(strip_inline_markdown("``code ` inside``"), "code ` inside");
    assert_eq!(
        strip_inline_markdown("<not a url> and <http://example.test>"),
        "<not a url> and http://example.test"
    );
}

#[test]
fn parse_blocks_handles_fences_breaks_and_syntax_boundaries() {
    let blocks = parse_markdown_blocks(
        "paragraph\n---\n###### Small\n####### not-heading\n#not-heading\n\n~~~\nraw **code**\n",
    );

    assert_eq!(blocks.len(), 4);
    assert_eq!(blocks[0].kind, MarkdownBlockKind::Paragraph);
    assert_eq!(blocks[0].text, "paragraph");
    assert_eq!(blocks[1].kind, MarkdownBlockKind::Heading(6));
    assert_eq!(blocks[1].text, "Small");
    assert_eq!(blocks[2].kind, MarkdownBlockKind::Paragraph);
    assert_eq!(blocks[2].text, "####### not-heading\n#not-heading");
    assert_eq!(blocks[3].kind, MarkdownBlockKind::CodeBlock(None));
    assert_eq!(blocks[3].text, "raw **code**");
}

#[test]
fn parse_blocks_clamps_bullet_indent_and_accepts_parenthesized_ordering() {
    let blocks = parse_markdown_blocks("            + nested\n12) item\n1234. paragraph");

    assert_eq!(blocks[0].kind, MarkdownBlockKind::BulletItem(4));
    assert_eq!(blocks[0].text, "nested");
    assert_eq!(blocks[1].kind, MarkdownBlockKind::OrderedItem("12)".to_string()));
    assert_eq!(blocks[1].text, "item");
    assert_eq!(blocks[2].kind, MarkdownBlockKind::Paragraph);
    assert_eq!(blocks[2].text, "1234. paragraph");
}

#[test]
fn parse_blocks_handles_multiline_quotes_alternate_breaks_and_marker_boundaries() {
    let blocks = parse_markdown_blocks(
        "intro\n* * *\n> first\n>second\n\n_ _ _\n- not list\n* also not\n+ neither\n",
    );

    assert_eq!(blocks.len(), 5);
    assert_eq!(blocks[0].kind, MarkdownBlockKind::Paragraph);
    assert_eq!(blocks[0].text, "intro");
    assert_eq!(blocks[1].kind, MarkdownBlockKind::Quote);
    assert_eq!(blocks[1].text, "first\nsecond");
    assert_eq!(blocks[2].kind, MarkdownBlockKind::BulletItem(0));
    assert_eq!(blocks[2].text, "not list");
    assert_eq!(blocks[3].kind, MarkdownBlockKind::BulletItem(0));
    assert_eq!(blocks[3].text, "also not");
    assert_eq!(blocks[4].kind, MarkdownBlockKind::BulletItem(0));
    assert_eq!(blocks[4].text, "neither");
}

#[test]
fn markdown_body_view_builds_every_block_kind_with_editor_and_text_fallbacks() {
    let (mut app, _) = App::new();
    app.chat_markdown_block_editors
        .insert(markdown_block_key(2, 3, 0), text_editor::Content::with_text("heading"));
    let blocks = vec![
        MarkdownBlock { kind: MarkdownBlockKind::Heading(2), text: "heading".to_string() },
        MarkdownBlock { kind: MarkdownBlockKind::Paragraph, text: "paragraph".to_string() },
        MarkdownBlock { kind: MarkdownBlockKind::BulletItem(2), text: "bullet".to_string() },
        MarkdownBlock {
            kind: MarkdownBlockKind::OrderedItem("3.".to_string()),
            text: "ordered".to_string(),
        },
        MarkdownBlock {
            kind: MarkdownBlockKind::CodeBlock(Some("rust".to_string())),
            text: "let x = 1;".to_string(),
        },
        MarkdownBlock { kind: MarkdownBlockKind::Quote, text: "quote".to_string() },
    ];

    assert_eq!(element_child_count(markdown_body_view(&app, 2, 3, &blocks)), blocks.len());
    assert_eq!(element_child_count(markdown_body_view(&app, 2, 3, &[])), 0);
}
