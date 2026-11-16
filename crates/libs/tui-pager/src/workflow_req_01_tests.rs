use super::*;
use crossterm::event::KeyModifiers;

fn key(pager: &mut TranscriptPager, code: KeyCode) {
    pager.handle_key_event(KeyEvent::new(code, KeyModifiers::NONE));
}
fn search(query: &str) -> search::Search {
    let mut search = search::Search::default();
    for ch in query.chars() {
        search.push(ch);
    }
    search
}

#[test]
fn workflow_req_01_unicode_query_matches_across_spans() {
    let lines = vec![Line::from(vec!["中文".into(), "查询".into()])];
    let mut search = search("文查");
    search.scan(&lines, 100);
    assert_eq!(search.hits, vec![(0, 3)]);
}

#[test]
fn workflow_req_01_literal_query_does_not_interpret_patterns() {
    let mut search = search(".*");
    search.scan(&[Line::from("abc .* xyz")], 100);
    assert_eq!(search.hits, vec![(0, 4)]);
}

#[test]
fn workflow_req_01_scan_respects_every_budget() {
    let lines = vec![Line::from("文".repeat(20_000))];
    let mut search = search("文文");
    for budget in [1, 2, 3, 10, 128] {
        assert_eq!(search.scan(&lines, budget), budget);
    }
    assert_eq!(search.line, 0);
}

#[test]
fn workflow_req_01_empty_lines_consume_budget() {
    let lines = vec![Line::default(); 100];
    let mut search = search("x");
    assert_eq!(search.scan(&lines, 7), 7);
    assert_eq!(search.line, 7);
}

#[test]
fn workflow_req_01_next_previous_wrap_over_matches() {
    let mut search = search("a");
    search.scan(&[Line::from("a a")], 100);
    search.navigate(true);
    assert_eq!(search.selected, 1);
    search.navigate(false);
    assert_eq!(search.selected, 0);
}

#[test]
fn workflow_req_01_escape_restores_position_without_closing() {
    let mut pager = TranscriptPager::new(vec![Line::from("line"); 30]);
    pager.scroll_to_top();
    pager.scroll_lines(5);
    key(&mut pager, KeyCode::Char('/'));
    key(&mut pager, KeyCode::Char('文'));
    key(&mut pager, KeyCode::Esc);
    assert_eq!(pager.scroll_offset, 5);
    assert!(!pager.is_done());
    assert!(pager.search.is_none());
}

#[test]
fn workflow_req_01_no_match_is_reported_with_loaded_scope() {
    let mut pager = TranscriptPager::new(vec![Line::from("hello")]);
    key(&mut pager, KeyCode::Char('/'));
    key(&mut pager, KeyCode::Char('文'));
    let area = Rect::new(0, 0, 120, 8);
    let mut buffer = Buffer::empty(area);
    pager.render(area, &mut buffer);
    let text: String = buffer.content.iter().map(|cell| cell.symbol()).collect();
    assert!(text.contains("no match"));
    assert!(text.contains("loaded only"));
}

#[test]
fn workflow_req_01_stream_append_retains_scanned_prefix() {
    let mut search = search("a");
    search.scan(&[Line::from("a"), Line::from("b")], 100);
    search.invalidate_from(2);
    search.scan(&[Line::from("a"), Line::from("b"), Line::from("a")], 100);
    assert_eq!(search.hits, vec![(0, 0), (2, 0)]);
}
