
mod behavior_0 {
use crate::acp_state::AcpSettingsState;
#[test]
fn acp_security_and_autonomy_defaults_are_populated_0() {
    let acp = AcpSettingsState::default();
    assert!(acp.catalog.is_empty());
    assert!(acp.enabled.is_empty());
    assert!(!acp.loading);
    assert!(acp.saving_agent.is_none());
    assert!(acp.copied_agent.is_none());
    assert!(acp.save_error.is_none());
    assert!(acp.status_message.is_none());


}
}
