use super::*;
use crate::markdown_render::render_markdown_text;
use crate::test_backend::VT100Backend;
use ratatui::layout::Rect;
use ratatui::style::Color;
use ratatui::text::Line;
use ratatui::text::Span;
use tui_render::terminal_hyperlinks::plain_hyperlink_lines;

#[test]
fn vt100_blockquote_line_emits_green_fg() {
    // 设置小型离屏终端。
    // Set up a small off-screen terminal
    let width: u16 = 40;
    let height: u16 = 10;
    let backend = VT100Backend::new(width, height);
    let mut term = crate::custom_terminal::Terminal::with_options(backend).expect("terminal");
    // 将视口放到最后一行，使历史插入向上滚动。
    // Place viewport on the last line so history inserts scroll upward
    let viewport = Rect::new(0, height - 1, width, 1);
    term.set_viewport_area(viewport);

    // 构造块引用样行：应用行级绿色和 `> ` 前缀。
    // Build a blockquote-like line: apply line-level green style and prefix "> "
    let mut line: Line<'static> = Line::from(vec!["> ".into(), "Hello world".into()]);
    line = line.style(Color::Green);
    insert_history_lines(&mut term, vec![line]).expect("Failed to insert history lines in test");

    let mut saw_colored = false;
    'outer: for row in 0..height {
        for col in 0..width {
            if let Some(cell) = term.backend().vt100().screen().cell(row, col)
                && cell.has_contents()
                && cell.fgcolor() != vt100::Color::Default
            {
                saw_colored = true;
                break 'outer;
            }
        }
    }
    assert!(saw_colored, "expected at least one colored cell in vt100 output");
}

#[test]
fn vt100_blockquote_wrap_preserves_color_on_all_wrapped_lines() {
    // 使用窄视口和长块引用强制换行。
    // Force wrapping by using a narrow viewport width and a long blockquote line.
    let width: u16 = 20;
    let height: u16 = 8;
    let backend = VT100Backend::new(width, height);
    let mut term = crate::custom_terminal::Terminal::with_options(backend).expect("terminal");
    // 视口位于最后一行，使历史直接出现在其上方。
    // Viewport is the last line so history goes directly above it.
    let viewport = Rect::new(0, height - 1, width, 1);
    term.set_viewport_area(viewport);

    // 创建带明显前缀且足以换行的长块引用。
    // Create a long blockquote with a distinct prefix and enough text to wrap.
    let mut line: Line<'static> =
        Line::from(vec!["> ".into(), "This is a long quoted line that should wrap".into()]);
    line = line.style(Color::Green);

    insert_history_lines(&mut term, vec![line]).expect("Failed to insert history lines in test");

    // 解析并检查最终屏幕缓冲区。
    // Parse and inspect the final screen buffer.
    let screen = term.backend().vt100().screen();

    // 收集非空行，它们应对应生成的换行结果。
    // Collect rows that are non-empty; these should correspond to our wrapped lines.
    let mut non_empty_rows: Vec<u16> = Vec::new();
    for row in 0..height {
        let mut any = false;
        for col in 0..width {
            if let Some(cell) = screen.cell(row, col)
                && cell.has_contents()
                && cell.contents() != "\0"
                && cell.contents() != " "
            {
                any = true;
                break;
            }
        }
        if any {
            non_empty_rows.push(row);
        }
    }

    // 换行后应至少产生两行。
    // Expect at least two rows due to wrapping.
    assert!(
        non_empty_rows.len() >= 2,
        "expected wrapped output to span >=2 rows, got {non_empty_rows:?}",
    );

    // 每个非空行的所有非空格单元都应使用非默认前景色。
    // For each non-empty row, ensure all non-space cells are using a non-default fg color.
    for row in non_empty_rows {
        for col in 0..width {
            if let Some(cell) = screen.cell(row, col) {
                let contents = cell.contents();
                if !contents.is_empty() && contents != " " {
                    assert!(
                        cell.fgcolor() != vt100::Color::Default,
                        "expected non-default fg on row {row} col {col}, got {:?}",
                        cell.fgcolor()
                    );
                }
            }
        }
    }
}

#[test]
fn vt100_colored_prefix_then_plain_text_resets_color() {
    let width: u16 = 40;
    let height: u16 = 6;
    let backend = VT100Backend::new(width, height);
    let mut term = crate::custom_terminal::Terminal::with_options(backend).expect("terminal");
    let viewport = Rect::new(0, height - 1, width, 1);
    term.set_viewport_area(viewport);

    // 首个 span 着色，其余保持普通样式。
    // First span colored, rest plain.
    let line: Line<'static> = Line::from(vec![
        Span::styled("1. ", ratatui::style::Style::default().fg(Color::LightBlue)),
        Span::raw("Hello world"),
    ]);

    insert_history_lines(&mut term, vec![line]).expect("Failed to insert history lines in test");

    let screen = term.backend().vt100().screen();

    // 找到首个非空行，验证前三个单元着色，后续单元恢复默认。
    // Find the first non-empty row; verify first three cells are colored, following cells default.
    'rows: for row in 0..height {
        let mut has_text = false;
        for col in 0..width {
            if let Some(cell) = screen.cell(row, col)
                && cell.has_contents()
                && cell.contents() != " "
            {
                has_text = true;
                break;
            }
        }
        if !has_text {
            continue;
        }

        // 期望 `1. Hello world` 从第零列开始。
        // Expect "1. Hello world" starting at col 0.
        for col in 0..3 {
            let cell = screen.cell(row, col).unwrap();
            assert!(
                cell.fgcolor() != vt100::Color::Default,
                "expected colored prefix at col {col}, got {:?}",
                cell.fgcolor()
            );
        }
        for col in 3..(3 + "Hello world".len() as u16) {
            let cell = screen.cell(row, col).unwrap();
            assert_eq!(
                cell.fgcolor(),
                vt100::Color::Default,
                "expected default color for plain text at col {col}, got {:?}",
                cell.fgcolor()
            );
        }
        break 'rows;
    }
}

#[test]
fn vt100_deep_nested_mixed_list_third_level_marker_is_colored() {
    // 构造五层 Markdown：有序、无序、有序、无序、无序。
    // Markdown with five levels (ordered → unordered → ordered → unordered → unordered).
    let md = "1. First\n   - Second level\n     1. Third level (ordered)\n        - Fourth level (bullet)\n          - Fifth level to test indent consistency\n";
    let text = render_markdown_text(md);
    let lines: Vec<Line<'static>> = text.lines.clone();

    let width: u16 = 60;
    let height: u16 = 12;
    let backend = VT100Backend::new(width, height);
    let mut term = crate::custom_terminal::Terminal::with_options(backend).expect("terminal");
    let viewport = ratatui::layout::Rect::new(0, height - 1, width, 1);
    term.set_viewport_area(viewport);

    insert_history_lines(&mut term, lines).expect("Failed to insert history lines in test");

    let screen = term.backend().vt100().screen();

    // 将屏幕行重建为字符串以定位第三级内容。
    // Reconstruct screen rows as strings to locate the 3rd level line.
    let rows: Vec<String> = screen.rows(0, width).collect();

    let needle = "1. Third level (ordered)";
    let row_idx = rows.iter().position(|r| r.contains(needle)).unwrap_or_else(|| {
        panic!("expected to find row containing {needle:?}, have rows: {rows:?}")
    });
    let col_start = rows[row_idx].find(needle).unwrap() as u16; // column where '1' starts

    // 验证第三级数字标记 `1.` 着色，而其后空格之后的正文恢复默认色。
    // Verify that the numeric marker ("1.") at the third level is colored
    // (non-default fg) and the content after the following space resets to default.
    for c in [col_start, col_start + 1] {
        let cell = screen.cell(row_idx as u16, c).unwrap();
        assert!(
            cell.fgcolor() != vt100::Color::Default,
            "expected colored 3rd-level marker at row {row_idx} col {c}, got {:?}",
            cell.fgcolor()
        );
    }
    let content_col = col_start + 3; // skip '1', '.', and the space
    if let Some(cell) = screen.cell(row_idx as u16, content_col) {
        assert_eq!(
            cell.fgcolor(),
            vt100::Color::Default,
            "expected default color for 3rd-level content at row {row_idx} col {content_col}, got {:?}",
            cell.fgcolor()
        );
    }
}

#[test]
fn vt100_prefixed_url_keeps_prefix_and_url_on_same_row() {
    let width: u16 = 48;
    let height: u16 = 8;
    let backend = VT100Backend::new(width, height);
    let mut term = crate::custom_terminal::Terminal::with_options(backend).expect("terminal");
    let viewport = Rect::new(0, height - 1, width, 1);
    term.set_viewport_area(viewport);

    let url = "http://a-long-url.com/this/that/blablablab/new.aspx/many_people_like_how";
    let line: Line<'static> = Line::from(vec!["  │ ".into(), url.into()]);

    insert_history_lines(&mut term, vec![line]).expect("insert history");

    let rows: Vec<String> = term.backend().vt100().screen().rows(0, width).collect();

    assert!(
        rows.iter().any(|r| r.contains("│ http://a-long-url.com")),
        "expected prefix and URL on same row, rows: {rows:?}"
    );
    assert!(
        !rows.iter().any(|r| r.trim_end() == "│"),
        "unexpected orphan prefix row, rows: {rows:?}"
    );
}

#[test]
fn vt100_prefixed_url_like_without_scheme_keeps_prefix_and_token_on_same_row() {
    let width: u16 = 48;
    let height: u16 = 8;
    let backend = VT100Backend::new(width, height);
    let mut term = crate::custom_terminal::Terminal::with_options(backend).expect("terminal");
    let viewport = Rect::new(0, height - 1, width, 1);
    term.set_viewport_area(viewport);

    let url_like = "example.test/api/v1/projects/alpha-team/releases/2026-02-17/builds/1234567890";
    let line: Line<'static> = Line::from(vec!["  │ ".into(), url_like.into()]);

    insert_history_lines(&mut term, vec![line]).expect("insert history");

    let rows: Vec<String> = term.backend().vt100().screen().rows(0, width).collect();

    assert!(
        rows.iter().any(|r| r.contains("│ example.test/api/v1/projects")),
        "expected prefix and URL-like token on same row, rows: {rows:?}"
    );
    assert!(
        !rows.iter().any(|r| r.trim_end() == "│"),
        "unexpected orphan prefix row, rows: {rows:?}"
    );
}

#[test]
fn vt100_user_message_url_wrap_preserves_gutter_and_background() {
    use crate::history_cell::HistoryCell;
    use crate::history_cell::UserHistoryCell;

    let width = 36;
    let height = 12;
    let backend = VT100Backend::new(width, height);
    let mut term = crate::custom_terminal::Terminal::with_options(backend).expect("terminal");
    term.set_viewport_area(Rect::new(
        /*x*/ 0,
        /*y*/ height - 1,
        /*width*/ width,
        /*height*/ 1,
    ));

    let url = "https://example.test/forwarded/threads/10930?page=1&queue=customer_support_unprocessed&forwardedScope=all";
    let cell = UserHistoryCell::new(url.to_string(), Vec::new());
    let lines = plain_hyperlink_lines(cell.display_lines(width))
        .into_iter()
        .map(|line| line.style(ratatui::style::Style::default().bg(Color::Blue)))
        .collect::<Vec<_>>();

    let screen_size = term.last_known_screen_size;
    insert_history_hyperlink_lines_with_mode_and_wrap_policy(
        &mut term,
        &lines,
        InsertHistoryMode::Standard,
        HistoryLineWrapPolicy::PreWrap,
        screen_size,
    )
    .expect("insert wrapped user message");

    let screen = term.backend().vt100().screen();
    let rows = screen.rows(/*start*/ 0, width).collect::<Vec<_>>();
    let message_rows =
        rows.iter().enumerate().filter(|(_, row)| !row.trim().is_empty()).collect::<Vec<_>>();

    assert!(message_rows.len() > 1, "expected wrapped URL: {rows:?}");
    assert!(
        message_rows[0].1.starts_with("▌ "),
        "the first v3 user-message row must retain its gutter: {rows:?}"
    );
    assert_eq!(
        message_rows
            .iter()
            .enumerate()
            .map(|(index, (_, row))| {
                if index == 0 { row.strip_prefix("▌ ").unwrap().trim() } else { row.trim() }
            })
            .collect::<String>(),
        url
    );
    for (row, row_text) in message_rows {
        assert_ne!(
            screen.cell(row as u16, /*col*/ 0).unwrap().bgcolor(),
            vt100::Color::Default,
            "wrapped user-message gutter lost its background on row {row}"
        );
        let content_col =
            row_text.trim_end().chars().count().saturating_sub(1).min(usize::from(width - 1))
                as u16;
        assert_ne!(
            screen.cell(row as u16, content_col).unwrap().bgcolor(),
            vt100::Color::Default,
            "wrapped user-message content lost its background on row {row}"
        );
    }
}

#[test]
fn vt100_prefixed_mixed_url_line_wraps_suffix_words_together() {
    let width: u16 = 24;
    let height: u16 = 10;
    let backend = VT100Backend::new(width, height);
    let mut term = crate::custom_terminal::Terminal::with_options(backend).expect("terminal");
    let viewport = Rect::new(0, height - 1, width, 1);
    term.set_viewport_area(viewport);

    let url = "https://example.test/path/abcdef12345";
    let line: Line<'static> =
        Line::from(vec!["  │ ".into(), "see ".into(), url.into(), " tail words".into()]);

    insert_history_lines(&mut term, vec![line]).expect("insert mixed history");

    let rows: Vec<String> = term.backend().vt100().screen().rows(0, width).collect();
    assert!(
        rows.iter().any(|r| r.contains("│ see")),
        "expected prefixed prose before URL, rows: {rows:?}"
    );
    assert!(
        rows.iter().any(|r| r.contains("tail words")),
        "expected suffix words to wrap as a phrase, rows: {rows:?}"
    );
}

#[test]
fn vt100_prefixed_mixed_url_line_preserves_prefix_on_wrapped_rows() {
    let width: u16 = 24;
    let height: u16 = 10;
    let backend = VT100Backend::new(width, height);
    let mut term = crate::custom_terminal::Terminal::with_options(backend).expect("terminal");
    let viewport =
        Rect::new(/*x*/ 0, /*y*/ height - 1, /*width*/ width, /*height*/ 1);
    term.set_viewport_area(viewport);

    let line: Line<'static> = Line::from(vec![
        "  ".into(),
        "see https://example.com and enough trailing prose to force another wrapped row".into(),
    ]);

    insert_history_lines(&mut term, vec![line]).expect("insert mixed history");

    let rows: Vec<String> = term.backend().vt100().screen().rows(0, width).collect();
    let continuation_row = rows
        .iter()
        .find(|row| row.contains("prose to force another"))
        .unwrap_or_else(|| panic!("expected continuation row in screen rows: {rows:?}"));

    assert!(
        continuation_row.starts_with("  "),
        "expected wrapped continuation row to keep the original prefix, rows: {rows:?}"
    );
}

#[test]
fn vt100_prefixed_non_url_line_preserves_prefix_on_wrapped_rows() {
    let width: u16 = 32;
    let height: u16 = 10;
    let backend = VT100Backend::new(width, height);
    let mut term = crate::custom_terminal::Terminal::with_options(backend).expect("terminal");
    let viewport =
        Rect::new(/*x*/ 0, /*y*/ height - 1, /*width*/ width, /*height*/ 1);
    term.set_viewport_area(viewport);

    let line = Line::from(
        "      dog while this deliberately long string tests code block scrolling versus soft wrapping",
    );

    insert_history_lines(&mut term, vec![line]).expect("insert prefixed history");

    let rows: Vec<String> = term.backend().vt100().screen().rows(0, width).collect();
    let continuation_row = rows
        .iter()
        .find(|row| row.contains("tests code block scrolling"))
        .unwrap_or_else(|| panic!("expected continuation row in screen rows: {rows:?}"));

    assert!(
        continuation_row.starts_with("      "),
        "expected wrapped continuation row to keep the original prefix, rows: {rows:?}"
    );
}

#[test]
fn vt100_terminal_wrap_policy_does_not_pre_wrap_long_paragraph() {
    let width: u16 = 20;
    let height: u16 = 8;
    let backend = VT100Backend::new(width, height);
    let mut term = crate::custom_terminal::Terminal::with_options(backend).expect("terminal");
    let viewport = Rect::new(0, height - 1, width, 1);
    term.set_viewport_area(viewport);

    let line = Line::from("alpha beta gamma delta epsilon zeta");

    insert_history_lines_with_wrap_policy(&mut term, vec![line], HistoryLineWrapPolicy::Terminal)
        .expect("insert raw history");

    let rows: Vec<String> = term.backend().vt100().screen().rows(0, width).collect();
    assert!(
        rows.iter().any(|row| row.trim_end() == "alpha beta gamma del"),
        "expected terminal soft-wrap instead of Codex word pre-wrap, rows: {rows:?}"
    );
}

#[test]
fn vt100_zellij_raw_insert_keeps_soft_wrapped_tail_above_viewport() {
    let width: u16 = 20;
    let height: u16 = 8;
    let backend = VT100Backend::new(width, height);
    let mut term = crate::custom_terminal::Terminal::with_options(backend).expect("terminal");
    let viewport =
        Rect::new(/*x*/ 0, /*y*/ height - 2, /*width*/ width, /*height*/ 2);
    term.set_viewport_area(viewport);

    let line = Line::from("raw-start-aaaaaaaaaaaaaaaaaaaaaaaa-tail-must-remain");
    insert_history_lines_with_mode_and_wrap_policy(
        &mut term,
        vec![line],
        InsertHistoryMode::FullScreen,
        HistoryLineWrapPolicy::Terminal,
    )
    .expect("insert Zellij raw history");

    let rows: Vec<String> = term.backend().vt100().screen().rows(0, width).collect();
    insta::assert_snapshot!("zellij_raw_terminal_wrap_above_viewport", rows.join("\n"));
    let history_rows = rows[..usize::from(term.viewport_area.y)]
        .iter()
        .map(|row| row.trim_end())
        .collect::<String>();
    let viewport_rows = rows[usize::from(term.viewport_area.y)..].join("\n");
    assert!(
        history_rows.contains("tail-must-remain"),
        "expected wrapped raw tail above the viewport, rows: {rows:?}"
    );
    assert!(
        !viewport_rows.contains("tail-must-remain"),
        "raw tail must not be written through the viewport, rows: {rows:?}"
    );
}

#[test]
fn vt100_zellij_raw_replay_keeps_overflowing_soft_wrapped_tail_above_viewport() {
    let width: u16 = 20;
    let height: u16 = 8;
    let backend = VT100Backend::new(width, height);
    let mut term = crate::custom_terminal::Terminal::with_options(backend).expect("terminal");
    term.set_viewport_area(Rect::new(
        /*x*/ 0, /*y*/ 0, /*width*/ width, /*height*/ 2,
    ));

    let line = Line::from(format!("raw-start-{}tail-must-remain", "a".repeat(130)));
    insert_history_lines_with_mode_and_wrap_policy(
        &mut term,
        vec![line],
        InsertHistoryMode::FullScreen,
        HistoryLineWrapPolicy::Terminal,
    )
    .expect("replay Zellij raw history");

    let rows: Vec<String> = term.backend().vt100().screen().rows(0, width).collect();
    insta::assert_snapshot!("zellij_raw_terminal_wrap_overflow_above_viewport", rows.join("\n"));
    let history_rows = rows[..usize::from(term.viewport_area.y)]
        .iter()
        .map(|row| row.trim_end())
        .collect::<String>();
    let viewport_rows = rows[usize::from(term.viewport_area.y)..].join("\n");
    assert!(
        history_rows.contains("tail-must-remain"),
        "expected overflowing raw tail above the viewport, rows: {rows:?}"
    );
    assert!(
        !viewport_rows.contains("tail-must-remain"),
        "overflowing raw tail must not be written through the viewport, rows: {rows:?}"
    );
}

#[test]
fn vt100_unwrapped_url_like_clears_continuation_rows() {
    let width: u16 = 20;
    let height: u16 = 10;
    let backend = VT100Backend::new(width, height);
    let mut term = crate::custom_terminal::Terminal::with_options(backend).expect("terminal");
    let viewport = Rect::new(0, height - 1, width, 1);
    term.set_viewport_area(viewport);

    let filler_line: Line<'static> = Line::from(vec![
        "  │ ".into(),
        "XXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX".into(),
    ]);
    insert_history_lines(&mut term, vec![filler_line]).expect("insert filler history");

    let url_like = "example.test/api/v1/short";
    let url_line: Line<'static> = Line::from(vec!["  │ ".into(), url_like.into()]);
    insert_history_lines(&mut term, vec![url_line]).expect("insert url-like history");

    let rows: Vec<String> = term.backend().vt100().screen().rows(0, width).collect();
    let first_row = rows
        .iter()
        .position(|row| row.contains("│ example.test/api"))
        .unwrap_or_else(|| panic!("expected url-like first row in screen rows: {rows:?}"));
    assert!(
        first_row + 1 < rows.len(),
        "expected a continuation row for wrapped URL-like line, rows: {rows:?}"
    );
    let continuation_row = rows[first_row + 1].trim_end();

    assert!(
        continuation_row.contains("/v1/short") || continuation_row.contains("short"),
        "expected continuation row to contain wrapped URL-like tail, got: {continuation_row:?}"
    );
    assert!(
        !continuation_row.contains('X'),
        "expected continuation row to be cleared before writing wrapped URL-like content, got: {continuation_row:?}"
    );
}

#[test]
fn vt100_long_unwrapped_url_does_not_insert_extra_blank_gap_before_content() {
    let width: u16 = 56;
    let height: u16 = 24;
    let backend = VT100Backend::new(width, height);
    let mut term = crate::custom_terminal::Terminal::with_options(backend).expect("terminal");
    let viewport = Rect::new(0, height - 1, width, 1);
    term.set_viewport_area(viewport);

    let prompt = "Write a long URL as output for testing";
    insert_history_lines(&mut term, vec![Line::from(prompt)]).expect("insert prompt line");

    let long_url = format!(
        "https://example.test/api/v1/projects/alpha-team/releases/2026-02-17/builds/1234567890/{}",
        "very-long-segment-".repeat(16),
    );
    let url_line: Line<'static> = Line::from(vec!["• ".into(), long_url.into()]);
    insert_history_lines(&mut term, vec![url_line]).expect("insert long url line");

    let rows: Vec<String> = term.backend().vt100().screen().rows(0, width).collect();
    let prompt_row = rows
        .iter()
        .position(|row| row.contains("Write a long URL as output for testing"))
        .unwrap_or_else(|| panic!("expected prompt row in screen rows: {rows:?}"));
    let url_row = rows
        .iter()
        .position(|row| row.contains("• https://example.test/api"))
        .unwrap_or_else(|| panic!("expected URL first row in screen rows: {rows:?}"));

    assert!(
        url_row <= prompt_row + 2,
        "expected URL content to appear immediately after prompt (allowing at most one spacer row), got prompt_row={prompt_row}, url_row={url_row}, rows={rows:?}",
    );
}
