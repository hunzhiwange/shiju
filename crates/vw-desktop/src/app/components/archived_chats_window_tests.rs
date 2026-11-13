//! Explicit native acceptance contract; native execution uses the same module.
#[test]
fn archived_chats_native_acceptance_has_explicit_isolated_entry() {
    let manifest = include_str!("../../../Cargo.toml");
    assert!(
        manifest.contains("name = \"archived-chats-window\""),
        "native archive acceptance target missing"
    );
}

use vw_shared::session::info::Info;

pub(super) fn fixture() -> Vec<Info> {
    (0..1178).map(|index| {
        let project = if index < 5 { "batch" } else if index < 8 { "duplicate" } else { "history" };
        serde_json::from_value(serde_json::json!({
            "id": format!("chat-{index:04}"), "slug": "fixture", "projectID": project,
            "directory": format!("/isolated/{project}"),
            "parentID": (index == 2).then_some("chat-0001"),
            "title": format!("归档聊天 {index:04} Rust 🦀 {}", "很长的中文标题".repeat(8)),
            "version": "1", "time": {"created": index, "updated": 1794350000000_u64 + index, "archived": 1}
        })).unwrap()
    }).collect()
}

#[test]
fn archived_chats_acceptance_fixture_preserves_large_list_and_duplicate_projects() {
    let rows = fixture();
    assert_eq!(rows.len(), 1178);
    assert_eq!(
        rows.iter().map(|row| &row.id).collect::<std::collections::HashSet<_>>().len(),
        1178
    );
    assert_eq!(
        rows.iter().map(|row| &row.project_id).collect::<std::collections::HashSet<_>>().len(),
        3
    );
    assert_eq!(rows[2].parent_id.as_deref(), Some("chat-0001"));
}
