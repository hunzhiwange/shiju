use super::{AcpLaunchProfile, configure};
use agent_config::schema::AcpAgentConfig;
use model_transport::RelayIdentity;

#[test]
fn kimi_session_model_writes_integer_limits() {
    let mut profile = AcpLaunchProfile::new(
        &AcpAgentConfig { command: "kimi".into(), args: vec!["acp".into()], ..Default::default() },
        RelayIdentity {
            tenant: Default::default(),
            project: "project".into(),
            session: "kimi-config".into(),
            provider: "provider".into(),
            model: "selected-model".into(),
            generation: 0,
        },
    )
    .unwrap();
    profile.model_alias = "selected-model".into();
    profile.model_api = Some(model_types::ApiInfo {
        id: "model".into(),
        url: "https://example.invalid/v1/chat/completions".into(),
        adapter: "openai-compatible".into(),
        protocols: None,
    });
    profile.model_context_limit = 1_000_000;
    profile.model_output_limit = 384_000;

    configure(&mut profile, "http://127.0.0.1:12345").unwrap();

    let home = std::path::Path::new(&profile.config().env["KIMI_CODE_HOME"]);
    let settings =
        std::fs::read_to_string(home.join("config.toml")).unwrap().parse::<toml::Table>().unwrap();
    let model = &settings["models"]["session-selected"];
    assert_eq!(model["max_context_size"].as_integer(), Some(1_000_000));
    assert_eq!(model["max_output_size"].as_integer(), Some(384_000));
}
