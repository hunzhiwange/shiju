//! Contract checks; external protocol acceptance must also be run before release.
#[test]
fn acp_onboarding_aionrs_registry_contract() {
    let spec = vw_acp::resolve_agent_spec("aionrs").unwrap();
    assert_eq!(spec.display_name, "aionrs");
    assert!(std::path::Path::new(&spec.command).file_stem().unwrap() == "aionrs-acp");
    assert!(spec.args.is_empty());
    for platform in [config_acp_install::AcpInstallPlatform::Unix, config_acp_install::AcpInstallPlatform::Windows] {
        assert!(config_acp_install::acp_install_command_for_platform("aionrs", platform).is_none());
    }
}
