use super::*;
// 验证 `slash_filter_uses_supplied_vibe_window_data` 场景的触发条件与预期结果。
#[test]
fn slash_filter_uses_supplied_vibe_window_data() {
    let items = vec![
        V3SelectionItem::new("search", "Search"),
        V3SelectionItem::new("settings", "Settings"),
    ];
    assert_eq!(filter_slash_commands(&items, "set")[0].id, "settings");
}
