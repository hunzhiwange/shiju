use super::*;
use crate::{config::schema::AcpAgentConfig, model_relay::RelayIdentity};

#[test]
fn qoder_unreviewed_command_cannot_write_launch_material() {
    let mut profile = AcpLaunchProfile::new(
        &AcpAgentConfig {
            command: "qodercli".into(),
            args: vec!["--acp".into()],
            ..Default::default()
        },
        RelayIdentity {
            tenant: crate::session::ui_store::current_tenant_scope(),
            project: "project".into(),
            session: "qoder-rejected".into(),
            provider: "provider".into(),
            model: "model".into(),
            generation: 0,
        },
    )
    .unwrap();
    profile.model_supports_tool_call = true;
    assert!(configure(&mut profile, "https://example.invalid").is_err());
    assert!(configure(&mut profile, "http://127.0.0.1:12345").is_err());
    assert_eq!(std::fs::read_dir(profile.directory()).unwrap().count(), 0);
}
