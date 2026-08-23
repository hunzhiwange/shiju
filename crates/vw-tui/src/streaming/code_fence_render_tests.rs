//! 验证开放代码围栏的流式快速路径与规范 Markdown 渲染保持等价。
//! Unicode、链接、空行、资源上限和重算条件共同覆盖状态复用边界。
//! 所有检查都在每次追加后比较最终富文本，避免只验证内部状态。

use super::StreamingRender;
use super::append;
use super::append_rich_and_assert_matches_full;
use super::assert_rich_stream_matches_full_render;
use super::lines_to_plain_strings;
use super::test_cwd;
use crate::render::highlight::MAX_HIGHLIGHT_LINE_BYTES;
use crate::streaming::StreamRenderMode;
use insta::assert_debug_snapshot;
use pretty_assertions::assert_eq;

#[test]
// 围栏逐步增长时应保留语法样式、宽字符、链接注解和有意空行。
fn growing_code_fences_preserve_styles_unicode_links_and_blank_lines() {
    let (_, render) = assert_rich_stream_matches_full_render(
        &[
            "Before [link](https://example.com).\n\n```rust,no_run\n",
            "fn main() {\n",
            "    /* a multiline\n",
            "       comment */\n\n",
            "    println!(\"界 e\u{301} https://example.com\");\n",
            "}\n",
            "```\n\n~~~~python\n",
            "text = \"\"\"a multiline\n",
            "string\"\"\"\n",
            "~~~~\n",
        ],
        Some(18),
    );
    assert!(render.open_code_fence.is_none());
    assert_debug_snapshot!(lines_to_plain_strings(&render.lines), @r#"
    [
        "Before link",
        "(https://example.com).",
        "",
        "fn main() {",
        "    /* a multiline",
        "       comment */",
        "",
        "    println!(\"界 e\u{301} https://example.com\");",
        "}",
        "",
        "text = \"\"\"a multiline",
        "string\"\"\"",
    ]
    "#);
}

#[test]
// 超长开放围栏不能反复丢弃已经稳定渲染的前缀。
fn long_open_fence_retains_its_rendered_prefix() {
    let cwd = test_cwd();
    let mut source = String::new();
    let mut render = StreamingRender::new();
    append_rich_and_assert_matches_full(
        &mut render,
        &mut source,
        "```rust\nfn main() {\n",
        Some(80),
        &cwd,
    );
    let first_line = render.lines[0].line.spans[1].content.as_ptr();
    for index in 0..512 {
        append(
            &mut render,
            &mut source,
            &format!("    let value_{index} = {index};\n"),
            Some(80),
            &cwd,
            StreamRenderMode::Rich,
        );
        assert!(render.open_code_fence.is_some());
    }
    assert_eq!(render.lines[0].line.spans[1].content.as_ptr(), first_line);
    append_rich_and_assert_matches_full(&mut render, &mut source, "}\n```\n", Some(80), &cwd);
}

#[test]
// 含糊闭合符和规范化输入应交回标准解析器，避免快速路径猜测语义。
fn ambiguous_and_normalized_fences_use_canonical_semantics() {
    let streams: &[&[&str]] = &[
        &["```rust\n", "x\n", "``", "`\n"],
        &["```rust\n", "x\n", "```\t\n", "after\n"],
        &["```rust\n", "x\n", "  ````  \n", "after\n"],
        &["~~~rust\n", "x\n", "~~~\u{a0}\n", "after\n"],
        &["```rust\r\n", "let value = 'é';\r\n", "```\r\n"],
        &["```rust\n", "let value = '\0';\n", "```\n"],
        &["```r&#117;st,no_run\n", "fn main() {}\n", "```\n"],
        &[" ```rust\n", "  fn main() {}\n", " ```\n"],
        &["```unknown-language\n", "first\n", "second\n", "```\n"],
        &[
            "```markdown\n| A | B |\n",
            "| --- | --- |\n",
            "| 1 | 2 |\n```\n\n```rust\nfn first() {}\n",
            "fn second() {}\n",
            "```\n",
        ],
    ];
    for chunks in streams {
        assert_rich_stream_matches_full_render(chunks, Some(40));
    }
    for indent in 0..=3 {
        let closing = format!("{}~~~~~\t\n", " ".repeat(indent));
        assert_rich_stream_matches_full_render(
            &["~~~~rust\n", "x\n", &closing, "after\n"],
            Some(40),
        );
    }
}

#[test]
// 越过高亮上限后必须移除早期颜色，使整块代码采用一致降级样式。
fn crossing_a_highlight_limit_removes_previously_highlighted_colors() {
    let cwd = test_cwd();
    let mut source = String::new();
    let mut render = StreamingRender::new();
    for chunk in [
        "```rust\nlet first = 1;\n".to_string(),
        format!("{}\n", "x".repeat(MAX_HIGHLIGHT_LINE_BYTES + 1)),
        "let second = 2;\n".to_string(),
        "```\n".to_string(),
    ] {
        append_rich_and_assert_matches_full(&mut render, &mut source, &chunk, Some(80), &cwd);
    }
}

#[test]
// 终端宽度或渲染模式变化会使围栏状态失效，重算不得沿用旧缓存。
fn recomputing_after_resize_or_mode_change_drops_fence_state() {
    let (mut source, mut render) =
        assert_rich_stream_matches_full_render(&["```rust\nfn first() {}\n"], Some(80));
    assert!(render.open_code_fence.is_some());
    let cwd = test_cwd();
    render.recompute(&source, Some(20), &cwd, StreamRenderMode::Rich);
    assert!(render.open_code_fence.is_none());
    append_rich_and_assert_matches_full(
        &mut render,
        &mut source,
        "fn second() {}\n",
        Some(20),
        &cwd,
    );
    assert!(render.open_code_fence.is_some());
    render.clear();
    assert!(render.open_code_fence.is_none());
}
