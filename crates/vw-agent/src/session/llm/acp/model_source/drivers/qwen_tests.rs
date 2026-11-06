use super::*;
use crate::{config::schema::AcpAgentConfig, model_relay::RelayIdentity};

fn profile() -> AcpLaunchProfile {
    AcpLaunchProfile::new(
        &AcpAgentConfig {
            command: "qwen".into(),
            args: vec![
                "--acp".into(),
                "--experimental-skills".into(),
                "--approval-mode".into(),
                "yolo".into(),
            ],
            ..Default::default()
        },
        RelayIdentity {
            tenant: crate::session::ui_store::current_tenant_scope(),
            project: "qwen-test".into(),
            session: "isolated".into(),
            provider: "fixture".into(),
            model: "acp-fixture-primary".into(),
            generation: 0,
        },
    )
    .unwrap()
}

#[test]
fn acp_model_source_qwen_private_launch_revokes_credentials() {
    let mut launch = profile();
    configure(&mut launch, "http://127.0.0.1:12345").unwrap();
    let result = std::process::Command::new("python3")
        .args(["-c", "import os,sys; assert os.path.isdir(os.environ['QWEN_HOME']); assert os.environ['OPENAI_BASE_URL']=='http://127.0.0.1:12345/v1'; assert os.environ['OPENAI_API_KEY'] not in sys.argv; print('ISOLATED')"])
        .args(&launch.config().args).envs(&launch.config().env).output().unwrap();
    assert!(result.status.success());
    assert_eq!(result.stdout, b"ISOLATED\n");
    let directory = launch.directory().to_owned();
    let binding = launch.binding.clone();
    let token = binding.bearer_token().to_owned();
    drop(launch);
    assert!(!directory.exists());
    assert!(!binding.accepts(&token, &binding.identity));
}

#[test]
fn acp_model_source_qwen_conflicts_fail_before_config_publication() {
    for key in ["OPENAI_API_KEY", "qwen_home", "NODE_OPTIONS", "SHIJU_QWEN_MANAGED"] {
        let mut launch = profile();
        // A caller-controlled credential or configuration cannot take precedence.
        let mut config = launch.config().clone();
        config.env.remove(super::super::super::profile::REFERENCE_ENV);
        config.env.insert(key.into(), "conflict".into());
        launch = AcpLaunchProfile::new(&config, launch.binding.identity.clone()).unwrap();
        assert!(configure(&mut launch, "http://127.0.0.1:12345").is_err());
        assert!(!launch.directory().join("settings.json").exists());
    }
    for endpoint in
        ["https://example.com", "http://127.0.0.1:12345/v1", "http://user@127.0.0.1:12345"]
    {
        let mut launch = profile();
        assert!(configure(&mut launch, endpoint).is_err());
        assert!(!launch.directory().join("settings.json").exists());
    }
}

#[test]
fn acp_model_source_qwen_native_resume_arguments_are_rejected() {
    let launch = profile();
    let mut config = launch.config().clone();
    config.env.remove(super::super::super::profile::REFERENCE_ENV);
    config.args.extend(["--resume".into(), "native-session".into()]);
    let mut launch = AcpLaunchProfile::new(&config, launch.binding.identity.clone()).unwrap();
    assert!(configure(&mut launch, "http://127.0.0.1:12345").is_err());
    assert!(!launch.directory().join("settings.json").exists());
}

#[test]
fn acp_full_access_launch_qwen_managed_accepts_permission_neutral_registry_entry() {
    for args in
        [vec!["--acp".to_owned()], vec!["--acp".to_owned(), "--experimental-skills".to_owned()]]
    {
        let previous = profile();
        let mut config = previous.config().clone();
        config.env.remove(super::super::super::profile::REFERENCE_ENV);
        config.args = args.clone();
        let mut launch = AcpLaunchProfile::new(&config, previous.binding.identity.clone()).unwrap();
        configure(&mut launch, "http://127.0.0.1:12345").unwrap();
        assert_eq!(&launch.config().args[..args.len()], &args);
        assert!(!launch.config().args.iter().any(|arg| arg == "yolo"));
        assert!(launch.config().args.iter().any(|arg| arg == "--shiju-model-source"));
    }
}
