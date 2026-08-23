//! 使用 VT100 后端验证历史插入后的物理屏幕内容、换行和光标恢复。
//! 这些集成场景覆盖宽字符、样式片段与终端自动换行的真实组合。

use crate::test_backend::VT100Backend;
use ratatui::layout::Rect;
use ratatui::style::Stylize;
use ratatui::text::Line;

// Small helper macro to assert a collection contains an item with a clearer
// failure message.
// 小型辅助宏为集合包含断言提供更清晰的失败信息。
macro_rules! assert_contains {
    ($collection:expr, $item:expr $(,)?) => {
        assert!(
            $collection.contains(&$item),
            "Expected {:?} to contain {:?}",
            $collection,
            $item
        );
    };
    ($collection:expr, $item:expr, $($arg:tt)+) => {
        assert!($collection.contains(&$item), $($arg)+);
    };
}

struct TestScenario {
    term: codex_tui::Terminal<VT100Backend>,
}

impl TestScenario {
    // 统一创建固定几何终端并设置活动视口，供所有插入场景复用。
    fn new(width: u16, height: u16, viewport: Rect) -> Self {
        let backend = VT100Backend::new(width, height);
        let mut term =
            codex_tui::Terminal::with_options(backend).expect("failed to construct terminal");
        term.set_viewport_area(viewport);
        Self { term }
    }

    fn run_insert(&mut self, lines: Vec<Line<'static>>) {
        codex_tui::insert_history_lines(&mut self.term, lines)
            .expect("Failed to insert history lines in test");
    }
}

#[test]
// 不需换行的两行历史应完整出现在活动视口上方。
fn basic_insertion_no_wrap() {
    // Screen of 20x6; viewport is the last row (height=1 at y=5)
    // 屏幕为 20×6，活动视口是 y=5 的最后一行。
    let area = Rect::new(
        /*x*/ 0, /*y*/ 5, /*width*/ 20, /*height*/ 1,
    );
    let mut scenario = TestScenario::new(/*width*/ 20, /*height*/ 6, area);

    let lines = vec!["first".into(), "second".into()];
    scenario.run_insert(lines);
    let rows = scenario.term.backend().vt100().screen().contents();
    assert_contains!(rows, String::from("first"));
    assert_contains!(rows, String::from("second"));
}

#[test]
// 超长无空格令牌应跨物理行折行，同时不丢失任何字符。
fn long_token_wraps() {
    let area = Rect::new(
        /*x*/ 0, /*y*/ 5, /*width*/ 20, /*height*/ 1,
    );
    let mut scenario = TestScenario::new(/*width*/ 20, /*height*/ 6, area);

    let long = "A".repeat(45); // > 2 lines at width 20
    let lines = vec![long.clone().into()];
    scenario.run_insert(lines);
    let screen = scenario.term.backend().vt100().screen();

    // Count total A's on the screen
    // 统计屏幕中的全部 A，确认换行仅改变位置而未截断内容。
    let mut count_a = 0usize;
    for row in 0..6 {
        for col in 0..20 {
            if let Some(cell) = screen.cell(row, col)
                && let Some(ch) = cell.contents().chars().next()
                && ch == 'A'
            {
                count_a += 1;
            }
        }
    }

    assert_eq!(
        count_a,
        long.len(),
        "wrapped content did not preserve all characters"
    );
}

#[test]
// Emoji 和中日韩宽字符插入后都应能从重建屏幕中找回。
fn emoji_and_cjk() {
    let area = Rect::new(
        /*x*/ 0, /*y*/ 5, /*width*/ 20, /*height*/ 1,
    );
    let mut scenario = TestScenario::new(/*width*/ 20, /*height*/ 6, area);

    let text = String::from("😀😀😀😀😀 你好世界");
    let lines = vec![text.clone().into()];
    scenario.run_insert(lines);
    let rows = scenario.term.backend().vt100().screen().contents();
    for ch in text.chars().filter(|c| !c.is_whitespace()) {
        assert!(
            rows.contains(ch),
            "missing character {ch:?} in reconstructed screen"
        );
    }
}

#[test]
// 混合 ANSI 样式片段应保留连续可见文本，样式字节不污染内容。
fn mixed_ansi_spans() {
    let area = Rect::new(
        /*x*/ 0, /*y*/ 5, /*width*/ 20, /*height*/ 1,
    );
    let mut scenario = TestScenario::new(/*width*/ 20, /*height*/ 6, area);

    let line = vec!["red".red(), "+plain".into()].into();
    scenario.run_insert(vec![line]);
    let rows = scenario.term.backend().vt100().screen().contents();
    assert_contains!(rows, String::from("red+plain"));
}

#[test]
// 历史插入完成后应恢复终端记录的光标位置，不能把光标留在回滚区。
fn cursor_restoration() {
    let area = Rect::new(
        /*x*/ 0, /*y*/ 5, /*width*/ 20, /*height*/ 1,
    );
    let mut scenario = TestScenario::new(/*width*/ 20, /*height*/ 6, area);

    let lines = vec!["x".into()];
    scenario.run_insert(lines);
    assert_eq!(scenario.term.last_known_cursor_pos, (0, 0).into());
}

#[test]
// 普通英文段落应优先在词间换行，避免把单词从中间拆开。
fn word_wrap_no_mid_word_split() {
    // Screen of 40x10; viewport is the last row
    // 屏幕为 40×10，活动视口位于最后一行。
    let area = Rect::new(
        /*x*/ 0, /*y*/ 9, /*width*/ 40, /*height*/ 1,
    );
    let mut scenario = TestScenario::new(/*width*/ 40, /*height*/ 10, area);

    let sample = "Years passed, and Willowmere thrived in peace and friendship. Mira’s herb garden flourished with both ordinary and enchanted plants, and travelers spoke of the kindness of the woman who tended them.";
    scenario.run_insert(vec![sample.into()]);
    let joined = scenario.term.backend().vt100().screen().contents();
    assert!(
        !joined.contains("bo\nth"),
        "word 'both' should not be split across lines:\n{joined}"
    );
}

#[test]
// 破折号后的文本仍应按单词边界换行，覆盖问题报告中的具体组合。
fn em_dash_and_space_word_wrap() {
    // Repro from report: ensure we break before "inside", not mid-word.
    // 复现报告场景：应在 “inside” 之前断行，而不是拆开单词。
    let area = Rect::new(
        /*x*/ 0, /*y*/ 9, /*width*/ 40, /*height*/ 1,
    );
    let mut scenario = TestScenario::new(/*width*/ 40, /*height*/ 10, area);

    let sample = "Mara found an old key on the shore. Curious, she opened a tarnished box half-buried in sand—and inside lay a single, glowing seed.";
    scenario.run_insert(vec![sample.into()]);
    let joined = scenario.term.backend().vt100().screen().contents();
    assert!(
        !joined.contains("insi\nde"),
        "word 'inside' should not be split across lines:\n{joined}"
    );
}
