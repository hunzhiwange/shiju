use super::*;
use crate::{config::schema::AcpAgentConfig, model_relay::RelayIdentity};

fn profile() -> AcpLaunchProfile {
    let mut profile = AcpLaunchProfile::new(
        &AcpAgentConfig {
            command: "iflow".into(),
            args: vec!["--experimental-acp".into()],
            ..Default::default()
        },
        RelayIdentity {
            tenant: crate::session::ui_store::current_tenant_scope(),
            project: "iflow-test".into(),
            session: "isolated".into(),
            provider: "fixture".into(),
            model: "acp-fixture-primary".into(),
            generation: 0,
        },
    )
    .unwrap();
    profile.model_supports_temperature = true;
    profile.model_output_limit = 4096;
    profile.model_context_limit = 32768;
    profile
}

#[test]
fn acp_model_source_iflow_child_receives_private_route_and_revokes_on_drop() {
    let mut launch = profile();
    configure(&mut launch, "http://127.0.0.1:12345").unwrap();
    let result = std::process::Command::new("python3")
        .args(["-c", "import os,sys; assert os.environ['IFLOW_baseUrl']=='http://127.0.0.1:12345/v1'; assert os.environ['IFLOW_modelName']=='acp-fixture-primary'; assert os.environ['IFLOW_selectedAuthType']=='openai-compatible'; assert os.environ['IFLOW_tokensLimit']=='32768'; assert os.environ['IFLOW_outputTokensLimit']=='4096'; assert os.environ['SHIJU_IFLOW_MODEL_LOCK']==os.environ['IFLOW_modelName']; assert os.path.isdir(os.environ['IFLOW_HOME']); assert os.environ['IFLOW_apiKey'] not in sys.argv; print('PRIVATE')"])
        .args(&launch.config().args)
        .envs(&launch.config().env)
        .output().unwrap();
    assert!(result.status.success());
    assert_eq!(result.stdout, b"PRIVATE\n");
    assert_eq!(launch.config().args, ["--experimental-acp", "--shiju-model-source"]);
    let root = launch.directory().to_owned();
    let binding = launch.binding.clone();
    let token = binding.bearer_token().to_owned();
    drop(launch);
    assert!(!root.exists());
    assert!(!binding.accepts(
        &token,
        &RelayIdentity {
            tenant: crate::session::ui_store::current_tenant_scope(),
            project: "iflow-test".into(),
            session: "isolated".into(),
            provider: "fixture".into(),
            model: "acp-fixture-primary".into(),
            generation: 0,
        }
    ));
}

#[test]
fn acp_model_source_iflow_conflicts_and_resume_fail_before_launch() {
    for args in [
        vec!["--experimental-acp", "--resume", "native"],
        vec!["--experimental-acp", "--model", "other"],
    ] {
        let mut launch = profile();
        // Arguments are added through the same validated production boundary.
        launch
            .append_arguments(&args[1..].iter().map(|s| (*s).into()).collect::<Vec<_>>())
            .unwrap();
        assert!(configure(&mut launch, "http://127.0.0.1:12345").is_err());
    }
    let mut launch = profile();
    launch
        .overlay_environment(std::collections::HashMap::from([(
            "IFLOW_apiKey".into(),
            "native-secret".into(),
        )]))
        .unwrap();
    let error = configure(&mut launch, "http://127.0.0.1:12345").unwrap_err().to_string();
    assert!(!error.contains("native-secret"));
}

#[test]
fn acp_model_source_iflow_nonlocal_endpoint_and_sampling_fail_closed() {
    let mut launch = profile();
    assert!(configure(&mut launch, "https://example.com").is_err());
    let mut launch = profile();
    launch.model_supports_temperature = false;
    assert!(configure(&mut launch, "http://127.0.0.1:12345").is_err());
}

#[test]
fn acp_model_source_iflow_history_survives_rotation_without_route_credentials() {
    let session = tempfile::tempdir().unwrap();
    let mut first = profile();
    let mut identity = first.binding.identity.clone();
    identity.session = session.path().to_string_lossy().into_owned();
    let config = AcpAgentConfig {
        command: "iflow".into(),
        args: vec!["--experimental-acp".into()],
        ..Default::default()
    };
    first = AcpLaunchProfile::new(&config, identity.clone()).unwrap();
    first.model_supports_temperature = true;
    configure(&mut first, "http://127.0.0.1:12345").unwrap();
    let history = std::path::PathBuf::from(&first.config().env["SHIJU_IFLOW_HISTORY_HOME"]);
    assert!(!history.starts_with(first.directory()));
    let anchor = history.join("rotation-test.json");
    std::fs::write(&anchor, br#"{"chatHistory":[]}"#).unwrap();
    let root = first.directory().to_owned();
    let old_token = first.binding.bearer_token().to_owned();
    drop(first);
    assert!(!root.exists());
    identity.generation += 1;
    identity.model = "acp-fixture-switched".into();
    let mut next = AcpLaunchProfile::new(&config, identity).unwrap();
    next.model_supports_temperature = true;
    configure(&mut next, "http://127.0.0.1:12346").unwrap();
    assert_eq!(next.config().env["SHIJU_IFLOW_HISTORY_HOME"], history.to_str().unwrap());
    assert_ne!(next.binding.bearer_token(), old_token);
    assert!(!std::fs::read_to_string(&anchor).unwrap().contains(&old_token));
    std::fs::remove_file(anchor).unwrap();
    drop(next);
    std::fs::remove_dir(history).unwrap();
}
