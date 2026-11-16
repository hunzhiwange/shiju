//! Authored for the deferred regression pass; not executed during implementation.
use crate::app::native_copy::wrap;
use crate::commands::{CommandRoute, route_slash_submission};
use ratatui::text::Line;

#[test]
fn workflow_req_06_raw_routes_to_native_selection() {
    assert_eq!(route_slash_submission("/raw").unwrap(), Some(CommandRoute::NativeCopy));
}

#[test]
fn workflow_req_06_raw_rejects_arguments() {
    assert!(route_slash_submission("/raw unexpected").is_err());
}

#[test]
fn workflow_req_06_chinese_wraps_at_cell_boundaries() {
    assert_eq!(wrap(&[Line::from("中文记录")], 4), vec!["中文", "记录"]);
}

#[test]
fn workflow_req_06_combining_marks_stay_with_base() {
    assert_eq!(wrap(&[Line::from("ae\u{301}b")], 2), vec!["ae\u{301}", "b"]);
}

#[test]
fn workflow_req_06_terminal_escape_is_not_written() {
    let rows = wrap(&[Line::from("\u{1b}[2J\r秘密")], 80);
    assert!(rows.iter().all(|row| !row.chars().any(char::is_control)));
    assert!(rows.concat().contains("秘密"));
}

#[test]
fn workflow_req_06_empty_snapshot_and_zero_width_are_bounded() {
    assert!(wrap(&[], 0).is_empty());
    assert_eq!(wrap(&[Line::from("ab")], 0), vec!["a", "b"]);
}
