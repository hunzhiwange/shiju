//! markdown 模块的单元测试。
//!
//! 覆盖块级解析（标题/代码块/列表/引用）与行内标记清理，并确认链接点击
//! 会映射为外部打开消息。

use super::markdown::{
    MarkdownBlock, MarkdownBlockKind, markdown_block_font, markdown_block_key, markdown_body_view,
    markdown_editor_text, markdown_visual_contract, parse_markdown_blocks, selection_actions,
    strip_inline_markdown,
};
use super::markdown_rich::link_click_message;
use crate::app::components::chat_panel::tool_text_support::selected_chat_text_for_message;
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
    assert_eq!(element_child_count(markdown_body_view(&app, 7, 0, &blocks)), 6);

    let contract = markdown_visual_contract();
    assert_eq!(contract.heading_sizes, [21.0, 19.0, 17.0, 16.0, 16.0, 16.0]);
    assert_eq!(contract.line_height, 1.6);
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
    let ascii = MarkdownBlock {
        kind: MarkdownBlockKind::CodeBlock(None),
        text: "let x = 1;".to_string(),
        inline: Vec::new(),
    };
    let mixed = MarkdownBlock {
        kind: MarkdownBlockKind::CodeBlock(None),
        text: "// 中文注释\nlet x = 1;".to_string(),
        inline: Vec::new(),
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
fn formatted_markdown_uses_one_editor_for_cross_line_selection() {
    let mut app = App::new().0;
    let blocks = parse_markdown_blocks("first\n\nsecond\n\nthird");
    app.chat_markdown_block_editors.insert(
        markdown_block_key(0, 0, 0),
        text_editor::Content::with_text(&markdown_editor_text(&blocks)),
    );
    app.chat_markdown_block_editors
        .get_mut(&markdown_block_key(0, 0, 0))
        .expect("markdown editor")
        .perform(text_editor::Action::SelectAll);

    assert_eq!(
        selected_chat_text_for_message(&app, 0).as_deref(),
        Some("first\n\nsecond\n\nthird")
    );
}

#[test]
fn markdown_editor_text_separates_blocks_and_keeps_list_items_compact() {
    let blocks = parse_markdown_blocks("正文\n\n- 第一项\n- 第二项\n\n结尾");

    assert_eq!(markdown_editor_text(&blocks), "正文\n\n• 第一项\n• 第二项\n\n结尾");
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
    assert_eq!(blocks[0].kind, MarkdownBlockKind::Heading(2));
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

    assert_eq!(blocks.len(), 7);
    assert_eq!(blocks[0].kind, MarkdownBlockKind::Paragraph);
    assert_eq!(blocks[0].text, "intro");
    assert_eq!(blocks[1].kind, MarkdownBlockKind::Rule);
    assert_eq!(blocks[2].kind, MarkdownBlockKind::Quote);
    assert_eq!(blocks[2].text, "first\nsecond");
    assert_eq!(blocks[3].kind, MarkdownBlockKind::Rule);
    assert_eq!(blocks[4].kind, MarkdownBlockKind::BulletItem(0));
    assert_eq!(blocks[5].kind, MarkdownBlockKind::BulletItem(0));
    assert_eq!(blocks[6].kind, MarkdownBlockKind::BulletItem(0));
}

#[test]
fn markdown_body_view_builds_every_block_kind_with_editor_and_text_fallbacks() {
    let (mut app, _) = App::new();
    app.chat_markdown_block_editors
        .insert(markdown_block_key(2, 3, 0), text_editor::Content::with_text("heading"));
    let blocks = vec![
        MarkdownBlock {
            kind: MarkdownBlockKind::Heading(2),
            text: "heading".to_string(),
            inline: Vec::new(),
        },
        MarkdownBlock {
            kind: MarkdownBlockKind::Paragraph,
            text: "paragraph".to_string(),
            inline: Vec::new(),
        },
        MarkdownBlock {
            kind: MarkdownBlockKind::BulletItem(2),
            text: "bullet".to_string(),
            inline: Vec::new(),
        },
        MarkdownBlock {
            kind: MarkdownBlockKind::OrderedItem("3.".to_string()),
            text: "ordered".to_string(),
            inline: Vec::new(),
        },
        MarkdownBlock {
            kind: MarkdownBlockKind::CodeBlock(Some("rust".to_string())),
            text: "let x = 1;".to_string(),
            inline: Vec::new(),
        },
        MarkdownBlock {
            kind: MarkdownBlockKind::Quote,
            text: "quote".to_string(),
            inline: Vec::new(),
        },
    ];

    assert_eq!(element_child_count(markdown_body_view(&app, 2, 3, &blocks)), 6);
    assert_eq!(element_child_count(markdown_body_view(&app, 2, 3, &[])), 0);
}

#[test]
fn long_markdown_keeps_bounded_editor_work() {
    let mut markdown = "长回复正文。".repeat(2_000);
    for idx in 0..200 {
        markdown.push_str(&format!("\n\n- 列表项 {idx}\n\n```rust\nlet item_{idx} = {idx};\n```"));
    }

    let blocks = parse_markdown_blocks(&markdown);
    let source_line_count = markdown.lines().count();

    assert_eq!(blocks.len(), 401);
    assert!(blocks.len() < source_line_count);
    assert!(blocks.iter().map(|block| block.text.len()).sum::<usize>() <= markdown.len());
}

#[test]
fn table_parses_cells_and_keeps_code_fences_verbatim() {
    let blocks = parse_markdown_blocks(concat!(
        "开头\n\n",
        "| 名称 | 地址 |\n| --- | ---: |\n",
        "| A \\| B | [文档](https://example.com/docs) |\n\n",
        "```text\n| 名称 | 地址 |\n| --- | --- |\n```\n\n",
        "结尾",
    ));

    assert_eq!(blocks.len(), 4);
    let MarkdownBlockKind::Table(table) = &blocks[1].kind else {
        panic!("expected a GFM table");
    };
    assert_eq!(table.rows[0][0][0].text, "名称");
    assert_eq!(table.rows[1][0][0].text, "A | B");
    assert_eq!(table.rows[1][1][0].url.as_deref(), Some("https://example.com/docs"));
    assert!(matches!(blocks[2].kind, MarkdownBlockKind::CodeBlock(_)));
    assert_eq!(blocks[3].text, "结尾");
}

#[test]
fn links_keep_web_destinations_and_ignore_code_and_local_schemes() {
    let blocks = parse_markdown_blocks(concat!(
        "请看[**文档**](https://example.com/docs_(zh))、",
        "<https://example.com/autolink> 和网址https://example.com/bare。\n",
        "`https://example.com/code` [本地](file:///tmp/test) ",
        "[https://example.com/label](javascript:alert(1))",
    ));
    let links: Vec<_> = blocks[0]
        .inline
        .iter()
        .filter_map(|span| span.url.as_deref().map(|url| (span.text.as_str(), url)))
        .collect();

    assert_eq!(
        links,
        [
            ("文档", "https://example.com/docs_(zh)"),
            ("https://example.com/autolink", "https://example.com/autolink"),
            ("https://example.com/bare", "https://example.com/bare"),
        ]
    );
}

#[test]
fn bare_urls_keep_balanced_paths_and_complete_queries() {
    let blocks = parse_markdown_blocks(concat!(
        "访问https://example.com/Foo_(bar)?a=1&amp;b=2。
",
        "（WWW.example.com/path），再看 (https://example.com/other).",
    ));
    let links: Vec<_> = blocks[0].inline.iter().filter_map(|span| span.url.as_deref()).collect();
    assert_eq!(
        links,
        [
            "https://example.com/Foo_(bar)?a=1&b=2",
            "https://www.example.com/path",
            "https://example.com/other",
        ]
    );
}

#[test]
fn table_link_renders_and_emits_open_message_in_both_themes() {
    use iced::advanced::clipboard;
    use iced::advanced::renderer::{Headless, Style};
    use iced::{Event, Font, Point, Renderer, Size, Theme, mouse};
    use iced_runtime::user_interface::{Cache, UserInterface};

    let blocks = parse_markdown_blocks(concat!(
        "| 名称 | 说明 | 第三列 |\n| --- | --- | --- |\n",
        "| [Docs](https://example.com/docs) | https://example.com/bare | 中文内容 |",
    ));
    let MarkdownBlockKind::Table(table) = &blocks[0].kind else {
        panic!("expected a GFM table");
    };

    {
        let mut fonts = iced::advanced::graphics::text::font_system().write().unwrap();
        for font in crate::fonts::load_all() {
            fonts.load_font(font);
        }
    }
    let mut renderer = iced::futures::executor::block_on(Renderer::new(
        Font::DEFAULT,
        15.0.into(),
        Some("tiny-skia"),
    ))
    .expect("headless renderer");
    let size = Size::new(340.0, 120.0);
    for theme in [Theme::Light, Theme::Dark] {
        let mut ui = UserInterface::build(
            super::markdown_rich::table_view(table, &theme, theme.palette().text),
            size,
            Cache::new(),
            &mut renderer,
        );
        ui.draw(
            &mut renderer,
            &theme,
            &Style { text_color: theme.palette().text },
            mouse::Cursor::Unavailable,
        );
        let pixels = renderer.screenshot(Size::new(340, 120), 1.0, theme.palette().background);
        assert!(pixels.chunks_exact(4).any(|pixel| pixel != &pixels[..4]));

        for (x, target) in [(20.0, "https://example.com/docs"), (210.0, "https://example.com/bare")]
        {
            let cursor = mouse::Cursor::Available(Point::new(x, 47.0));
            let mut messages = Vec::new();
            for event in [
                mouse::Event::ButtonPressed(mouse::Button::Left),
                mouse::Event::ButtonReleased(mouse::Button::Left),
            ] {
                ui.update(
                    &[Event::Mouse(event)],
                    cursor,
                    &mut renderer,
                    &mut clipboard::Null,
                    &mut messages,
                );
            }
            assert!(
                matches!(
                    messages.as_slice(),
                    [Message::View(ViewMessage::OpenUrlExternal(url))] if url == target
                ),
                "clicking {target} emitted {messages:?}"
            );
        }
    }
}

#[test]
fn markdown_groups_standalone_package_commands_and_keeps_adjacent_prose() {
    let blocks = parse_markdown_blocks(concat!(
        "运行\n",
        "　pnpm install\npnpm demo:1\n\n",
        "默认访问 http://localhost:5173。也可手动执行类型检查：\n\n",
        "pnpm exec tsc -p demo/1 --noEmit\n\n",
        "pnpm is a package manager.",
    ));

    assert_eq!(blocks.len(), 5);
    assert_eq!(blocks[0].kind, MarkdownBlockKind::Paragraph);
    assert_eq!(blocks[1].kind, MarkdownBlockKind::CodeBlock(Some("shell".to_owned())));
    assert_eq!(blocks[1].text, "pnpm install\npnpm demo:1");
    assert!(!blocks[2].inline.is_empty());
    assert_eq!(blocks[3].kind, MarkdownBlockKind::CodeBlock(Some("shell".to_owned())));
    assert_eq!(blocks[3].text, "pnpm exec tsc -p demo/1 --noEmit");
    assert_eq!(blocks[4].kind, MarkdownBlockKind::Paragraph);
}

#[test]
fn markdown_code_payload_preserves_shorter_fences_and_internal_whitespace() {
    use crate::app::components::chat_panel::utils::normalize_display_text;

    for fence in ["````", "~~~~"] {
        let payload = "  # 中文注释  \n```\n\n\npnpm install";
        let raw = format!("{fence}shell\n{payload}\n{fence}\n\n正文");
        let normalized = normalize_display_text(&raw);
        let blocks = parse_markdown_blocks(normalized.as_ref());
        assert_eq!(blocks.len(), 2);
        assert_eq!(blocks[0].kind, MarkdownBlockKind::CodeBlock(Some("shell".to_owned())));
        assert_eq!(blocks[0].text, payload);
    }
}

#[test]
fn markdown_code_copy_renders_feedback_and_restores_in_both_themes() {
    use iced::advanced::clipboard;
    use iced::advanced::renderer::{Headless, Style};
    use iced::{Event, Font, Point, Renderer, Size, Theme, mouse};
    use iced_runtime::user_interface::{Cache, UserInterface};

    let code = "# 中文注释\npnpm install\n  pnpm demo:1";
    let blocks = parse_markdown_blocks(&format!("```shell\n{code}\n```"));
    let (mut app, _) = App::new();
    app.chat_markdown_block_editors.insert(0, text_editor::Content::with_text(code));
    {
        let mut fonts = iced::advanced::graphics::text::font_system().write().unwrap();
        for font in crate::fonts::load_all() {
            fonts.load_font(font);
        }
    }
    let mut renderer = iced::futures::executor::block_on(Renderer::new(
        Font::DEFAULT,
        15.0.into(),
        Some("tiny-skia"),
    ))
    .expect("headless renderer");

    let render_and_click = |app: &App, renderer: &mut Renderer| {
        let theme = app.theme();
        let size = Size::new(340.0, 160.0);
        let mut ui = UserInterface::build(
            markdown_body_view(app, 0, 0, &blocks),
            size,
            Cache::new(),
            renderer,
        );
        ui.draw(
            renderer,
            &theme,
            &Style { text_color: theme.palette().text },
            mouse::Cursor::Unavailable,
        );
        let pixels = renderer.screenshot(Size::new(340, 160), 1.0, theme.palette().background);

        let mut messages = Vec::new();
        let cursor = mouse::Cursor::Available(Point::new(314.0, 20.0));
        for event in [
            mouse::Event::ButtonPressed(mouse::Button::Left),
            mouse::Event::ButtonReleased(mouse::Button::Left),
        ] {
            ui.update(
                &[Event::Mouse(event)],
                cursor,
                renderer,
                &mut clipboard::Null,
                &mut messages,
            );
        }
        assert!(matches!(messages.as_slice(), [Message::CopyCode(text)] if text == code));
        (pixels, messages.pop().expect("copy action"))
    };

    for theme in [Theme::Light, Theme::Dark] {
        app.app_theme = theme;
        let (before, copy) = render_and_click(&app, &mut renderer);
        let _ = app.update(copy);
        let (copied, _) = render_and_click(&app, &mut renderer);
        assert_ne!(copied, before);

        let hash = app.last_copied_code_hash.expect("copied content hash");
        let _ = app.update(Message::CopyFeedbackExpired(hash));
        let (restored, repeat_copy) = render_and_click(&app, &mut renderer);
        assert_eq!(restored, before);
        let _ = app.update(repeat_copy);
        assert_eq!(app.last_copied_code_hash, Some(hash));
        let _ = app.update(Message::CopyFeedbackExpired(hash));
    }
}

#[test]
fn markdown_styles_inline_text_in_paragraphs_and_tables() {
    let blocks = parse_markdown_blocks(concat!(
        "**粗体和 *斜体***，~~删除~~，`中文代码`，参见[文档][guide]。\n\n",
        "| 选项 | 值 |\n| --- | --- |\n| *启用* | `true` |\n\n",
        "[guide]: https://example.com/guide",
    ));
    assert_eq!(blocks.len(), 2);
    let spans = &blocks[0].inline;
    assert!(spans.iter().any(|part| part.text == "粗体和 " && part.style.bold));
    assert!(spans.iter().any(|part| part.text == "斜体" && part.style.bold && part.style.italic));
    assert!(spans.iter().any(|part| part.text == "删除" && part.style.strike));
    assert!(spans.iter().any(|part| part.text == "中文代码" && part.style.code));
    assert!(spans.iter().any(|part| {
        part.text == "文档" && part.url.as_deref() == Some("https://example.com/guide")
    }));

    let MarkdownBlockKind::Table(table) = &blocks[1].kind else {
        panic!("expected table");
    };
    assert!(table.rows[1][0][0].style.italic);
    assert!(table.rows[1][1][0].style.code);
}

#[test]
fn markdown_parses_heading_divider_and_read_only_task_states() {
    let blocks = parse_markdown_blocks(concat!(
        "总览\n===\n\n",
        "## 步骤 ##\n\n",
        "---\n\n",
        "- [ ] 待办\n  - [x] 已完成\n1. [X] 已验证",
    ));
    assert_eq!(blocks.len(), 6);
    assert_eq!(blocks[0].kind, MarkdownBlockKind::Heading(1));
    assert_eq!(blocks[1].text, "步骤");
    assert_eq!(blocks[1].kind, MarkdownBlockKind::Heading(2));
    assert_eq!(blocks[2].kind, MarkdownBlockKind::Rule);
    assert_eq!(
        blocks[3].kind,
        MarkdownBlockKind::TaskItem { indent: 0, number: None, checked: false }
    );
    assert_eq!(
        blocks[4].kind,
        MarkdownBlockKind::TaskItem { indent: 1, number: None, checked: true }
    );
    assert_eq!(
        blocks[5].kind,
        MarkdownBlockKind::TaskItem { indent: 0, number: Some("1.".into()), checked: true }
    );
}

#[test]
fn markdown_table_fills_available_width_and_overflows_only_when_needed() {
    use iced::advanced::layout;
    use iced::advanced::renderer::Headless;
    use iced::{Font, Renderer, Size, Theme};

    let blocks = parse_markdown_blocks("| **名称** | `值` |\n| --- | --- |\n| 示例 | 1 |");
    let MarkdownBlockKind::Table(table) = &blocks[0].kind else {
        panic!("expected table");
    };
    let renderer = iced::futures::executor::block_on(Renderer::new(
        Font::DEFAULT,
        15.0.into(),
        Some("tiny-skia"),
    ))
    .expect("headless renderer");

    for width in [240.0, 480.0, 800.0] {
        let theme = Theme::Dark;
        let mut element = super::markdown_rich::table_view(table, &theme, theme.palette().text);
        let mut tree = Tree::new(element.as_widget());
        let limits = layout::Limits::new(Size::ZERO, Size::new(width, 300.0));
        let node = element.as_widget_mut().layout(&mut tree, &renderer, &limits);
        assert_eq!(node.size().width, width);
        let grid = &node.children()[0].children()[0];
        if width == 240.0 {
            assert!(grid.size().width > width);
        } else {
            assert!((grid.size().width - width).abs() < 0.01);
        }
    }
}
