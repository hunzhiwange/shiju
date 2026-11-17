use super::*;
use crate::app::components::{system_settings::SystemTab, system_settings_search::SystemSettingsSearchTarget};

#[test]
fn third_party_task_07_scope_and_navigation() {
    for &(page, _) in ThirdPartyPage::ALL {
        assert_eq!(SystemSettingsSearchTarget::ThirdParty(page).page(), SystemTab::Apps);
    }
    assert_eq!(SystemSettingsSearchTarget::Agents(crate::app::state::AgentConfigTab::Mcp).page(), SystemTab::Agents);
}
