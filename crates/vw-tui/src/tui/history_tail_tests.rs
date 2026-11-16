//! 覆盖内联终端中替换可见历史尾部而不破坏既有回滚缓冲的算法。
//! 测试后端保留真实物理行与软换行语义，用于区分逻辑行和终端行。
//! 无法访问旧尾部时采取非破坏策略，宁可追加也不清除未知历史。
//! fixture 先写入视口外哨兵，再观察替换动作是否误清空上方内容。
//! 窄视口还会强制逻辑行换行，以覆盖物理终端行计数路径。

use super::replace_visible_terminal_history_tail;
use crate::custom_terminal::Terminal;
use crate::insert_history::HistoryLineWrapPolicy;
use crate::insert_history::InsertHistoryMode;
use crate::insert_history::insert_history_lines;
use crate::terminal_hyperlinks::plain_hyperlink_lines;
use crate::test_backend::VT100Backend;
use pretty_assertions::assert_eq;
use ratatui::layout::Rect;
use ratatui::text::Line;
use std::io::Write;

#[test]
// 替换当前可见尾部时，上方已有终端回滚内容必须完整保留。
fn replacing_visible_history_tail_preserves_existing_terminal_scrollback() {
    let width = 52;
    let height = 8;
    let backend = VT100Backend::with_scrollback(width, height, /*scrollback_len*/ 32);
    let mut terminal = Terminal::with_options(backend).expect("terminal");
    for line in [
        "shell history that predates Codex",
        "shell output one",
        "shell output two",
        "shell output three",
        "shell output four",
        "shell output five",
        "shell output six",
        "shell output seven",
    ] {
        writeln!(terminal.backend_mut(), "{line}\r").expect("seed shell history");
    }
    terminal.set_viewport_area(Rect::new(
        /*x*/ 0,
        /*y*/ height - 2,
        width,
        /*height*/ 2,
    ));
    insert_history_lines(
        &mut terminal,
        vec![
            Line::from("/status"),
            Line::from("Account: business"),
            Line::from("old closing border"),
        ],
    )
    .expect("insert initial status card");

    let previous_lines = plain_hyperlink_lines(vec![Line::from("old closing border")]);
    let replacement = plain_hyperlink_lines(vec![
        Line::from("Thread usage: 50 credits"),
        Line::from("new closing border"),
    ]);
    assert!(
        replace_visible_terminal_history_tail(
            &mut terminal,
            &previous_lines,
            &replacement,
            InsertHistoryMode::Standard,
            HistoryLineWrapPolicy::PreWrap,
        )
        .expect("replace status-card tail")
    );

    let contents = terminal.backend().vt100().screen().contents();
    assert!(contents.contains("/status"), "{contents}");
    assert!(contents.contains("Account: business"), "{contents}");
    assert!(contents.contains("Thread usage: 50 credits"), "{contents}");
    assert!(!contents.contains("old closing border"), "{contents}");

    let mut screen_with_scrollback = terminal.backend().vt100().screen().clone();
    screen_with_scrollback.set_scrollback(/*rows*/ usize::MAX);
    let scrollback = screen_with_scrollback.contents();
    assert!(
        scrollback.contains("shell history that predates Codex"),
        "existing terminal scrollback was discarded: {scrollback}"
    );
}

#[test]
// 旧尾部超出可读区域时只能追加新内容，不能冒险删除不可见历史。
fn inaccessible_history_tail_is_preserved_for_non_destructive_append() {
    let width = 36;
    let height = 6;
    let backend = VT100Backend::with_scrollback(width, height, /*scrollback_len*/ 16);
    let mut terminal = Terminal::with_options(backend).expect("terminal");
    terminal.set_viewport_area(Rect::new(
        /*x*/ 0,
        /*y*/ 1,
        width,
        /*height*/ height - 1,
    ));
    let before = terminal.backend().vt100().screen().contents();
    let previous_lines =
        plain_hyperlink_lines(vec![Line::from("first line"), Line::from("second line")]);
    let replacement = plain_hyperlink_lines(vec![Line::from("Thread usage: 50 credits")]);

    assert!(
        !replace_visible_terminal_history_tail(
            &mut terminal,
            &previous_lines,
            &replacement,
            InsertHistoryMode::Standard,
            HistoryLineWrapPolicy::PreWrap,
        )
        .expect("leave inaccessible history untouched")
    );
    assert_eq!(terminal.backend().vt100().screen().contents(), before);
}

#[test]
// 被终端软换行的逻辑行应按实际占用物理行数替换，避免留下残片。
fn replacing_soft_wrapped_history_counts_physical_terminal_rows() {
    let width = 12;
    let height = 8;
    let backend = VT100Backend::new(width, height);
    let mut terminal = Terminal::with_options(backend).expect("terminal");
    terminal.set_viewport_area(Rect::new(
        /*x*/ 0,
        /*y*/ height - 2,
        width,
        /*height*/ 2,
    ));
    let previous_lines = plain_hyperlink_lines(vec![Line::from("old-long-line-spanning-two-rows")]);
    let screen_size = terminal.last_known_screen_size;
    crate::insert_history::insert_history_hyperlink_lines_with_mode_and_wrap_policy(
        &mut terminal,
        &previous_lines,
        InsertHistoryMode::Standard,
        HistoryLineWrapPolicy::Terminal,
        screen_size,
    )
    .expect("insert soft-wrapped history");
    let replacement = plain_hyperlink_lines(vec![Line::from("new billing")]);

    assert!(
        replace_visible_terminal_history_tail(
            &mut terminal,
            &previous_lines,
            &replacement,
            InsertHistoryMode::Standard,
            HistoryLineWrapPolicy::Terminal,
        )
        .expect("replace physically wrapped rows")
    );
    let contents = terminal.backend().vt100().screen().contents();
    assert!(contents.contains("new billing"), "{contents}");
    assert!(!contents.contains("old-long"), "{contents}");
    assert!(!contents.contains("spanning"), "{contents}");
}
