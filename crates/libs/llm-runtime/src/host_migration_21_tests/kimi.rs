#![allow(unused_imports)]
use crate::acp::model_source::profile::AcpLaunchProfile;
use std::{collections::HashMap, io::Write, path::Path};
use serde_json::json;
use model_reasoning::{ReasoningFormat, reasoning_levels, reasoning_profile};
use crate::acp::model_source::drivers::kimi::*;
use agent_config::schema::AcpAgentConfig;
use model_transport::RelayIdentity;
use model_types::ApiInfo;


fn profile(model: &str) -> AcpLaunchProfile {
    let mut profile = AcpLaunchProfile::new(
        &AcpAgentConfig { command: "kimi".into(), args: vec!["acp".into()], ..Default::default() },
        RelayIdentity {
            tenant: vw_api_transport::TenantScope::default(),
            project: "project".into(),
            session: "session".into(),
            provider: "provider".into(),
            model: model.into(),
            generation: 0,
        },
    )
    .unwrap();
    profile.model_api = Some(model_types::ApiInfo {
        protocols: None,
        id: model.into(),
        url: "https://example.invalid/v1/chat/completions".into(),
        adapter: "openai-compatible".into(),
    });
    profile.model_supports_tool_call = true;
    profile
}

#[test]
fn acp_model_source_kimi_child_reads_private_route_without_argv_credentials() {
    let mut profile = profile("selected-model");
    configure(&mut profile, "http://127.0.0.1:12345").unwrap();
    let output = std::process::Command::new("python3")
        .args(["-c", "import os,json,pathlib; p=pathlib.Path(os.environ['KIMI_CODE_HOME'])/'config.toml'; s=p.read_text(); assert 'selected-model' in s; assert 'http://127.0.0.1:12345/v1' in s; assert 'api_key_env' in s; assert os.environ['VW_KIMI_RELAY_TOKEN'] not in s; print('BOUND')"])
        .envs(&profile.config().env).output().unwrap();
    assert!(output.status.success());
    assert_eq!(output.stdout, b"BOUND\n");
    assert_eq!(profile.config().args, ["acp"]);
    assert!(!profile.config().args.iter().any(|arg| arg.contains(profile.binding.bearer_token())));
    let binding = profile.binding.clone();
    let token = binding.bearer_token().to_owned();
    drop(profile);
    assert!(!binding.accepts(&token, binding.identity()));
}

#[test]
fn acp_model_source_kimi_reissue_preserves_history_without_persisting_credentials() {
    let state = tempfile::tempdir().unwrap();
    let mut first = profile("first");
    first.state_root = Some(state.path().to_owned());
    configure(&mut first, "http://127.0.0.1:12345").unwrap();
    let home = std::path::PathBuf::from(&first.config().env["KIMI_CODE_HOME"]);
    std::fs::write(home.join("history-probe"), b"history").unwrap();
    let token = first.binding.bearer_token().to_owned();
    drop(first);
    let mut next = profile("second");
    next.state_root = Some(state.path().to_owned());
    configure(&mut next, "http://127.0.0.1:12346").unwrap();
    assert_eq!(next.config().env["KIMI_CODE_HOME"], home.to_string_lossy());
    assert_eq!(std::fs::read(home.join("history-probe")).unwrap(), b"history");
    let content = std::fs::read_to_string(home.join("config.toml")).unwrap();
    assert!(content.contains("second"));
    assert!(!content.contains(&token));
    assert!(!content.contains(next.binding.bearer_token()));
    assert_ne!(next.binding.bearer_token(), token);
}

#[test]
fn acp_model_source_kimi_conflicts_and_unsupported_routes_fail_before_publication() {
    let mut profile = profile("selected");
    profile.append_arguments(&["--login".into()]).unwrap();
    assert!(configure(&mut profile, "http://127.0.0.1:12345").is_err());
    let mut profile = self::profile("selected");
    profile
        .overlay_environment(std::collections::HashMap::from([(
            "KIMI_CODE_HOME".into(),
            "/native".into(),
        )]))
        .unwrap();
    assert!(configure(&mut profile, "http://127.0.0.1:12345").is_err());
    let mut profile = self::profile("selected");
    assert!(configure(&mut profile, "https://example.invalid").is_err());
    profile.model_api = None;
    assert!(configure(&mut profile, "http://127.0.0.1:12345").is_err());
    assert_eq!(std::fs::read_dir(profile.directory()).unwrap().count(), 0);
}
