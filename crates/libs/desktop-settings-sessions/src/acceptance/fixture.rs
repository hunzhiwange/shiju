use session_info::Info;
pub fn fixture() -> Vec<Info> {
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
