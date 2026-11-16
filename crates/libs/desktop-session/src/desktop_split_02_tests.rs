// Tests migrated from desktop host state; production types retain their original owners.
mod behavior_0 {
use crate::session::runtime::SessionToolInventory;
#[test]
fn tool_inventory_defaults_without_restrictions() {
    let inventory = SessionToolInventory::default();
    assert!(inventory.base_tools.is_empty());
    assert!(!inventory.restricts_tools);
}
}
