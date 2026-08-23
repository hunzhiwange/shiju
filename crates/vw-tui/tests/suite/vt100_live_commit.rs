//! 验证实时行环超出容量后，只把稳定前缀提交到终端历史。
//! 未提交尾部继续留在实时环中，避免历史区与活动输出重复显示。

use crate::test_backend::VT100Backend;
use ratatui::layout::Rect;
use ratatui::text::Line;

#[test]
// 五行输入保留三行实时尾部时，最早两行应进入视口上方的终端历史。
fn live_001_commit_on_overflow() {
    let backend = VT100Backend::new(/*width*/ 20, /*height*/ 6);
    let mut term =
        codex_tui::Terminal::with_options(backend).expect("terminal should be constructed");
    let area = Rect::new(
        /*x*/ 0, /*y*/ 5, /*width*/ 20, /*height*/ 1,
    );
    term.set_viewport_area(area);

    // Build 5 explicit rows at width 20.
    // 在宽度 20 下构造五个显式逻辑行。
    let mut rb = codex_tui::RowBuilder::new(/*target_width*/ 20);
    rb.push_fragment("one\n");
    rb.push_fragment("two\n");
    rb.push_fragment("three\n");
    rb.push_fragment("four\n");
    rb.push_fragment("five\n");

    // Keep the last 3 in the live ring; commit the first 2.
    // 实时环保留最后三行，只提交最早两行。
    let commit_rows = rb.drain_commit_ready(/*max_keep*/ 3);
    let lines: Vec<Line<'static>> = commit_rows.into_iter().map(|r| r.text.into()).collect();

    codex_tui::insert_history_lines(&mut term, lines)
        .expect("Failed to insert history lines in test");

    let screen = term.backend().vt100().screen();

    // The words "one" and "two" should appear above the viewport.
    // “one”和“two”应出现在活动视口上方。
    let joined = screen.contents();
    assert!(
        joined.contains("one"),
        "expected committed 'one' to be visible\n{joined}"
    );
    assert!(
        joined.contains("two"),
        "expected committed 'two' to be visible\n{joined}"
    );
    // The last three (three,four,five) remain in the live ring, not committed here.
    // 最后三行仍留在实时环中，本用例不把它们提交到历史。
}
