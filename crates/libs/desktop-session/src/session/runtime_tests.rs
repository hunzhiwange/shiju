use super::runtime::*;

#[test]
fn sorted_unique_tools_trims_drops_blanks_sorts_and_deduplicates() {
    let tools = vec![
        " write ".to_string(),
        String::new(),
        "read".to_string(),
        "  ".to_string(),
        "write".to_string(),
    ];

    assert_eq!(sorted_unique_tools(tools), vec!["read", "write"]);
}

#[test]
fn intersect_tools_preserves_left_order_and_duplicates() {
    let left = vec!["write".to_string(), "read".to_string(), "write".to_string()];
    let right = vec!["read".to_string(), "write".to_string()];

    assert_eq!(intersect_tools(&left, &right), left);
    assert!(intersect_tools(&left, &["shell".to_string()]).is_empty());
}

#[test]
fn request_allowed_tools_from_inventory_uses_filtered_tools() {
    let mut runtime = desktop_chat_tools::state::SessionToolSelectorState::default();
    let inventory = SessionToolInventory {
        base_tools: vec!["file_read".to_string(), "file_write".to_string()],
        restricts_tools: false,
    };

    assert_eq!(request_allowed_tools_from_inventory(&runtime, &inventory), None);

    runtime.toggle_tool(&inventory.base_tools, "file_write");

    assert_eq!(
        request_allowed_tools_from_inventory(&runtime, &inventory),
        Some(vec!["file_read".to_string()])
    );
}

#[test]
fn request_allowed_tools_from_inventory_preserves_agent_restriction() {
    let runtime = desktop_chat_tools::state::SessionToolSelectorState::default();
    let inventory = SessionToolInventory {
        base_tools: vec!["file_read".to_string()],
        restricts_tools: true,
    };

    assert_eq!(
        request_allowed_tools_from_inventory(&runtime, &inventory),
        Some(vec!["file_read".to_string()])
    );
}

#[test]
fn request_allowed_tools_from_inventory_rejects_empty_restricted_inventory() {
    let runtime = desktop_chat_tools::state::SessionToolSelectorState::default();
    let inventory = SessionToolInventory {
        base_tools: vec![String::new(), "  ".to_string()],
        restricts_tools: true,
    };

    assert_eq!(request_allowed_tools_from_inventory(&runtime, &inventory), None);
}
