//! 覆盖不同终端与复用器组合下的历史插入和视口扩展策略。
//! Windows Terminal 与 Zellij 的差异必须显式分流，不能依赖通用 DEC 行为猜测。
//! VT100 后端验证策略执行后真实屏幕和回滚缓冲均保持预期内容。
//! 底部两行作为活动视口，模拟输入区增长触发的向上滚动。

use super::ScrollbackStrategy;
use crate::custom_terminal::Terminal;
use crate::insert_history::HistoryLineWrapPolicy;
use crate::insert_history::InsertHistoryMode;
use crate::insert_history::insert_history_lines_with_mode_and_wrap_policy;
use crate::test_backend::VT100Backend;
use codex_terminal_detection::Multiplexer;
use codex_terminal_detection::TerminalInfo;
use codex_terminal_detection::TerminalName;
use crossterm::cursor::MoveTo;
use crossterm::queue;
use crossterm::style::Print;
use pretty_assertions::assert_eq;
use ratatui::layout::Rect;
use ratatui::layout::Size;
use ratatui::text::Line;

#[test]
// Windows Terminal 默认使用全屏插入，但 Zellij 接管时应采用复用器安全路径。
fn windows_terminal_uses_full_screen_unless_zellij_is_active() {
    let mut terminal = TerminalInfo {
        name: TerminalName::WindowsTerminal,
        term_program: None,
        version: None,
        term: None,
        multiplexer: None,
    };
    let mut strategy = ScrollbackStrategy::detect(&terminal);

    assert_eq!(
        [
            strategy.history_insertion_mode(HistoryLineWrapPolicy::PreWrap),
            strategy.history_insertion_mode(HistoryLineWrapPolicy::Terminal),
        ],
        [InsertHistoryMode::FullScreen, InsertHistoryMode::FullScreen]
    );

    terminal.multiplexer = Some(Multiplexer::Zellij { version: None });
    strategy = ScrollbackStrategy::detect(&terminal);
    assert_eq!(strategy, ScrollbackStrategy::Zellij);
}

#[test]
// Zellij 仅在历史已由终端换行时需要全屏插入，应用换行不应重复处理。
fn zellij_only_uses_full_screen_insertion_for_terminal_wrapped_history() {
    assert_eq!(
        [
            ScrollbackStrategy::Zellij.history_insertion_mode(HistoryLineWrapPolicy::PreWrap),
            ScrollbackStrategy::Zellij.history_insertion_mode(HistoryLineWrapPolicy::Terminal),
        ],
        [InsertHistoryMode::Standard, InsertHistoryMode::FullScreen]
    );
}

#[test]
// 全屏插入历史后，既有视口上方内容仍应留在终端回滚缓冲中。
fn full_screen_history_insertion_preserves_terminal_scrollback() {
    let width = 24;
    let height = 6;
    let backend = VT100Backend::with_scrollback(width, height, /*scrollback_len*/ 32);
    let mut terminal = Terminal::with_options(backend).expect("terminal with scrollback");
    terminal.set_viewport_area(Rect::new(
        /*x*/ 0,
        /*y*/ height - 2,
        width,
        /*height*/ 2,
    ));

    for (row, line) in [
        "oldest-history-row",
        "history-row-2",
        "history-row-3",
        "history-row-4",
        "stale-composer-1",
        "stale-composer-2",
    ]
    .into_iter()
    .enumerate()
    {
        queue!(
            terminal.backend_mut(),
            MoveTo(/*x*/ 0, row as u16),
            Print(line)
        )
        .expect("seed terminal row");
    }

    insert_history_lines_with_mode_and_wrap_policy(
        &mut terminal,
        vec![Line::from("new-history-row")],
        ScrollbackStrategy::FullScreen.history_insertion_mode(HistoryLineWrapPolicy::PreWrap),
        HistoryLineWrapPolicy::PreWrap,
    )
    .expect("insert history through the full screen");

    let visible = terminal.backend().vt100().screen().contents();
    let mut scrollback_screen = terminal.backend().vt100().screen().clone();
    scrollback_screen.set_scrollback(/*rows*/ usize::MAX);
    let scrollback = scrollback_screen.contents();

    insta::assert_snapshot!(format!("SCROLLBACK:\n{scrollback}\nVISIBLE:\n{visible}"), @r"
    SCROLLBACK:
    oldest-history-row
    history-row-2
    history-row-3
    history-row-4
    new-history-row
    VISIBLE:
    history-row-2
    history-row-3
    history-row-4
    new-history-row
    ");
}

#[test]
// 视口增高并滚动内容时，旧历史与当前输入区的相对位置都不能丢失。
fn full_screen_viewport_growth_preserves_terminal_scrollback() {
    let width = 24;
    let height = 6;
    let backend = VT100Backend::with_scrollback(width, height, /*scrollback_len*/ 32);
    let mut terminal = Terminal::with_options(backend).expect("terminal with scrollback");
    terminal.set_viewport_area(Rect::new(
        /*x*/ 0,
        /*y*/ height - 2,
        width,
        /*height*/ 2,
    ));

    for (row, line) in [
        "oldest-history-row",
        "history-row-2",
        "history-row-3",
        "history-row-4",
        "stale-composer-1",
        "stale-composer-2",
    ]
    .into_iter()
    .enumerate()
    {
        queue!(
            terminal.backend_mut(),
            MoveTo(/*x*/ 0, row as u16),
            Print(line)
        )
        .expect("seed terminal row");
    }

    ScrollbackStrategy::FullScreen
        .grow_viewport(
            &mut terminal,
            /*viewport_top*/ height - 2,
            Size::new(width, height),
            /*scroll_by*/ 2,
        )
        .expect("grow viewport through full-screen scrolling");

    let visible = terminal.backend().vt100().screen().contents();
    let mut scrollback_screen = terminal.backend().vt100().screen().clone();
    scrollback_screen.set_scrollback(/*rows*/ usize::MAX);
    let scrollback = scrollback_screen.contents();

    insta::assert_snapshot!(format!("SCROLLBACK:\n{scrollback}\nVISIBLE:\n{visible}"), @r"
    SCROLLBACK:
    oldest-history-row
    history-row-2
    history-row-3
    history-row-4
    VISIBLE:
    history-row-3
    history-row-4
    ");
}
