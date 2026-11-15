use super::*;

#[test]
fn toggle_sets_overlay_visibility_directly() {
    let mut app = State::default();

    update(&mut app, SearchMessage::Toggle(true));
    assert!(app.show_search_overlay);

    update(&mut app, SearchMessage::Toggle(false));
    assert!(!app.show_search_overlay);
}
