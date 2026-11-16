#![allow(unused_imports)]
use crate::acp::model_source::profile::AcpLaunchProfile;
use std::{collections::HashMap, io::Write, path::Path};
use serde_json::json;
use model_reasoning::{ReasoningFormat, reasoning_levels, reasoning_profile};
use crate::acp::model_source::drivers::goose::*;
use agent_config::schema::AcpAgentConfig;
use model_transport::RelayIdentity;
use model_types::ApiInfo;


fn profile(config: &AcpAgentConfig) -> AcpLaunchProfile {
    AcpLaunchProfile::new(
        config,
        RelayIdentity {
            tenant: vw_api_transport::TenantScope::default(),
            project: "project".into(),
            session: "session".into(),
            provider: "provider".into(),
            model: "selected-model-high".into(),
            generation: 0,
        },
    )
    .unwrap()
}

#[test]
fn acp_model_source_goose_child_environment_is_private_and_revocable() {
    let original =
        AcpAgentConfig { command: "goose".into(), args: vec!["acp".into()], ..Default::default() };
    let mut profile = profile(&original);
    configure(&mut profile, "http://127.0.0.1:12345").unwrap();
    let binding = profile.binding.clone();
    let token = binding.bearer_token().to_owned();
    let identity = binding.identity().clone();
    let directory = profile.directory().to_owned();
    let output = std::process::Command::new("python3")
        .args(["-c", "import os,sys; assert os.environ['GOOSE_PROVIDER']=='openai'; assert os.environ['GOOSE_MODEL']=='selected-model-high'; assert os.environ['SHIJU_GOOSE_MODEL_LOCK']==os.environ['GOOSE_MODEL']; assert os.environ['OPENAI_HOST']=='http://127.0.0.1:12345'; assert os.environ['OPENAI_BASE_PATH']=='v1/chat/completions'; assert os.environ['OPENAI_CUSTOM_HEADERS']==''; assert os.path.isabs(os.environ['GOOSE_PATH_ROOT']); assert os.stat(os.environ['GOOSE_PATH_ROOT']).st_mode & 0o777 == 0o700; assert os.environ['OPENAI_API_KEY']; assert sys.argv[1:]==['acp','--shiju-model-source']; print('PRIVATE_GOOSE_LAUNCH_OK')"])
        .args(&profile.config().args)
        .envs(&profile.config().env)
        .output()
        .unwrap();
    assert!(output.status.success());
    assert_eq!(String::from_utf8(output.stdout).unwrap().trim(), "PRIVATE_GOOSE_LAUNCH_OK");
    assert!(!profile.config().args.iter().any(|arg| arg.contains(&token)));
    assert_eq!(original.args, ["acp"]);
    assert!(original.env.is_empty());
    assert!(binding.accepts(&token, &identity));
    drop(profile);
    assert!(!directory.exists());
    assert!(!binding.accepts(&token, &identity));
}

#[test]
fn acp_model_source_goose_rejects_conflicting_overrides_without_secrets() {
    for key in [
        "GOOSE_PATH_ROOT",
        "GOOSE_PROVIDER",
        "GOOSE_MODEL",
        "OPENAI_HOST",
        "OPENAI_API_KEY",
        "OPENAI_CUSTOM_HEADERS",
        "SHIJU_GOOSE_MODEL_LOCK",
    ] {
        let original = AcpAgentConfig {
            command: "goose".into(),
            args: vec!["acp".into()],
            env: std::collections::HashMap::from([(
                key.to_lowercase(),
                "conflicting-secret".into(),
            )]),
        };
        let mut profile = profile(&original);
        let error = configure(&mut profile, "http://127.0.0.1:12345").unwrap_err().to_string();
        assert!(!error.contains("conflicting-secret"));
        assert!(!error.contains(profile.binding.bearer_token()));
        assert_eq!(profile.config().args, original.args);
        assert_eq!(profile.config().env.len(), original.env.len() + 1);
    }
}

#[test]
fn acp_model_source_goose_rejects_non_acp_command_and_non_loopback_relay() {
    for args in [vec![], vec!["serve".into()], vec!["acp".into(), "--shiju-model-source".into()]] {
        let mut profile =
            profile(&AcpAgentConfig { command: "goose".into(), args, ..Default::default() });
        assert!(configure(&mut profile, "http://127.0.0.1:12345").is_err());
        assert!(!profile.config().env.contains_key("GOOSE_PATH_ROOT"));
    }
    for endpoint in [
        "https://example.invalid",
        "http://localhost:12345",
        "http://127.0.0.1:12345/path",
        "http://user:secret@127.0.0.1:12345",
    ] {
        let mut profile = profile(&AcpAgentConfig {
            command: "goose".into(),
            args: vec!["acp".into()],
            ..Default::default()
        });
        assert!(configure(&mut profile, endpoint).is_err());
        assert!(!profile.config().env.contains_key("GOOSE_PATH_ROOT"));
    }
}
