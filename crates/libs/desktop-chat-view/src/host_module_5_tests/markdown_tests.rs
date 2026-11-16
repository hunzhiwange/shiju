//! markdown 模块的单元测试。
//!
//! 覆盖块级解析（标题/代码块/列表/引用）与行内标记清理，并确认链接点击
//! 会映射为外部打开消息。

use super::fixture::ViewMessage;
use super::fixture::{Message, RenderFixture};
use super::markdown::*;
use super::markdown_links::{LinkTarget, WorkspaceLinks, link_click_message};
use super::selected_chat_text_for_message;
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
    let (app, _) = RenderFixture::new();
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
fn formatted_markdown_uses_one_editor_for_cross_line_selection() {
    let mut app = RenderFixture::new().0;
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
fn link_click_maps_to_external_open() {
    let message = link_click_message(LinkTarget::Web("https://example.com".to_string()), None);
    assert!(matches!(
        message,
        Message::View(ViewMessage::OpenUrlExternal(url)) if url == "https://example.com"
    ));
}

#[test]
fn markdown_body_view_builds_every_block_kind_with_editor_and_text_fallbacks() {
    let (mut app, _) = RenderFixture::new();
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
        for font in desktop_assets::fonts::load_all() {
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
            super::markdown_rich::table_view(
                table,
                &theme,
                theme.palette().text,
                WorkspaceLinks::default(),
                None,
            ),
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
fn markdown_code_copy_renders_feedback_and_restores_in_both_themes() {
    use iced::advanced::clipboard;
    use iced::advanced::renderer::{Headless, Style};
    use iced::{Event, Font, Point, Renderer, Size, Theme, mouse};
    use iced_runtime::user_interface::{Cache, UserInterface};

    let code = "# 中文注释\npnpm install\n  pnpm demo:1";
    let blocks = parse_markdown_blocks(&format!("```shell\n{code}\n```"));
    let (mut app, _) = RenderFixture::new();
    app.chat_markdown_block_editors.insert(0, text_editor::Content::with_text(code));
    {
        let mut fonts = iced::advanced::graphics::text::font_system().write().unwrap();
        for font in desktop_assets::fonts::load_all() {
            fonts.load_font(font);
        }
    }
    let mut renderer = iced::futures::executor::block_on(Renderer::new(
        Font::DEFAULT,
        15.0.into(),
        Some("tiny-skia"),
    ))
    .expect("headless renderer");

    let render_and_click = |app: &RenderFixture, renderer: &mut Renderer| {
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
        let mut element = super::markdown_rich::table_view(
            table,
            &theme,
            theme.palette().text,
            WorkspaceLinks::default(),
            None,
        );
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

#[cfg(unix)]
#[test]
fn markdown_keeps_code_fragments_and_identifiers_unlinked() {
    let blocks = parse_markdown_blocks(concat!(
        "`</script>`、`<\\/script>`、`</scr` + `ipt>`、",
        "`deepseek/deepseek-flash`、`meta-llama/Llama-3.1`、`@scope/package.js`、",
        "`/tmp/test --verbose`、`/tmp/a && echo done`、`/bin/cat /tmp/test.js`，",
        "命令 /help，片段 </broken/tag",
    ));
    let spans = &blocks[0].inline;
    for code in ["</script>", "<\\/script>", "</scr", "deepseek/deepseek-flash"] {
        assert!(spans.iter().any(|span| span.text == code && span.style.code));
    }
    let mut app = RenderFixture::new().0;
    app.project_path = Some("/repo".into());
    app.set_file_index("/repo", vec!["src/main.rs".into()]);
    let rendered =
        super::markdown_rich::inline_spans(spans, &iced::Theme::Light, &WorkspaceLinks::new(&app));
    assert!(rendered.iter().all(|span| span.link.is_none()));
}

#[cfg(unix)]
#[test]
fn markdown_auto_links_indexed_paths_and_keeps_explicit_links() {
    let mut app = RenderFixture::new().0;
    app.project_path = Some("/repo".into());
    app.set_file_index(
        "/repo",
        vec!["index.php".into(), "src/main.js".into(), "src/README".into()],
    );
    let links = WorkspaceLinks::new(&app);
    let blocks = parse_markdown_blocks(concat!(
        "`index.php`，`src/main.js:12`，`src/README`，/repo/src，",
        "`application/json`，`src/missing.js`，/api/v1，",
        "[外部文件](/tmp/index.php)，http://example.com/main.js",
    ));
    let spans = super::markdown_rich::inline_spans(&blocks[0].inline, &iced::Theme::Light, &links);
    let targets = spans.into_iter().filter_map(|span| span.link).collect::<Vec<_>>();
    assert_eq!(
        targets,
        [
            LinkTarget::File("/repo/index.php".into()),
            LinkTarget::File("/repo/src/main.js".into()),
            LinkTarget::File("/repo/src/README".into()),
            LinkTarget::File("/repo/src".into()),
            LinkTarget::File("/tmp/index.php".into()),
            LinkTarget::Web("http://example.com/main.js".into()),
        ]
    );
    assert!(matches!(
        links.open(targets[5].clone()),
        Message::View(ViewMessage::OpenUrlExternal(url)) if url == "http://example.com/main.js"
    ));
}

#[test]
fn markdown_renders_padded_inline_code_in_both_themes() {
    use iced::advanced::renderer::{Headless, Style};
    use iced::{Font, Padding, Renderer, Size, Theme, mouse};
    use iced_runtime::user_interface::{Cache, UserInterface};

    let blocks = parse_markdown_blocks("在`data-gateway`仓库里查看`中文代码`。");
    let (mut app, _) = RenderFixture::new();
    {
        let mut fonts = iced::advanced::graphics::text::font_system().write().unwrap();
        for font in desktop_assets::fonts::load_all() {
            fonts.load_font(font);
        }
    }
    let mut renderer = iced::futures::executor::block_on(Renderer::new(
        Font::DEFAULT,
        15.0.into(),
        Some("tiny-skia"),
    ))
    .expect("headless renderer");

    for theme in [Theme::Light, Theme::Dark] {
        let spans = super::markdown_rich::inline_spans(
            &blocks[0].inline,
            &theme,
            &WorkspaceLinks::default(),
        );
        for code in ["data-gateway", "中文代码"] {
            let span = spans.iter().find(|span| span.text == code).expect("inline code span");
            assert_eq!(span.padding, Padding::from([0, 3]));
        }

        app.app_theme = theme.clone();
        let view = markdown_body_view(&app, 0, 0, &blocks);
        let mut ui =
            UserInterface::build(view, Size::new(340.0, 60.0), Cache::new(), &mut renderer);
        ui.draw(
            &mut renderer,
            &theme,
            &Style { text_color: theme.palette().text },
            mouse::Cursor::Unavailable,
        );
        let pixels = renderer.screenshot(Size::new(340, 60), 1.0, theme.palette().background);
        assert!(pixels.chunks_exact(4).any(|pixel| pixel != &pixels[..4]));
    }
}

#[cfg(unix)]
#[test]
fn markdown_file_link_click_resolves_the_workspace_path() {
    use iced::advanced::clipboard;
    use iced::advanced::renderer::Headless;
    use iced::{Event, Font, Point, Renderer, Size, Theme, mouse};
    use iced_runtime::user_interface::{Cache, UserInterface};

    let blocks = parse_markdown_blocks("[Open](<./a%20b%25.js#L3>)");
    let theme = Theme::Light;
    let view = super::markdown_rich::inline_view(
        &blocks[0].inline,
        15.0,
        theme.palette().text,
        &theme,
        |target| link_click_message(target, Some("/workspace/session")),
    );
    let mut renderer = iced::futures::executor::block_on(Renderer::new(
        Font::DEFAULT,
        15.0.into(),
        Some("tiny-skia"),
    ))
    .expect("headless renderer");
    let mut ui = UserInterface::build(view, Size::new(200.0, 40.0), Cache::new(), &mut renderer);
    let cursor = mouse::Cursor::Available(Point::new(12.0, 10.0));
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

    assert!(matches!(
        messages.as_slice(),
        [Message::View(ViewMessage::OpenChatPath(path))]
            if std::path::Path::new(path) == std::path::Path::new("/workspace/session/a b%.js")
    ));
}

#[test]
fn markdown_code_payload_preserves_shorter_fences_and_internal_whitespace() {
    use crate::message_view::text_normalize::normalize_display_text;

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
