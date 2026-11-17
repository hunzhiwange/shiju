use super::{GROUPS, SystemTab, ThirdPartyPage};

#[test]
fn third_party_task_07_scope_and_navigation() {
    assert!(!GROUPS.iter().any(|(_, tabs)| tabs.contains(&SystemTab::Apps)));
    assert_eq!(SystemTab::Apps.label(), "第三方智能体");
    for &(page, _) in ThirdPartyPage::ALL {
        assert_eq!(ThirdPartyPage::ALL.iter().filter(|(item, _)| *item == page).count(), 1);
    }
}
