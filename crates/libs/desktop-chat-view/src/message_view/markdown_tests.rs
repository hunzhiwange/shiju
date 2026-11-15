use super::markdown::*;
use super::markdown_links::LinkTarget;

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
    assert_eq!(markdown_block_font(&mixed), desktop_chat_tools::tool_text_support::chat_text_font());
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
fn markdown_editor_text_separates_blocks_and_keeps_list_items_compact() {
    let blocks = parse_markdown_blocks("正文\n\n- 第一项\n- 第二项\n\n结尾");

    assert_eq!(markdown_editor_text(&blocks), "正文\n\n• 第一项\n• 第二项\n\n结尾");
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
    assert_eq!(
        table.rows[1][1][0].target,
        Some(LinkTarget::Web("https://example.com/docs".into()))
    );
    assert!(matches!(blocks[2].kind, MarkdownBlockKind::CodeBlock(_)));
    assert_eq!(blocks[3].text, "结尾");
}

#[test]
fn links_keep_web_destinations_and_reject_commands_and_remote_file_schemes() {
    let blocks = parse_markdown_blocks(concat!(
        "请看[**文档**](https://example.com/docs_(zh))、",
        "<https://example.com/autolink> 和网址https://example.com/bare。\n",
        "`http://example.com/code` `curl https://example.com/command` ",
        "[远程](file://server/tmp/test) ",
        "[https://example.com/label](javascript:alert(1))",
    ));
    let links: Vec<_> = blocks[0]
        .inline
        .iter()
        .filter_map(|span| match &span.target {
            Some(LinkTarget::Web(url)) => Some((span.text.as_str(), url.as_str())),
            Some(LinkTarget::File(path) | LinkTarget::InferredFile(path)) => {
                panic!("unexpected local path: {path}")
            }
            None => None,
        })
        .collect();

    assert_eq!(
        links,
        [
            ("文档", "https://example.com/docs_(zh)"),
            ("https://example.com/autolink", "https://example.com/autolink"),
            ("https://example.com/bare", "https://example.com/bare"),
            ("http://example.com/code", "http://example.com/code"),
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
    let links: Vec<_> = blocks[0]
        .inline
        .iter()
        .filter_map(|span| match &span.target {
            Some(LinkTarget::Web(url)) => Some(url.as_str()),
            Some(LinkTarget::File(path) | LinkTarget::InferredFile(path)) => {
                panic!("unexpected local path: {path}")
            }
            None => None,
        })
        .collect();
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
        part.text == "文档" && matches!(&part.target, Some(LinkTarget::Web(url)) if url == "https://example.com/guide")
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
fn markdown_preserves_local_path_candidates_and_explicit_file_links() {
    let blocks = parse_markdown_blocks(concat!(
        "目录 /Users/example/project，文件 `/tmp/中文 文件.rs:12:4`。\n",
        "[带空格](/tmp/My%20File.rs#L42) 和 <file:///tmp/a%2520b.rs#L2>。\n",
        "相对文件 `src/main.rs`、[文档目录](docs)，以及 \"~/My Folder\"。\n",
        "`./src/Makefile`、`src/README:12`、`src/components/`、[文件](src/README)。\n",
        "`https://example.com/x`、https://example.com/x",
    ));
    let files: Vec<_> = blocks[0]
        .inline
        .iter()
        .filter_map(|span| match &span.target {
            Some(LinkTarget::File(path) | LinkTarget::InferredFile(path)) => Some(path.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(
        files,
        [
            "/Users/example/project",
            "/tmp/中文 文件.rs",
            "/tmp/My File.rs",
            "/tmp/a%20b.rs",
            "src/main.rs",
            "docs",
            "~/My Folder",
            "./src/Makefile",
            "src/README",
            "src/components/",
            "src/README",
        ]
    );
}

#[test]
fn markdown_keeps_slashes_between_words_as_plain_text() {
    let prose = "查看/修改仓库里的某段代码，支持读取/写入";
    let blocks = parse_markdown_blocks(&format!("- {prose}"));
    assert_eq!(blocks[0].text, prose);
    assert!(blocks[0].inline.iter().all(|span| span.target.is_none()));
}
