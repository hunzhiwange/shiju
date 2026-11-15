use super::*;
// 验证 `command_popup_filters_vibe_window_rows` 场景的触发条件与预期结果。
#[test]
fn command_popup_filters_vibe_window_rows() {
    let mut popup = CommandPopup::new(vec![
        V3SelectionItem {
            id: "search".into(),
            label: "Search".into(),
            description: Some("Find text".into()),
            ..Default::default()
        },
        V3SelectionItem {
            id: "settings".into(),
            label: "Settings".into(),
            ..Default::default()
        },
    ]);
    popup.on_composer_text_change("/sea".into());
    assert_eq!(popup.selected_item().map(|item| item.id.as_str()), Some("search"));
}
// 验证 `command_popup_emits_selection_action_without_dispatching` 场景的触发条件与预期结果。
#[test]
fn command_popup_emits_selection_action_without_dispatching() {
    let mut popup = CommandPopup::new(vec![V3SelectionItem::new("todo", "Todo")]);
    assert_eq!(
        popup.handle_key_event(KeyEvent::from(KeyCode::Enter)),
        V3Action::Select { surface: "command_palette".into(), item_id: "todo".into() }
    );
}
