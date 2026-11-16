use super::{SystemTab, system_tab_matches_query};

#[test]
fn archived_chats_browse_navigation_is_searchable() {
    let tab =
        SystemTab::all().find(|tab| tab.to_string() == "已归档的聊天").expect("archive navigation");
    assert!(system_tab_matches_query(tab, "archived"));
    assert!(system_tab_matches_query(tab, "归档"));
}
