use super::{BottomPane, BottomPaneParams};
use tui_contracts::action::{V3Action, V3FileSearchMatch, V3FooterData, V3SelectionItem};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

fn key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

// Exercise the public key/idle path: ASCII input is held briefly for paste detection.
fn type_char(pane: &mut BottomPane, ch: char, query: Option<&str>, action: V3Action) {
    // Keep separate keystrokes outside the burst interval, including after selection.
    std::thread::sleep(std::time::Duration::from_millis(50));
    assert_eq!(pane.handle_key_event(key(KeyCode::Char(ch))), action);
    let deadline = pane.composer().next_paste_flush_deadline().expect("pending ASCII input");
    std::thread::sleep(deadline.saturating_duration_since(std::time::Instant::now()));
    assert!(pane.flush_paste_burst_if_due(std::time::Instant::now()));
    assert_eq!(pane.composer().active_file_search_query(), query);
    assert_eq!(pane.file_search_visible(), query.is_some());
}

#[test]
fn typing_at_requests_search_and_selecting_inserts_path() {
    let mut pane = BottomPane::new(BottomPaneParams {
        footer_data: V3FooterData::default(),
        placeholder: "Compose a ShiJu task".into(),
    });
    pane.set_command_palette_items(vec![V3SelectionItem::new("search", "Search")]);

    type_char(&mut pane, '@', Some(""), V3Action::None);
    assert!(!pane.command_palette_visible());
    type_char(&mut pane, 's', Some("s"), V3Action::None);
    type_char(&mut pane, 'r', Some("sr"), V3Action::None);

    pane.set_file_search_matches(
        "sr",
        vec![
            V3FileSearchMatch { id: "lib".into(), path: "src/lib.rs".into(), ..Default::default() },
            V3FileSearchMatch {
                id: "main".into(),
                path: "src/main.rs".into(),
                ..Default::default()
            },
        ],
    );
    assert_eq!(pane.handle_key_event(key(KeyCode::Down)), V3Action::None);
    assert_eq!(
        pane.handle_key_event(key(KeyCode::Enter)),
        V3Action::FileSearchQueryChanged { query: None }
    );
    assert_eq!(pane.draft(), "@src/main.rs ");
    assert!(!pane.file_search_visible());

    type_char(&mut pane, '@', Some(""), V3Action::None);
    assert_eq!(
        pane.handle_key_event(key(KeyCode::Esc)),
        V3Action::FileSearchQueryChanged { query: None }
    );
    assert_eq!(pane.draft(), "@src/main.rs @");
    assert!(!pane.file_search_visible());

    // The dismissed @ is still the active token until the buffered ! is flushed.
    type_char(
        &mut pane,
        '!',
        None,
        V3Action::FileSearchQueryChanged { query: Some(String::new()) },
    );
    assert_eq!(pane.draft(), "@src/main.rs @!");
}
