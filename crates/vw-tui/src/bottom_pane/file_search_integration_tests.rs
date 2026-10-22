use super::{BottomPane, BottomPaneParams};
use crate::vibe::action::{V3Action, V3FileSearchMatch, V3FooterData, V3SelectionItem};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

fn key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

#[test]
fn typing_at_requests_search_and_selecting_inserts_path() {
    let mut pane = BottomPane::new(BottomPaneParams {
        footer_data: V3FooterData::default(),
        placeholder: "Compose a ShiJu task".into(),
    });
    pane.set_command_palette_items(vec![V3SelectionItem::new("search", "Search")]);

    assert_eq!(
        pane.handle_key_event(key(KeyCode::Char('@'))),
        V3Action::FileSearchQueryChanged { query: Some(String::new()) }
    );
    assert!(pane.file_search_visible());
    assert!(!pane.command_palette_visible());
    assert_eq!(
        pane.handle_key_event(key(KeyCode::Char('s'))),
        V3Action::FileSearchQueryChanged { query: Some("s".into()) }
    );
    assert_eq!(
        pane.handle_key_event(key(KeyCode::Char('r'))),
        V3Action::FileSearchQueryChanged { query: Some("sr".into()) }
    );

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

    assert_eq!(
        pane.handle_key_event(key(KeyCode::Char('@'))),
        V3Action::FileSearchQueryChanged { query: Some(String::new()) }
    );
    assert_eq!(
        pane.handle_key_event(key(KeyCode::Esc)),
        V3Action::FileSearchQueryChanged { query: None }
    );
    assert_eq!(pane.draft(), "@src/main.rs @");
    assert!(!pane.file_search_visible());

    assert_eq!(pane.handle_key_event(key(KeyCode::Char('!'))), V3Action::None);
    assert_eq!(pane.draft(), "@src/main.rs @!");
}
