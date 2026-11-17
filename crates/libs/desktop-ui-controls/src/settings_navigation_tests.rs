use super::{GROUPS, SystemTab, system_tab_keywords};

#[test]
fn third_party_agents_groups_all_app_integration_entries_once() {
    let (_, tabs) = GROUPS.iter().find(|(label, _)| *label == "第三方智能体").unwrap();
    let expected = [
        SystemTab::Apps,
        SystemTab::Providers,
        #[cfg(not(target_arch = "wasm32"))]
        SystemTab::Auth,
    ];
    assert_eq!(*tabs, expected.as_slice());
    for tab in expected {
        assert_eq!(SystemTab::all().filter(|candidate| *candidate == tab).count(), 1);
        assert!(system_tab_keywords(tab).contains(&"第三方智能体"));
        assert!(system_tab_keywords(tab).contains(&"cc-switch"));
    }
    for tab in [SystemTab::Models, SystemTab::Agents, SystemTab::Memory] {
        assert!(!tabs.contains(&tab));
    }
}
