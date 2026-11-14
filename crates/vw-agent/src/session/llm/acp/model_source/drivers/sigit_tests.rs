use super::*;
use crate::{config::schema::AcpAgentConfig, model_relay::RelayIdentity};

fn profile(args: &[&str]) -> AcpLaunchProfile {
    let mut launch = AcpLaunchProfile::new(
        &AcpAgentConfig {
            command: "sigit".into(),
            args: args.iter().map(|s| (*s).into()).collect(),
            ..Default::default()
        },
        RelayIdentity {
            tenant: crate::session::ui_store::current_tenant_scope(),
            project: "sigit-fixture".into(),
            session: "isolated".into(),
            provider: "fixture".into(),
            model: "fixture-model".into(),
            generation: 0,
        },
    )
    .unwrap();
    launch.model_supports_tool_call = true;
    launch
}

#[test]
fn acp_onboarding_sigit_model_binding_is_private_and_conflicts_fail_closed() {
    let mut launch = profile(&["--acp"]);
    configure(&mut launch, "http://127.0.0.1:12345").unwrap();
    let env = &launch.config().env;
    assert_eq!(env["SIGIT_MODEL"], "fixture-model");
    assert_eq!(env["OPENAI_BASE_URL"], "http://127.0.0.1:12345/v1");
    assert_eq!(env["OPENAI_API_KEY"], launch.binding.bearer_token());
    assert!(!launch.config().args.iter().any(|arg| arg.contains(launch.binding.bearer_token())));
    let home = std::path::PathBuf::from(&env["SIGIT_CONFIG_DIR"]);
    assert!(home.starts_with(launch.directory()));
    assert_eq!(std::fs::read_dir(&home).unwrap().count(), 0);
    drop(launch);
    assert!(!home.exists());
    for args in [&["--acp", "--yolo"][..], &["--acp", "--config", "/tmp/other"]] {
        assert!(configure(&mut profile(args), "http://127.0.0.1:12345").is_err());
    }
    let mut launch = profile(&["--acp"]);
    assert!(configure(&mut launch, "https://example.test").is_err());
    let mut config = profile(&["--acp"]).config().clone();
    config.env.remove(crate::session::llm::acp::model_source::profile::REFERENCE_ENV);
    config.env.insert("SIGIT_MODEL".into(), "other".into());
    let identity = profile(&["--acp"]).binding.identity.clone();
    let mut launch = AcpLaunchProfile::new(&config, identity).unwrap();
    launch.model_supports_tool_call = true;
    assert!(configure(&mut launch, "http://127.0.0.1:12345").is_err());
}
