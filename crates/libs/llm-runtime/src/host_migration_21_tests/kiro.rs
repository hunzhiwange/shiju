#![allow(unused_imports)]
use crate::acp::model_source::profile::AcpLaunchProfile;
use std::{collections::HashMap, io::Write, path::Path};
use serde_json::json;
use model_reasoning::{ReasoningFormat, reasoning_levels, reasoning_profile};
use crate::acp::model_source::drivers::kiro::*;
use agent_config::schema::AcpAgentConfig;
use model_transport::RelayIdentity;
use model_types::ApiInfo;


fn profile(model: &str) -> AcpLaunchProfile {
    let mut profile = AcpLaunchProfile::new(
        &AcpAgentConfig {
            command: "kiro-cli".into(),
            args: vec!["acp".into()],
            ..Default::default()
        },
        RelayIdentity {
            tenant: vw_api_transport::TenantScope::default(),
            project: "project".into(),
            session: "session:kiro".into(),
            provider: "provider".into(),
            model: model.into(),
            generation: 0,
        },
    )
    .unwrap();
    profile.model_supports_tool_call = true;
    profile.model_api = Some(model_types::ApiInfo {
        protocols: None,
        id: model.into(),
        url: "https://example.invalid/v1/chat/completions".into(),
        adapter: "openai-compatible".into(),
    });
    profile
}

#[test]
fn acp_model_source_kiro_child_reads_private_model_without_persisted_credentials() {
    let mut profile = profile("selected-model");
    configure(&mut profile, "http://127.0.0.1:12345").unwrap();
    let output = std::process::Command::new("python3")
        .args([
            "-c",
            "import os,json,pathlib; p=pathlib.Path(os.environ['KIRO_HOME'])/'settings/cli.json'; s=p.read_text(); c=json.loads(s); assert c['chat.defaultModel']=='selected-model'; assert c['acp.agentEngine']=='v3'; assert os.environ['KIRO_API_KEY'] not in s; assert os.environ['KIRO_KAS_ENDPOINT']=='http://127.0.0.1:12345'; assert os.environ['KIRO_KAS_CONTROL_PLANE_ENDPOINT']==os.environ['KIRO_KAS_ENDPOINT']; print('BOUND')",
        ])
        .envs(&profile.config().env)
        .output()
        .unwrap();
    assert!(output.status.success());
    assert_eq!(output.stdout, b"BOUND\n");
    assert!(!profile.config().args.iter().any(|arg| arg.contains(profile.binding.bearer_token())));
    let binding = profile.binding.clone();
    let token = binding.bearer_token().to_owned();
    drop(profile);
    assert!(!binding.accepts(&token, binding.identity()));
}

#[test]
fn acp_model_source_kiro_rotation_preserves_history_and_replaces_model() {
    let state = tempfile::tempdir().unwrap();
    let mut first = profile("first");
    first.state_root = Some(state.path().to_owned());
    configure(&mut first, "http://127.0.0.1:12345").unwrap();
    let home = std::path::PathBuf::from(&first.config().env["KIRO_HOME"]);
    std::fs::write(home.join("history-probe"), b"history").unwrap();
    let old_token = first.binding.bearer_token().to_owned();
    drop(first);
    let mut next = profile("second");
    next.state_root = Some(state.path().to_owned());
    configure(&mut next, "http://127.0.0.1:12346").unwrap();
    assert_eq!(next.config().env["KIRO_HOME"], home.to_string_lossy());
    assert_eq!(std::fs::read(home.join("history-probe")).unwrap(), b"history");
    let settings = std::fs::read_to_string(home.join("settings/cli.json")).unwrap();
    assert!(settings.contains("second"));
    assert!(!settings.contains(&old_token));
    assert!(!settings.contains(next.binding.bearer_token()));
    assert_ne!(old_token, next.binding.bearer_token());
}

#[test]
fn acp_model_source_kiro_conflicts_fail_before_publication() {
    for endpoint in ["https://example.invalid", "http://127.0.0.1:12345/path"] {
        let mut profile = profile("selected");
        assert!(configure(&mut profile, endpoint).is_err());
        assert_eq!(std::fs::read_dir(profile.directory()).unwrap().count(), 0);
    }
    let mut profile = profile("selected");
    profile.append_arguments(&["--model".into(), "native".into()]).unwrap();
    assert!(configure(&mut profile, "http://127.0.0.1:12345").is_err());
    let mut profile = self::profile("selected");
    profile
        .overlay_environment(std::collections::HashMap::from([(
            "KIRO_HOME".into(),
            "/native".into(),
        )]))
        .unwrap();
    assert!(configure(&mut profile, "http://127.0.0.1:12345").is_err());
    let mut profile = self::profile("selected");
    profile.model_api = None;
    assert!(configure(&mut profile, "http://127.0.0.1:12345").is_err());
    let mut profile = self::profile("selected");
    profile.model_api.as_mut().unwrap().adapter = "google".into();
    assert!(configure(&mut profile, "http://127.0.0.1:12345").is_err());
    assert_eq!(std::fs::read_dir(profile.directory()).unwrap().count(), 0);
}
