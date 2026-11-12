use super::*;
use crate::{config::schema::AcpAgentConfig, model_relay::RelayIdentity};

#[test]
fn trae_unreviewed_or_conflicting_launch_cannot_publish_credentials() {
    let mut profile = AcpLaunchProfile::new(
        &AcpAgentConfig {
            command: "traecli".into(),
            args: vec!["acp".into(), "serve".into()],
            ..Default::default()
        },
        RelayIdentity {
            tenant: crate::session::ui_store::current_tenant_scope(),
            project: "project".into(),
            session: "trae-rejected".into(),
            provider: "provider".into(),
            model: "model".into(),
            generation: 0,
        },
    )
    .unwrap();
    profile.model_supports_tool_call = true;
    profile.model_supports_temperature = true;
    assert!(configure(&mut profile, "https://example.invalid").is_err());
    assert!(configure(&mut profile, "http://127.0.0.1:12345").is_err());
    profile
        .overlay_environment(HashMap::from([("COCO_NEXTCODE_URL".into(), "conflict".into())]))
        .unwrap();
    assert!(configure(&mut profile, "http://127.0.0.1:12345").is_err());
    assert_eq!(std::fs::read_dir(profile.directory()).unwrap().count(), 0);
}
