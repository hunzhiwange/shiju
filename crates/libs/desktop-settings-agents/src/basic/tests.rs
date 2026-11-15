use super::{shows_registration_controls, shows_delegation_controls};
#[test]
fn basic_agent_visibility_helpers_distinguish_main_from_workers() {
    let main = &crate::state::DelegateAgentSettingsEntry::from_config("main", None);
    let worker = &crate::state::DelegateAgentSettingsEntry::from_config("reviewer", None);

    assert!(!shows_registration_controls(main));
    assert!(shows_registration_controls(worker));
}

#[test]
fn builtin_agent_settings_show_chinese_description_and_fixed_role_controls() {
    let reviewer = &crate::state::DelegateAgentSettingsEntry::from_config("reviewer", None);

    assert_eq!(reviewer.label, "代码审查");
    assert_eq!(reviewer.description, "只读审查代码、测试与风险，给出有证据的缺陷报告。");
    assert!(shows_registration_controls(reviewer));
    assert!(!shows_delegation_controls(reviewer));
}
