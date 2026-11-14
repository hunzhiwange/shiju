use super::*;
use pretty_assertions::assert_eq;
use ratatui::text::Text;

fn lines_to_strings(text: &Text<'_>) -> Vec<String> {
    text.lines
        .iter()
        .map(|l| l.spans.iter().map(|s| s.content.clone()).collect::<String>())
        .collect()
}

#[test]
// 提供宽度时普通正文应按显示宽度折行。
fn wraps_plain_text_when_width_provided() {
    let markdown = "This is a simple sentence that should wrap.";
    let rendered = render_markdown_text_with_width(markdown, Some(16));
    let lines = lines_to_strings(&rendered);
    assert_eq!(
        lines,
        vec![
            "This is a simple".to_string(),
            "sentence that".to_string(),
            "should wrap.".to_string(),
        ]
    );
}

#[test]
// 列表项折行后续行应保留列表正文缩进。
fn wraps_list_items_preserving_indent() {
    let markdown = "- first second third fourth";
    let rendered = render_markdown_text_with_width(markdown, Some(14));
    let lines = lines_to_strings(&rendered);
    assert_eq!(lines, vec!["- first second".to_string(), "  third fourth".to_string(),]);
}

#[test]
// 嵌套列表折行不能丢失各层前缀与对齐关系。
fn wraps_nested_lists() {
    let markdown =
        "- outer item with several words to wrap\n  - inner item that also needs wrapping";
    let rendered = render_markdown_text_with_width(markdown, Some(20));
    let lines = lines_to_strings(&rendered);
    assert_eq!(
        lines,
        vec![
            "- outer item with".to_string(),
            "  several words to".to_string(),
            "  wrap".to_string(),
            "    - inner item".to_string(),
            "      that also".to_string(),
            "      needs wrapping".to_string(),
        ]
    );
}

#[test]
// 有序列表折行时编号和正文缩进应保持稳定。
fn wraps_ordered_lists() {
    let markdown = "1. ordered item contains many words for wrapping";
    let rendered = render_markdown_text_with_width(markdown, Some(18));
    let lines = lines_to_strings(&rendered);
    assert_eq!(
        lines,
        vec![
            "1. ordered item".to_string(),
            "   contains many".to_string(),
            "   words for".to_string(),
            "   wrapping".to_string(),
        ]
    );
}

#[test]
// 块引用折行后每条续行都应保留引用前缀。
fn wraps_blockquotes() {
    let markdown = "> block quote with content that should wrap nicely";
    let rendered = render_markdown_text_with_width(markdown, Some(22));
    let lines = lines_to_strings(&rendered);
    assert_eq!(
        lines,
        vec![
            "> block quote with".to_string(),
            "> content that should".to_string(),
            "> wrap nicely".to_string(),
        ]
    );
}

#[test]
// 列表内块引用折行应同时保留列表和引用两层上下文。
fn wraps_blockquotes_inside_lists() {
    let markdown = "- list item\n  > block quote inside list that wraps";
    let rendered = render_markdown_text_with_width(markdown, Some(24));
    let lines = lines_to_strings(&rendered);
    assert_eq!(
        lines,
        vec![
            "- list item".to_string(),
            "  > block quote inside".to_string(),
            "  > list that wraps".to_string(),
        ]
    );
}

#[test]
// 包含块引用的列表项在折行时不得拆散层级标记。
fn wraps_list_items_containing_blockquotes() {
    let markdown = "1. item with quote\n   > quoted text that should wrap";
    let rendered = render_markdown_text_with_width(markdown, Some(24));
    let lines = lines_to_strings(&rendered);
    assert_eq!(
        lines,
        vec![
            "1. item with quote".to_string(),
            "   > quoted text that".to_string(),
            "   > should wrap".to_string(),
        ]
    );
}

#[test]
// 代码块不折行，以保持复制粘贴所需的原始空白。
fn does_not_wrap_code_blocks() {
    let markdown = "````\nfn main() { println!(\"hi from a long line\"); }\n````";
    let rendered = render_markdown_text_with_width(markdown, Some(10));
    let lines = lines_to_strings(&rendered);
    assert_eq!(lines, vec!["fn main() { println!(\"hi from a long line\"); }".to_string(),]);
}

#[test]
// 无协议长 URL 形令牌不应被强制拆断。
fn does_not_split_long_url_like_token_without_scheme() {
    let url_like =
        "example.test/api/v1/projects/alpha-team/releases/2026-02-17/builds/1234567890";
    let rendered = render_markdown_text_with_width(url_like, Some(24));
    let lines = lines_to_strings(&rendered);

    assert_eq!(
        lines.iter().filter(|line| line.contains(url_like)).count(),
        1,
        "expected full URL-like token in one rendered line, got: {lines:?}"
    );
}

#[test]
// 围栏信息串带元数据时仍能提取语言并执行高亮。
fn fenced_code_info_string_with_metadata_highlights() {
    // CommonMark 信息串可能在语言令牌后带元数据；必须提取首词或逗号前令牌才能正确高亮。
    // CommonMark info strings like "rust,no_run" or "rust title=demo"
    // contain metadata after the language token.  The language must be
    // extracted (first word / comma-separated token) so highlighting works.
    for info in &["rust,no_run", "rust no_run", "rust title=\"demo\""] {
        let markdown = format!("```{info}\nfn main() {{}}\n```\n");
        let rendered = render_markdown_text(&markdown);
        let has_rgb = rendered.lines.iter().any(|line| {
            line.spans
                .iter()
                .any(|s| matches!(s.style.fg, Some(ratatui::style::Color::Rgb(..))))
        });
        assert!(has_rgb, "info string \"{info}\" should still produce syntax highlighting");
    }
}

#[test]
// CRLF 代码事件分裂后不能在真实代码行之间插入额外空行。
fn crlf_code_block_no_extra_blank_lines() {
    // pulldown-cmark 可能把 CRLF 代码块拆成多个 Text 事件；缓冲区必须逐字拼接，不插入分隔符。
    // pulldown-cmark can split CRLF code blocks into multiple Text events.
    // The buffer must concatenate them verbatim — no inserted separators.
    let markdown = "```rust\r\nfn main() {}\r\n    line2\r\n```\r\n";
    let rendered = render_markdown_text(markdown);
    let lines = lines_to_strings(&rendered);
    // 结果应精确包含两条代码行，中间不能出现伪空行。
    // Should be exactly two code lines; no spurious blank line between them.
    assert_eq!(
        lines,
        vec!["fn main() {}".to_string(), "    line2".to_string()],
        "CRLF code block should not produce extra blank lines: {lines:?}"
    );
}

#[test]
// 表格单元格的硬换行在宽度折行后仍保持独立逻辑行。
fn wrap_cell_preserves_hard_break_lines() {
    let mut cell = TableCell::default();
    cell.push_span("first line".into());
    cell.hard_break();
    cell.push_span("second line".into());

    let writer = W::new(
        "",
        std::iter::empty(),
        /*wrap_width*/ Some(80),
        /*cwd*/ None,
        &never_hide_link_destination,
    );
    let wrapped = writer.wrap_cell(&cell, /*width*/ 40);
    let rendered = wrapped
        .iter()
        .map(|line| line.line.spans.iter().map(|span| span.content.clone()).collect::<String>())
        .collect::<Vec<_>>();

    assert_eq!(rendered, vec!["first line".to_string(), "second line".to_string()]);
}

// 以下类型别名用于调用 `Writer` 的私有关联函数。
// ---------------------------------------------------------------
// Type alias for calling private associated functions on Writer.
// ---------------------------------------------------------------
type W<'a> = Writer<'a, 'a, std::iter::Empty<(Event<'a>, Range<usize>)>>;

/// 根据纯文本构建单行 `TableCell`。
/// Build a single-line `TableCell` from plain text.
fn make_cell(text: &str) -> TableCell {
    let mut cell = TableCell::default();
    cell.push_span(Span::raw(text.to_string()));
    cell
}

fn make_body_row(cells: Vec<TableCell>, has_table_pipe_syntax: bool) -> TableBodyRow {
    TableBodyRow { cells, has_table_pipe_syntax }
}

// 以下单元测试覆盖列分类、首选下限和优先级缩减。
// ===== Column-metrics unit tests =====

#[test]
// 平均词数达到阈值的列应分类为叙述正文。
fn column_classification_narrative_by_word_count() {
    // 第 0 列为短令牌，应分类为紧凑；第 1 列为多词正文，应分类为叙述。
    // Col 0: short tokens (1-2 words each) -> Compact
    // Col 1: prose (≥4 words per cell) → Narrative
    let header = vec![make_cell("ID"), make_cell("Description")];
    let rows = vec![
        vec![make_cell("1"), make_cell("a long description of the item")],
        vec![make_cell("2"), make_cell("another verbose body cell here")],
    ];
    let metrics = W::collect_table_column_metrics(&header, &rows, /*column_count*/ 2);
    assert_eq!(metrics[0].kind, TableColumnKind::Compact);
    assert_eq!(metrics[1].kind, TableColumnKind::Narrative);
}

#[test]
// URL 形长令牌主导的列应分类为长令牌列。
fn column_classification_token_heavy_by_url_like_tokens() {
    let header = vec![make_cell("URL")];
    let rows = vec![
        vec![make_cell("https://example.com/very/long/path")],
        vec![make_cell("https://another.example.org/deep")],
    ];
    let metrics = W::collect_table_column_metrics(&header, &rows, /*column_count*/ 1);
    assert_eq!(metrics[0].kind, TableColumnKind::TokenHeavy);
}

#[test]
// 本地路径列表同样应触发长令牌列分类。
fn column_classification_token_heavy_for_local_path_lists() {
    let header = vec![make_cell("Files")];
    let rows = vec![
        vec![make_cell(
            "codex-rs/core/src/next_prompt_suggestion.rs:1, codex-rs/core/src/next_prompt_suggestion_tests.rs:1",
        )],
        vec![make_cell(
            "codex-rs/core/src/context/next_prompt_suggestion.rs:1, codex-rs/core/src/context/contextual_user_message_tests.rs:1",
        )],
    ];
    let metrics = W::collect_table_column_metrics(&header, &rows, /*column_count*/ 1);
    assert_eq!(metrics[0].kind, TableColumnKind::TokenHeavy);
}

#[test]
// 全部短值的列应保持紧凑分类。
fn column_classification_compact_all_short() {
    // 两列都是短令牌，因此都应分类为紧凑。
    // Both columns short tokens -> both Compact
    let header = vec![make_cell("Status"), make_cell("Count")];
    let rows =
        vec![vec![make_cell("ok"), make_cell("42")], vec![make_cell("err"), make_cell("7")]];
    let metrics = W::collect_table_column_metrics(&header, &rows, /*column_count*/ 2);
    assert_eq!(metrics[0].kind, TableColumnKind::Compact);
    assert_eq!(metrics[1].kind, TableColumnKind::Compact);
}

#[test]
// 叙述列的首选下限应保留可读正文宽度。
fn preferred_floor_narrative_retains_readable_width() {
    let m = TableColumnMetrics {
        max_width: 40,
        header_token_width: 15,
        body_token_width: 8,
        kind: TableColumnKind::Narrative,
    };
    assert_eq!(W::preferred_column_floor(&m, /*min_column_width*/ 3), 16);

    let m2 = TableColumnMetrics {
        max_width: 12,
        header_token_width: 6,
        body_token_width: 8,
        kind: TableColumnKind::Narrative,
    };
    assert_eq!(W::preferred_column_floor(&m2, /*min_column_width*/ 3), 12);
}

#[test]
// 长令牌列在进一步缩减前也保留可读软下限。
fn preferred_floor_token_heavy_retains_readable_width() {
    let m = TableColumnMetrics {
        max_width: 80,
        header_token_width: 5,
        body_token_width: 60,
        kind: TableColumnKind::TokenHeavy,
    };
    assert_eq!(W::preferred_column_floor(&m, /*min_column_width*/ 3), 16);
}

#[test]
// 紧凑列下限应考虑正文最长令牌且遵守封顶值。
fn preferred_floor_compact_uses_body_token() {
    // 紧凑列下限取表头令牌宽度与正文令牌宽度上限 16 的较大者。
    // Compact: max(header_token_width, body_token_width.min(16))
    let m = TableColumnMetrics {
        max_width: 30,
        header_token_width: 5,
        body_token_width: 12,
        kind: TableColumnKind::Compact,
    };
    // 此例结果为 12：max(5, min(12, 16)) = max(5, 12) = 12。
    // max(5, min(12, 16)) = max(5, 12) = 12
    assert_eq!(W::preferred_column_floor(&m, /*min_column_width*/ 3), 12);

    // 正文令牌超过 16 时先封顶，再与表头宽度取较大者。
    // Body token exceeds 16 cap → capped at 16, then max with header
    let m2 = TableColumnMetrics {
        max_width: 30,
        header_token_width: 5,
        body_token_width: 20,
        kind: TableColumnKind::Compact,
    };
    // 此例正文先封顶为 16，最终结果为 16。
    // max(5, min(20, 16)) = max(5, 16) = 16
    assert_eq!(W::preferred_column_floor(&m2, /*min_column_width*/ 3), 16);
}

#[test]
// 批量缩减应与逐显示单元缩减产生相同宽度分配。
fn bulk_column_shrink_matches_one_cell_at_a_time() {
    fn priority(kind: TableColumnKind) -> usize {
        match kind {
            TableColumnKind::TokenHeavy => 0,
            TableColumnKind::Narrative => 1,
            TableColumnKind::Compact => 2,
        }
    }

    fn shrink_one_at_a_time(
        widths: &mut [usize],
        floors: &[usize],
        metrics: &[TableColumnMetrics],
        mut amount: usize,
    ) -> usize {
        while amount > 0 {
            let Some(idx) = widths
                .iter()
                .enumerate()
                .filter(|(idx, width)| **width > floors[*idx])
                .min_by_key(|(idx, width)| {
                    (
                        priority(metrics[*idx].kind),
                        usize::MAX - width.saturating_sub(floors[*idx]),
                    )
                })
                .map(|(idx, _)| idx)
            else {
                break;
            };
            widths[idx] -= 1;
            amount -= 1;
        }
        amount
    }

    for case in 0..64usize {
        let metrics = (0..5)
            .map(|idx| TableColumnMetrics {
                max_width: 100,
                header_token_width: 8,
                body_token_width: 8,
                kind: match (case + idx) % 3 {
                    0 => TableColumnKind::TokenHeavy,
                    1 => TableColumnKind::Narrative,
                    _ => TableColumnKind::Compact,
                },
            })
            .collect::<Vec<_>>();
        let floors = (0..5).map(|idx| 3 + (case * (idx + 1) + idx) % 14).collect::<Vec<_>>();
        let initial = floors
            .iter()
            .enumerate()
            .map(|(idx, floor)| floor + (case * (idx + 3) + idx * 7) % 41)
            .collect::<Vec<_>>();
        let slack =
            initial.iter().zip(&floors).map(|(width, floor)| width - floor).sum::<usize>();

        for amount in 0..=slack + 1 {
            let mut expected = initial.clone();
            let expected_remaining =
                shrink_one_at_a_time(&mut expected, &floors, &metrics, amount);
            let mut actual = initial.clone();
            let actual_remaining = W::shrink_columns(&mut actual, &floors, &metrics, amount);
            assert_eq!((actual, actual_remaining), (expected, expected_remaining));
        }
    }
}

#[test]
// 超大长令牌列应先被批量压缩，且总宽度满足预算。
fn column_widths_bulk_shrink_large_token_heavy_column() {
    let metrics = [
        TableColumnMetrics {
            max_width: 1_000_000,
            header_token_width: 4,
            body_token_width: 1_000_000,
            kind: TableColumnKind::TokenHeavy,
        },
        TableColumnMetrics {
            max_width: 8,
            header_token_width: 5,
            body_token_width: 8,
            kind: TableColumnKind::Compact,
        },
    ];

    assert_eq!(
        W::compute_column_widths(&metrics, /*available_width*/ Some(24)),
        Some(vec![16, 8])
    );
}

// 以下单元测试覆盖表格溢出行检测的正反例。
// ===== Spillover-detection unit tests =====

#[test]
// 无管道语法的单单元格正文行应识别为溢出段落。
fn spillover_detects_single_cell_row() {
    let row = make_body_row(
        vec![make_cell("some trailing text")],
        /*has_table_pipe_syntax*/ false,
    );
    assert!(W::is_spillover_row(&row, /*next_row*/ None));
}

#[test]
// 显式使用表格管道语法的单单元格行应留在表格中。
fn spillover_keeps_single_cell_row_with_table_pipe_syntax() {
    let row = make_body_row(
        vec![make_cell("some sparse value")],
        /*has_table_pipe_syntax*/ true,
    );
    assert!(!W::is_spillover_row(&row, /*next_row*/ None));
}

#[test]
// 只有首格含 HTML 的稀疏行应识别为溢出。
fn spillover_detects_html_content() {
    // 三单元格行中只有首格含 HTML，应识别为溢出。
    // 3-cell row where only cell 0 has HTML content
    let row = make_body_row(
        vec![make_cell("<div>content</div>"), make_cell(""), make_cell("")],
        /*has_table_pipe_syntax*/ false,
    );
    assert!(W::is_spillover_row(&row, /*next_row*/ None));
}

#[test]
// HTML 引导标签后接 HTML 行时两者应作为段落溢出。
fn spillover_detects_label_followed_by_html() {
    // 首格为 HTML 引导标签、下一行首格为 HTML 块时，两者应一起溢出。
    // cell 0 = "HTML block:" and next_row cell 0 = "<div>x</div>"
    let row = make_body_row(
        vec![make_cell("HTML block:"), make_cell(""), make_cell("")],
        /*has_table_pipe_syntax*/ false,
    );
    let next = make_body_row(
        vec![make_cell("<div>x</div>"), make_cell(""), make_cell("")],
        /*has_table_pipe_syntax*/ false,
    );
    assert!(W::is_spillover_row(&row, Some(&next)));
}

#[test]
// 表格末尾孤立的 HTML 引导标签仍应识别为溢出。
fn spillover_detects_trailing_html_label() {
    // 没有下一行的尾随 HTML 引导标签也视为溢出。
    // "HTML block:" with no next_row → trailing HTML label spillover
    let row = make_body_row(
        vec![make_cell("HTML block:"), make_cell(""), make_cell("")],
        /*has_table_pipe_syntax*/ false,
    );
    assert!(W::is_spillover_row(&row, /*next_row*/ None));
}

#[test]
// 多个非空单元格构成的正常行不得被误判为溢出。
fn spillover_keeps_normal_multi_cell_row() {
    // 三个单元格均非空时是正常表格行，不应溢出。
    // 3 cells all non-empty → not spillover
    let row = make_body_row(
        vec![make_cell("one"), make_cell("two"), make_cell("three")],
        /*has_table_pipe_syntax*/ true,
    );
    assert!(!W::is_spillover_row(&row, /*next_row*/ None));
}

#[test]
// 后续内容不是 HTML 时，普通标签行继续作为真实表格内容。
fn spillover_keeps_label_when_next_is_not_html() {
    // 普通 `Status:` 标签后接 `ok` 不像 HTML，不应判为溢出。
    // cell 0 = "Status:" and next_row cell 0 = "ok" → not spillover (not HTML)
    let row = make_body_row(
        vec![make_cell("Status:"), make_cell(""), make_cell("")],
        /*has_table_pipe_syntax*/ true,
    );
    let next = make_body_row(
        vec![make_cell("ok"), make_cell(""), make_cell("")],
        /*has_table_pipe_syntax*/ true,
    );
    assert!(!W::is_spillover_row(&row, Some(&next)));
}

#[test]
// 显式 Web 链接标签和可见目标都应附带正确注解。
fn annotates_explicit_web_link_label_and_visible_destination() {
    let lines = render_markdown_lines_with_width_and_cwd(
        "See [docs](https://example.com/reference).",
        /*width*/ Some(80),
        /*cwd*/ None,
    );
    let links = lines.iter().flat_map(|line| line.hyperlinks.iter()).collect::<Vec<_>>();

    assert_eq!(links.len(), 2);
    assert!(links.iter().all(|link| link.destination == "https://example.com/reference"));
}

#[test]
// 表格 URL 折行后仍应在注解中保留完整目的地及语义片段。
fn wrapped_table_url_fragments_keep_complete_web_destination() {
    let destination = "https://example.com/a/very/long/path/to/a/table/artifact";
    let markdown = format!("| Item | URL |\n| --- | --- |\n| report | {destination} |\n");
    let lines = render_markdown_lines_with_width_and_cwd(
        &markdown,
        /*width*/ Some(32),
        /*cwd*/ None,
    );
    let linked_rows =
        lines.iter().filter(|line| !line.hyperlinks.is_empty()).collect::<Vec<_>>();

    assert!(linked_rows.len() > 1, "expected a URL wrapped across table rows");
    assert!(
        linked_rows
            .iter()
            .all(|line| { line.hyperlinks.iter().all(|link| link.destination == destination) })
    );
}

#[test]
// 表格列宽测量应按半角浊音符的实际显示宽度计算。
fn table_widths_count_halfwidth_sound_marks() {
    let cell = make_cell("ｶﾞﾊﾟ");
    assert_eq!(W::cell_display_width(&cell), 4);
    assert_eq!(W::longest_token_width("ｶﾞﾊﾟtail"), 8);
}

#[test]
// 键值表格回退应继续保留 Web 链接注解。
fn key_value_table_keeps_web_annotations() {
    let destination = "https://example.com/a/very/long/path";
    let markdown = format!(
        "| c1 | c2 | c3 | c4 | c5 | c6 |\n| --- | --- | --- | --- | --- | --- |\n| {destination} | 2 | 3 | 4 | 5 | 6 |\n"
    );
    let lines = render_markdown_lines_with_width_and_cwd(
        &markdown,
        /*width*/ Some(20),
        /*cwd*/ None,
    );
    let destinations = lines
        .iter()
        .flat_map(|line| line.hyperlinks.iter().map(|link| link.destination.as_str()))
        .collect::<Vec<_>>();

    assert!(!destinations.is_empty());
    assert!(destinations.iter().all(|link| *link == destination));
}

#[test]
// 代码内容和非 Web Markdown 链接不应获得 Web 注解。
fn does_not_annotate_code_or_non_web_markdown_links() {
    let markdown = "`https://example.com/inline`\n\n```text\nhttps://example.com/block\n```\n\n[mail](mailto:test@example.com)\n\n[https://example.com/label](mailto:test@example.com)\n\n| Target |\n| --- |\n| [https://example.com/table-label](mailto:test@example.com) |";
    let lines = render_markdown_lines_with_width_and_cwd(
        markdown,
        /*width*/ Some(80),
        /*cwd*/ None,
    );

    assert!(lines.iter().all(|line| line.hyperlinks.is_empty()));
}

#[test]
// 管道表格回退路径也必须保留 Web 目的地注解。
fn pipe_table_fallback_keeps_web_annotations() {
    let destination = "https://example.com/a/long/path";
    let target = "https://target.example/path";
    let code_url = "https://code.example/not-a-link";
    let markdown = format!(
        "| URL | Code | Label |\n| --- | --- | --- |\n| {destination} | `{code_url}` | [https://shown.example]({target}) |\n"
    );
    let lines = render_markdown_lines_with_width_and_cwd(
        &markdown,
        /*width*/ Some(5),
        /*cwd*/ None,
    );
    let destinations = lines
        .iter()
        .flat_map(|line| line.hyperlinks.iter().map(|link| link.destination.as_str()))
        .collect::<Vec<_>>();

    assert!(destinations.contains(&destination));
    assert!(destinations.contains(&target));
    assert!(!destinations.contains(&code_url));
    assert!(!destinations.contains(&"https://shown.example"));
}
