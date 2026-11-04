use super::*;
use crate::{config::schema::AcpAgentConfig, model_relay::RelayIdentity};

fn profile(config: AcpAgentConfig) -> AcpLaunchProfile {
    let mut profile = AcpLaunchProfile::new(
        &config,
        RelayIdentity {
            tenant: crate::session::ui_store::current_tenant_scope(),
            project: "codebuddy-test".into(),
            session: "isolated".into(),
            provider: "fixture".into(),
            model: "acp-fixture-primary".into(),
            generation: 0,
        },
    )
    .unwrap();
    profile.model_supports_temperature = true;
    profile.model_output_limit = 4096;
    profile
}

#[test]
fn acp_model_source_codebuddy_child_routes_primary_and_auxiliary_without_argv_secrets() {
    let mut launch = profile(AcpAgentConfig {
        command: "codebuddy".into(),
        args: vec!["--acp".into()],
        ..Default::default()
    });
    configure(&mut launch, "http://127.0.0.1:12345").unwrap();
    let result = std::process::Command::new("python3")
        .args(["-c", "import os,sys; assert os.environ['CODEBUDDY_BASE_URL']=='http://127.0.0.1:12345/v1'; assert all(os.environ[k]=='acp-fixture-primary' for k in ['CODEBUDDY_MODEL','CODEBUDDY_SMALL_FAST_MODEL','CODEBUDDY_BIG_SLOW_MODEL','CODEBUDDY_CODE_SUBAGENT_MODEL']); assert os.environ['CODEBUDDY_DISABLE_CUSTOM_MODELS_FILE']=='1'; assert os.environ['CODEBUDDY_API_KEY_DISABLED']==''; assert os.environ['CODEBUDDY_CODE_MAX_OUTPUT_TOKENS']=='4096'; assert os.environ['CODEBUDDY_CONFIG_DIR']==os.environ['WORKBUDDY_CONFIG_DIR']; assert os.path.isdir(os.environ['CODEBUDDY_CONFIG_DIR']); assert os.environ['CODEBUDDY_API_KEY'] not in sys.argv; print('ISOLATED')"])
        .args(&launch.config().args)
        .envs(&launch.config().env)
        .output().unwrap();
    assert!(result.status.success(), "{}", String::from_utf8_lossy(&result.stderr));
    assert_eq!(result.stdout, b"ISOLATED\n");
    assert_eq!(launch.config().args, ["--acp", "--setting-sources", "none"]);
    let directory = launch.directory().to_owned();
    let binding = launch.binding.clone();
    let token = binding.bearer_token().to_owned();
    drop(launch);
    assert!(!directory.exists());
    assert!(!binding.accepts(
        &token,
        &RelayIdentity {
            tenant: crate::session::ui_store::current_tenant_scope(),
            project: "codebuddy-test".into(),
            session: "isolated".into(),
            provider: "fixture".into(),
            model: "acp-fixture-primary".into(),
            generation: 0
        }
    ));
}

#[test]
fn acp_model_source_codebuddy_conflicting_routes_and_resume_fail_closed() {
    for args in [
        vec!["--acp", "--model=other"],
        vec!["--acp", "--resume", "native"],
        vec!["--acp", "--settings", "native.json"],
    ] {
        let mut launch = profile(AcpAgentConfig {
            command: "codebuddy".into(),
            args: args.into_iter().map(String::from).collect(),
            ..Default::default()
        });
        assert!(configure(&mut launch, "http://127.0.0.1:12345").is_err());
    }
    let mut config = AcpAgentConfig {
        command: "codebuddy".into(),
        args: vec!["--acp".into()],
        ..Default::default()
    };
    config.env.insert("CODEBUDDY_API_KEY".into(), "native-secret".into());
    let mut launch = profile(config);
    let error = configure(&mut launch, "http://127.0.0.1:12345").unwrap_err().to_string();
    assert!(!error.contains("native-secret"));
}

#[test]
fn acp_model_source_codebuddy_rejects_unsupported_temperature_before_launch() {
    let mut launch = profile(AcpAgentConfig {
        command: "codebuddy".into(),
        args: vec!["--acp".into()],
        ..Default::default()
    });
    launch.model_supports_temperature = false;
    assert!(
        configure(&mut launch, "http://127.0.0.1:12345")
            .unwrap_err()
            .to_string()
            .contains("temperature")
    );
}
