use super::*;

#[test]
fn mobile_settings_search_trims_and_matches_gateway() {
    assert!(matches_query("网关连接与认证", SystemTab::GatewayClient, " 网关 "));
    assert!(matches_query("外观与主题", SystemTab::Themes, "主题"));
    assert!(!matches_query("外观与主题", SystemTab::Themes, "网关连接"));
}

#[test]
fn mobile_settings_empty_search_keeps_all_categories() {
    for &(label, tab) in CATEGORIES {
        assert!(matches_query(label, tab, "  "));
    }
    assert_eq!(CATEGORIES[0].1, SystemTab::GatewayClient);
}

#[test]
fn mobile_settings_categories_exclude_desktop_tools() {
    assert!(!CATEGORIES.iter().any(|(_, tab)| matches!(tab,
        SystemTab::Projects | SystemTab::Models | SystemTab::Agents)));
}
