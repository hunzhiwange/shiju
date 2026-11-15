use super::*;
// 验证 `stale_file_search_results_are_ignored` 场景的触发条件与预期结果。
#[test]
fn stale_file_search_results_are_ignored() {
    let mut popup = FileSearchPopup::new();
    popup.set_query("src");
    popup.set_matches(
        "old",
        vec![V3FileSearchMatch {
            id: "old".into(),
            path: "old.rs".into(),
            ..Default::default()
        }],
    );
    assert!(popup.selected_match().is_none());
}
// 验证 `file_search_selection_emits_only_an_action` 场景的触发条件与预期结果。
#[test]
fn file_search_selection_emits_only_an_action() {
    let mut popup = FileSearchPopup::new();
    popup.set_query("src");
    popup.set_matches(
        "src",
        vec![V3FileSearchMatch {
            id: "main".into(),
            path: "src/main.rs".into(),
            ..Default::default()
        }],
    );
    assert_eq!(
        popup.handle_key_event(KeyEvent::from(KeyCode::Enter)),
        V3Action::Select { surface: "file_search".into(), item_id: "main".into() }
    );
}
