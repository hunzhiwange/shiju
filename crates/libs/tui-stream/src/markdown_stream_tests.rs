use super::*;
use ratatui::style::Color;

#[tokio::test]
// 没有换行的局部源码不得产生提交。
async fn no_commit_until_newline() {
    let mut c = super::MarkdownStreamCollector::new(/*width*/ None, &super::test_cwd());
    c.push_delta("Hello, world");
    let out = c.commit_complete_lines();
    assert!(out.is_empty(), "should not commit without newline");
    c.push_delta("!\n");
    let out2 = c.commit_complete_lines();
    assert_eq!(out2.len(), 1, "one completed line after newline");
}

#[tokio::test]
// 定稿会提交最后一条没有尾随换行的局部行。
async fn finalize_commits_partial_line() {
    let mut c = super::MarkdownStreamCollector::new(/*width*/ None, &super::test_cwd());
    c.push_delta("Line without newline");
    let out = c.finalize_and_drain();
    assert_eq!(out.len(), 1);
}

#[tokio::test]
// 简单块引用经流式提交后保持绿色引用样式。
async fn e2e_stream_blockquote_simple_is_green() {
    let out = super::simulate_stream_markdown_for_tests(&["> Hello\n"], /*finalize*/ true);
    assert_eq!(out.len(), 1);
    let l = &out[0];
    assert_eq!(
        l.style.fg,
        Some(Color::Green),
        "expected blockquote line fg green, got {:?}",
        l.style.fg
    );
}

#[tokio::test]
// 嵌套块引用的层级与绿色样式应跨分块保持。
async fn e2e_stream_blockquote_nested_is_green() {
    let out = super::simulate_stream_markdown_for_tests(
        &["> Level 1\n>> Level 2\n"],
        /*finalize*/ true,
    );
    // 过滤段落开头可能插入的空行。
    // Filter out any blank lines that may be inserted at paragraph starts.
    let non_blank: Vec<_> = out
        .into_iter()
        .filter(|l| {
            let s = l.spans.iter().map(|sp| sp.content.clone()).collect::<Vec<_>>().join("");
            let t = s.trim();
            // 忽略段落边界插入的仅含 `>` 的引用空行。
            // Ignore quote-only blank lines like ">" inserted at paragraph boundaries.
            !(t.is_empty() || t == ">")
        })
        .collect();
    assert_eq!(non_blank.len(), 2);
    assert_eq!(non_blank[0].style.fg, Some(Color::Green));
    assert_eq!(non_blank[1].style.fg, Some(Color::Green));
}

#[tokio::test]
// 块引用内列表项在流式渲染中仍保留引用样式和标记。
async fn e2e_stream_blockquote_with_list_items_is_green() {
    let out = super::simulate_stream_markdown_for_tests(
        &["> - item 1\n> - item 2\n"],
        /*finalize*/ true,
    );
    assert_eq!(out.len(), 2);
    assert_eq!(out[0].style.fg, Some(Color::Green));
    assert_eq!(out[1].style.fg, Some(Color::Green));
}

#[tokio::test]
// 混合嵌套列表的有序标记应使用浅蓝强调色。
async fn e2e_stream_nested_mixed_lists_ordered_marker_is_light_blue() {
    let md = [
        "1. First\n",
        "   - Second level\n",
        "     1. Third level (ordered)\n",
        "        - Fourth level (bullet)\n",
        "          - Fifth level to test indent consistency\n",
    ];
    let out = super::simulate_stream_markdown_for_tests(&md, /*finalize*/ true);
    // 查找包含第三层有序列表正文的行。
    // Find the line that contains the third-level ordered text
    let find_idx = out.iter().position(|l| {
        l.spans
            .iter()
            .map(|s| s.content.clone())
            .collect::<String>()
            .contains("Third level (ordered)")
    });
    let idx = find_idx.expect("expected third-level ordered line");
    let line = &out[idx];
    // 预期该行至少一个 span 使用浅蓝样式。
    // Expect at least one span on this line to be styled light blue
    let has_light_blue =
        line.spans.iter().any(|s| s.style.fg == Some(ratatui::style::Color::LightBlue));
    assert!(
        has_light_blue,
        "expected an ordered-list marker span with light blue fg on: {line:?}"
    );
}

#[tokio::test]
// 窄宽度折行后，块引用的所有续行仍保持绿色样式。
async fn e2e_stream_blockquote_wrap_preserves_green_style() {
    let long = "> This is a very long quoted line that should wrap across multiple columns to verify style preservation.";
    let out = super::simulate_stream_markdown_for_tests(&[long, "\n"], /*finalize*/ true);
    // 使用窄宽度强制产生多条输出行。
    // Wrap to a narrow width to force multiple output lines.
    let wrapped = tui_text::wrapping::word_wrap_lines(
        out.iter(),
        tui_text::wrapping::RtOptions::new(/*width*/ 24),
    );
    // 过滤纯空白行。
    // Filter out purely blank lines
    let non_blank: Vec<_> = wrapped
        .into_iter()
        .filter(|l| {
            let s = l.spans.iter().map(|sp| sp.content.clone()).collect::<Vec<_>>().join("");
            !s.trim().is_empty()
        })
        .collect();
    assert!(non_blank.len() >= 2, "expected wrapped blockquote to span multiple lines");
    for (i, l) in non_blank.iter().enumerate() {
        assert_eq!(
            l.spans[0].style.fg,
            Some(Color::Green),
            "wrapped line {} should preserve green style, got {:?}",
            i,
            l.spans[0].style.fg
        );
    }
}

#[tokio::test]
// 段落后的标题应开始新行，不与前文拼接。
async fn heading_starts_on_new_line_when_following_paragraph() {
    // 先流式输入段落行，再在下一行输入标题；两者应分别渲染。
    // Stream a paragraph line, then a heading on the next line.
    // Expect two distinct rendered lines: "Hello." and "Heading".
    let mut c = super::MarkdownStreamCollector::new(/*width*/ None, &super::test_cwd());
    c.push_delta("Hello.\n");
    let out1 = c.commit_complete_lines();
    let s1: Vec<String> = out1
        .iter()
        .map(|l| l.spans.iter().map(|s| s.content.clone()).collect::<Vec<_>>().join(""))
        .collect();
    assert_eq!(
        out1.len(),
        1,
        "first commit should contain only the paragraph line, got {}: {:?}",
        out1.len(),
        s1
    );

    c.push_delta("## Heading\n");
    let out2 = c.commit_complete_lines();
    let s2: Vec<String> = out2
        .iter()
        .map(|l| l.spans.iter().map(|s| s.content.clone()).collect::<Vec<_>>().join(""))
        .collect();
    assert_eq!(s2, vec!["", "## Heading"], "expected a blank separator then the heading line");

    let line_to_string = |l: &ratatui::text::Line<'_>| -> String {
        l.spans.iter().map(|s| s.content.clone()).collect::<Vec<_>>().join("")
    };

    assert_eq!(line_to_string(&out1[0]), "Hello.");
    assert_eq!(line_to_string(&out2[1]), "## Heading");
}

#[tokio::test]
// 标题标记跨分块到达时也不得内联进前一段落。
async fn heading_not_inlined_when_split_across_chunks() {
    // 先输入无尾随换行段落，再由下一分块补换行和标题，最后补标题换行。
    // 收集器应先只提交段落，之后再独立提交标题。
    // Paragraph without trailing newline, then a chunk that starts with the newline
    // and the heading text, then a final newline. The collector should first commit
    // only the paragraph line, and later commit the heading as its own line.
    let mut c = super::MarkdownStreamCollector::new(/*width*/ None, &super::test_cwd());
    c.push_delta("Sounds good!");
    // 尚未达到提交边界。
    // No commit yet
    assert!(c.commit_complete_lines().is_empty());

    // 引入完成段落的换行以及标题开头。
    // Introduce the newline that completes the paragraph and the start of the heading.
    c.push_delta("\n## Adding Bird subcommand");
    let out1 = c.commit_complete_lines();
    let s1: Vec<String> = out1
        .iter()
        .map(|l| l.spans.iter().map(|s| s.content.clone()).collect::<Vec<_>>().join(""))
        .collect();
    assert_eq!(
        s1,
        vec!["Sounds good!"],
        "expected paragraph followed by blank separator before heading chunk"
    );

    // 用尾随换行完成标题行。
    // Now finish the heading line with the trailing newline.
    c.push_delta("\n");
    let out2 = c.commit_complete_lines();
    let s2: Vec<String> = out2
        .iter()
        .map(|l| l.spans.iter().map(|s| s.content.clone()).collect::<Vec<_>>().join(""))
        .collect();
    assert_eq!(
        s2,
        vec!["", "## Adding Bird subcommand"],
        "expected the heading line only on the final commit"
    );

    // 校验简单原始 Markdown 行不会产生多余内容。
    // Sanity check raw markdown rendering for a simple line does not produce spurious extras.
    let mut rendered: Vec<ratatui::text::Line<'static>> = Vec::new();
    let test_cwd = super::test_cwd();
    tui_markdown::markdown::append_markdown(
        "Hello.\n",
        /*width*/ None,
        Some(test_cwd.as_path()),
        &mut rendered,
    );
    let rendered_strings: Vec<String> = rendered
        .iter()
        .map(|l| l.spans.iter().map(|s| s.content.clone()).collect::<Vec<_>>().join(""))
        .collect();
    assert_eq!(
        rendered_strings,
        vec!["Hello."],
        "unexpected markdown lines: {rendered_strings:?}"
    );
}

fn lines_to_plain_strings(lines: &[ratatui::text::Line<'_>]) -> Vec<String> {
    lines
        .iter()
        .map(|l| l.spans.iter().map(|s| s.content.clone()).collect::<Vec<_>>().join(""))
        .collect()
}

#[tokio::test]
// 收集器自身不做表格暂留，换行完成的表头应立即提交。
async fn table_header_commits_without_holdback() {
    let mut c = super::MarkdownStreamCollector::new(/*width*/ None, &super::test_cwd());
    c.push_delta("| A | B |\n");
    let out1 = c.commit_complete_lines();
    let out1_str = lines_to_plain_strings(&out1);
    assert_eq!(out1_str, vec!["| A | B |".to_string()]);

    c.push_delta("| --- | --- |\n");
    let out = c.commit_complete_lines();
    let out_str = lines_to_plain_strings(&out);
    assert!(
        !out_str.is_empty(),
        "expected output to continue committing after delimiter: {out_str:?}"
    );

    c.push_delta("| 1 | 2 |\n");
    let out2 = c.commit_complete_lines();
    assert!(!out2.is_empty(), "expected output to continue committing after body row");

    c.push_delta("\n");
    let _ = c.commit_complete_lines();
}

#[tokio::test]
// 没有表格前缀结构的管道文本不应延迟提交。
async fn pipe_text_without_table_prefix_is_not_delayed() {
    let mut c = super::MarkdownStreamCollector::new(/*width*/ None, &super::test_cwd());
    c.push_delta("Escaped pipe in text: a | b | c\n");
    let out = c.commit_complete_lines();
    let out_str = lines_to_plain_strings(&out);
    assert_eq!(out_str, vec!["Escaped pipe in text: a | b | c".to_string()]);
}

#[tokio::test]
// 列表和围栏分块提交时不得重复任何已发出内容。
async fn lists_and_fences_commit_without_duplication() {
    // 列表场景。
    // List case
    assert_streamed_equals_full(&["- a\n- ", "b\n- c\n"]).await;

    // 围栏代码场景：使用小分块流式输入。
    // Fenced code case: stream in small chunks
    assert_streamed_equals_full(&["```", "\nco", "de 1\ncode 2\n", "```\n"]).await;
}

#[tokio::test]
// UTF-8 边界、宽字符、控制字符与组合字符序列均不得被错误切分。
async fn utf8_boundary_safety_and_wide_chars() {
    // 覆盖宽 Emoji、中日韩字符、控制字符及数字加组合长音符序列。
    // Emoji (wide), CJK, control char, digit + combining macron sequences
    let input = "🙂🙂🙂\n汉字漢字\nA\u{0003}0\u{0304}\n";
    let deltas = vec!["🙂", "🙂", "🙂\n汉", "字漢", "字\nA", "\u{0003}", "0", "\u{0304}", "\n"];

    let streamed = simulate_stream_markdown_for_tests(&deltas, /*finalize*/ true);
    let streamed_str = lines_to_plain_strings(&streamed);

    let mut rendered_all: Vec<ratatui::text::Line<'static>> = Vec::new();
    let test_cwd = super::test_cwd();
    tui_markdown::markdown::append_markdown(
        input,
        /*width*/ None,
        Some(test_cwd.as_path()),
        &mut rendered_all,
    );
    let rendered_all_str = lines_to_plain_strings(&rendered_all);

    assert_eq!(
        streamed_str, rendered_all_str,
        "utf8/wide-char streaming should equal full render without duplication or truncation"
    );
}

#[tokio::test]
// 深层嵌套第三层有序标记仍使用浅蓝色，正文保持默认色。
async fn e2e_stream_deep_nested_third_level_marker_is_light_blue() {
    let md = "1. First\n   - Second level\n     1. Third level (ordered)\n        - Fourth level (bullet)\n          - Fifth level to test indent consistency\n";
    let streamed = super::simulate_stream_markdown_for_tests(&[md], /*finalize*/ true);
    let streamed_strs = lines_to_plain_strings(&streamed);

    // 在流式输出中定位第三层行，不依赖精确缩进宽度。
    // Locate the third-level line in the streamed output; avoid relying on exact indent.
    let target_suffix = "1. Third level (ordered)";
    let mut found = None;
    for line in &streamed {
        let s: String = line.spans.iter().map(|sp| sp.content.clone()).collect();
        if s.contains(target_suffix) {
            found = Some(line.clone());
            break;
        }
    }
    let line = found.unwrap_or_else(|| {
        panic!("expected to find the third-level ordered list line; got: {streamed_strs:?}")
    });

    // 包含缩进和 `1.` 的标记应位于首个 span 并为浅蓝色，后续正文保持默认色。
    // The marker (including indent and "1.") is expected to be in the first span
    // and colored LightBlue; following content should be default color.
    assert!(!line.spans.is_empty(), "expected non-empty spans for the third-level line");
    let marker_span = &line.spans[0];
    assert_eq!(
        marker_span.style.fg,
        Some(Color::LightBlue),
        "expected LightBlue 3rd-level ordered marker, got {:?}",
        marker_span.style.fg
    );
    // 找到首个非空正文 span 并确认使用默认颜色。
    // Find the first non-empty non-space content span and verify it is default color.
    let mut content_fg = None;
    for sp in &line.spans[1..] {
        let t = sp.content.trim();
        if !t.is_empty() {
            content_fg = Some(sp.style.fg);
            break;
        }
    }
    assert_eq!(
        content_fg.flatten(),
        None,
        "expected default color for 3rd-level content, got {content_fg:?}"
    );
}

#[tokio::test]
// 空围栏代码块应被省略，但在后续标题前保留必要分隔。
async fn empty_fenced_block_is_dropped_and_separator_preserved_before_heading() {
    // 空围栏后接标题时不渲染围栏，但保留空白分隔使标题另起一行。
    // An empty fenced code block followed by a heading should not render the fence,
    // but should preserve a blank separator line so the heading starts on a new line.
    let deltas = vec!["```bash\n```\n", "## Heading\n"]; // empty block and close in same commit
    let streamed = simulate_stream_markdown_for_tests(&deltas, /*finalize*/ true);
    let texts = lines_to_plain_strings(&streamed);
    assert!(texts.iter().all(|s| !s.contains("```")), "no fence markers expected: {texts:?}");
    // 预期存在标题且无围栏标记；起始空白分隔可有可无。
    // Expect the heading and no fence markers. A blank separator may or may not be rendered at start.
    assert!(texts.iter().any(|s| s == "## Heading"), "expected heading line: {texts:?}");
}

#[tokio::test]
// 段落、空围栏和标题组合中，标题仍必须另起一行。
async fn paragraph_then_empty_fence_then_heading_keeps_heading_on_new_line() {
    let deltas = vec!["Para.\n", "```\n```\n", "## Title\n"]; // empty fence block in one commit
    let streamed = simulate_stream_markdown_for_tests(&deltas, /*finalize*/ true);
    let texts = lines_to_plain_strings(&streamed);
    let para_idx = match texts.iter().position(|s| s == "Para.") {
        Some(i) => i,
        None => panic!("para present"),
    };
    let head_idx = match texts.iter().position(|s| s == "## Title") {
        Some(i) => i,
        None => panic!("heading present"),
    };
    assert!(head_idx > para_idx, "heading should not merge with paragraph: {texts:?}");
}

#[tokio::test]
// 列表短横线跨分块时，松散列表结果应匹配完整渲染。
async fn loose_list_with_split_dashes_matches_full_render() {
    // 辅助工具发现的最小失败序列：两个分块仍可复现差异。
    // Minimized failing sequence discovered by the helper: two chunks
    // that still reproduce the mismatch.
    let deltas = vec!["- item.\n\n", "-"];

    let streamed = simulate_stream_markdown_for_tests(&deltas, /*finalize*/ true);
    let streamed_strs = lines_to_plain_strings(&streamed);

    let full: String = deltas.iter().copied().collect();
    let mut rendered_all: Vec<ratatui::text::Line<'static>> = Vec::new();
    let test_cwd = super::test_cwd();
    tui_markdown::markdown::append_markdown(
        &full,
        /*width*/ None,
        Some(test_cwd.as_path()),
        &mut rendered_all,
    );
    let rendered_all_strs = lines_to_plain_strings(&rendered_all);

    assert_eq!(
        streamed_strs, rendered_all_strs,
        "streamed output should match full render without dangling '-' lines"
    );
}

#[tokio::test]
// 松散与紧凑列表项在流式和完整渲染之间应保持一致。
async fn loose_vs_tight_list_items_streaming_matches_full() {
    // 增量取自指定时间附近的会话日志，用于复现真实分块顺序。
    // Deltas extracted from the session log around 2025-08-27T00:33:18.216Z
    let deltas = vec![
        "\n\n",
        "Loose",
        " vs",
        ".",
        " tight",
        " list",
        " items",
        ":\n",
        "1",
        ".",
        " Tight",
        " item",
        "\n",
        "2",
        ".",
        " Another",
        " tight",
        " item",
        "\n\n",
        "1",
        ".",
        " Loose",
        " item",
        " with",
        " its",
        " own",
        " paragraph",
        ".\n\n",
        "  ",
        " This",
        " paragraph",
        " belongs",
        " to",
        " the",
        " same",
        " list",
        " item",
        ".\n\n",
        "2",
        ".",
        " Second",
        " loose",
        " item",
        " with",
        " a",
        " nested",
        " list",
        " after",
        " a",
        " blank",
        " line",
        ".\n\n",
        "  ",
        " -",
        " Nested",
        " bullet",
        " under",
        " a",
        " loose",
        " item",
        "\n",
        "  ",
        " -",
        " Another",
        " nested",
        " bullet",
        "\n\n",
    ];

    let streamed = simulate_stream_markdown_for_tests(&deltas, /*finalize*/ true);
    let streamed_strs = lines_to_plain_strings(&streamed);

    // 完整渲染只用于差异诊断。
    // Compute a full render for diagnostics only.
    let full: String = deltas.iter().copied().collect();
    let mut rendered_all: Vec<ratatui::text::Line<'static>> = Vec::new();
    let test_cwd = super::test_cwd();
    tui_markdown::markdown::append_markdown(
        &full,
        /*width*/ None,
        Some(test_cwd.as_path()),
        &mut rendered_all,
    );

    // 同时断言精确纯文本，便于理解失败含义。
    // Also assert exact expected plain strings for clarity.
    let expected = vec![
        "Loose vs. tight list items:".to_string(),
        "".to_string(),
        "1. Tight item".to_string(),
        "2. Another tight item".to_string(),
        "3. Loose item with its own paragraph.".to_string(),
        "".to_string(),
        "   This paragraph belongs to the same list item.".to_string(),
        "".to_string(),
        "4. Second loose item with a nested list after a blank line.".to_string(),
        "    - Nested bullet under a loose item".to_string(),
        "    - Another nested bullet".to_string(),
    ];
    assert_eq!(
        streamed_strs, expected,
        "expected exact rendered lines for loose/tight section"
    );
}

// 以下定向测试来自模糊测试发现，每个都断言流式结果等于完整渲染。
// Targeted tests derived from fuzz findings. Each asserts streamed == full render.
async fn assert_streamed_equals_full(deltas: &[&str]) {
    let streamed = simulate_stream_markdown_for_tests(deltas, /*finalize*/ true);
    let streamed_strs = lines_to_plain_strings(&streamed);
    let full: String = deltas.iter().copied().collect();
    let mut rendered: Vec<ratatui::text::Line<'static>> = Vec::new();
    let test_cwd = super::test_cwd();
    tui_markdown::markdown::append_markdown(
        &full,
        /*width*/ None,
        Some(test_cwd.as_path()),
        &mut rendered,
    );
    let rendered_strs = lines_to_plain_strings(&rendered);
    assert_eq!(streamed_strs, rendered_strs, "full:\n---\n{full}\n---");
}

#[tokio::test]
// 模糊测试发现的第一类项目符号重复变体不得回归。
async fn fuzz_class_bullet_duplication_variant_1() {
    assert_streamed_equals_full(&[
        "aph.\n- let one\n- bull",
        "et two\n\n  second paragraph \n",
    ])
    .await;
}

#[tokio::test]
// 模糊测试发现的第二类项目符号重复变体不得回归。
async fn fuzz_class_bullet_duplication_variant_2() {
    assert_streamed_equals_full(&[
        "- e\n  c",
        "e\n- bullet two\n\n  second paragraph in bullet two\n",
    ])
    .await;
}

#[tokio::test]
// HTML 块后接正文时，增量结果应与完整渲染一致。
async fn streaming_html_block_then_text_matches_full() {
    assert_streamed_equals_full(&[
        "HTML block:\n",
        "<div>inline block</div>\n",
        "more stuff\n",
    ])
    .await;
}

#[tokio::test]
// 围栏代码内的表格形行按代码处理，不参与暂留。
async fn table_like_lines_inside_fenced_code_are_not_held() {
    assert_streamed_equals_full(&["```\n", "| a | b |\n", "```\n"]).await;
}

#[tokio::test]
// 收集器源码分块重新组合后应能正确进入助手围栏解包路径。
async fn collector_source_chunks_round_trip_into_agent_fence_unwrapping() {
    let deltas = ["```md\n", "| A | B |\n", "|---|---|\n", "| 1 | 2 |\n", "```\n"];
    let mut collector =
        super::MarkdownStreamCollector::new(/*width*/ None, &super::test_cwd());
    let mut committed_source = String::new();

    for delta in deltas {
        collector.push_delta(delta);
        if delta.contains('\n')
            && let Some(range) = collector.commit_complete_source()
        {
            committed_source.push_str(&collector.committed_source()[range]);
        }
    }
    assert_eq!(collector.committed_source(), committed_source);
    let raw_source = collector.finalize_and_take_source();

    let mut rendered = Vec::new();
    tui_markdown::markdown::append_markdown_agent(&raw_source, /*width*/ None, &mut rendered);
    let rendered_strs = lines_to_plain_strings(&rendered);

    assert!(
        rendered_strs.iter().any(|line| line.contains('━')),
        "expected markdown-fenced table to render with a separator: {rendered_strs:?}"
    );
    assert!(
        !rendered_strs.iter().any(|line| line.trim() == "| A | B |"),
        "did not expect raw table header after markdown-fence unwrapping: {rendered_strs:?}"
    );
}

#[test]
// 空收集器定稿应返回空源码并保持可复用状态。
fn finalizing_empty_collector_returns_empty_source() {
    let mut collector =
        super::MarkdownStreamCollector::new(/*width*/ None, &super::test_cwd());

    assert_eq!(collector.finalize_and_take_source(), String::new());
}
