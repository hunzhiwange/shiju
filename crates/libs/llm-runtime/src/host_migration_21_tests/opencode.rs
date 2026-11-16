#![allow(unused_imports)]
use crate::acp::model_source::profile::AcpLaunchProfile;
use std::{collections::HashMap, io::Write, path::Path};
use serde_json::json;
use model_reasoning::{ReasoningFormat, reasoning_levels, reasoning_profile};
use crate::acp::model_source::drivers::opencode::*;
use agent_config::schema::AcpAgentConfig;
use model_transport::RelayIdentity;
use model_types::ApiInfo;


fn profile() -> AcpLaunchProfile {
    let mut profile = AcpLaunchProfile::new(
        &AcpAgentConfig {
            command: "opencode".into(),
            args: vec!["acp".into()],
            ..Default::default()
        },
        RelayIdentity {
            tenant: vw_api_transport::TenantScope::default(),
            project: "project".into(),
            session: "session".into(),
            provider: "provider".into(),
            model: "selected-model".into(),
            generation: 0,
        },
    )
    .unwrap();
    profile.model_api = Some(model_types::ApiInfo {
        protocols: None,
        id: "selected-model".into(),
        url: "https://example.invalid/v1/chat/completions".into(),
        adapter: "openai-compatible".into(),
    });
    profile.model_supports_tool_call = true;
    profile
}

#[test]
fn acp_model_source_opencode_variants_follow_selected_model() {
    use model_reasoning::ReasoningOption;

    for (api_id, adapter, reasoning, declared, expected) in [
        ("gpt-5.1", "openai-compatible", true, Some(vec!["low", "high", "unknown"]), true),
        ("gpt-5.1", "openai-compatible", true, None, false),
        ("gpt-5.1", "openai-compatible", true, Some(vec![]), false),
        ("gpt-5.1", "openai-compatible", false, Some(vec!["low", "high"]), false),
        ("deepseek-v4-pro", "openai-compatible", true, Some(vec!["low", "high"]), false),
        ("gpt-5.1", "openai-responses", true, Some(vec!["low", "high"]), false),
    ] {
        let mut profile = profile();
        let api = profile.model_api.as_mut().unwrap();
        api.id = api_id.into();
        api.adapter = adapter.into();
        profile.model_supports_reasoning = reasoning;
        profile.model_reasoning_options = declared.map(|values| {
            vec![ReasoningOption::Effort {
                values: values.into_iter().map(str::to_owned).collect(),
            }]
        });
        configure(&mut profile, "http://127.0.0.1:12345").unwrap();
        let settings: serde_json::Value =
            serde_json::from_str(&profile.config().env["OPENCODE_CONFIG_CONTENT"]).unwrap();
        let model = &settings["provider"]["session-provider"]["models"]["selected-model"];
        assert_eq!(model["id"], "selected-model");
        assert_eq!(model["reasoning"], reasoning);
        if adapter == "openai-compatible" {
            assert_eq!(model["reasoning_options"], json!([]));
        } else {
            assert!(model.get("reasoning_options").is_none());
        }
        if expected {
            assert_eq!(
                model["variants"],
                json!({
                    "default": {},
                    "low": {"reasoningEffort": "low"},
                    "high": {"reasoningEffort": "high"}
                })
            );
        } else {
            assert!(model.get("variants").is_none());
        }
    }
}

#[test]
fn acp_model_source_opencode_child_reads_isolated_primary_and_auxiliary_routes() {
    let mut profile = profile();
    configure(&mut profile, "http://127.0.0.1:12345").unwrap();
    let output = std::process::Command::new("python3").args(["-c", r#"
import os,json,pathlib
c=json.loads(os.environ['OPENCODE_CONFIG_CONTENT'])
assert c['model']==c['small_model']=='session-provider/selected-model'
assert c['enabled_providers']==['session-provider']
assert c['provider']['session-provider']['whitelist']==['selected-model']
assert set(c['agent'])=={'build','plan','general','explore','title','summary','compaction'}
assert all(agent['model']==c['model'] for agent in c['agent'].values())
assert c['provider']['session-provider']['options']['baseURL']=='http://127.0.0.1:12345/v1'
assert c['provider']['session-provider']['options']['apiKey']=='{env:VW_OPENCODE_RELAY_TOKEN}'
for key in ['XDG_CONFIG_HOME','XDG_DATA_HOME','XDG_STATE_HOME','XDG_CACHE_HOME','OPENCODE_CONFIG_DIR','OPENCODE_TEST_HOME']:
    assert pathlib.Path(os.environ[key]).is_dir()
assert os.environ['OPENCODE_DISABLE_PROJECT_CONFIG']=='1'
assert os.environ['VW_OPENCODE_RELAY_TOKEN'] not in os.environ['OPENCODE_CONFIG_CONTENT']
print('ISOLATED')
"#]).envs(&profile.config().env).output().unwrap();
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    assert_eq!(output.stdout, b"ISOLATED\n");
    assert_eq!(profile.config().args, ["acp"]);
    let binding = profile.binding.clone();
    let token = binding.bearer_token().to_owned();
    drop(profile);
    assert!(!binding.accepts(&token, binding.identity()));
}

#[test]
fn acp_model_source_opencode_conflicts_fail_before_creating_runtime_state() {
    for key in ["OPENCODE_CONFIG", "OPENCODE_CONFIG_CONTENT", "XDG_DATA_HOME", "https_proxy"] {
        let mut profile = profile();
        profile
            .overlay_environment(std::collections::HashMap::from([(key.into(), "native".into())]))
            .unwrap();
        assert!(configure(&mut profile, "http://127.0.0.1:12345").is_err());
        assert_eq!(std::fs::read_dir(profile.directory()).unwrap().count(), 0);
    }
    let mut profile = profile();
    profile.append_arguments(&["--model=local/account".into()]).unwrap();
    assert!(configure(&mut profile, "http://127.0.0.1:12345").is_err());
    assert_eq!(std::fs::read_dir(profile.directory()).unwrap().count(), 0);
}

#[test]
fn acp_model_source_opencode_missing_transport_and_unowned_endpoints_fail_closed() {
    for endpoint in [
        "https://example.invalid",
        "http://127.0.0.1:12345/path",
        "http://user:secret@127.0.0.1:12345",
    ] {
        let mut profile = profile();
        assert!(configure(&mut profile, endpoint).is_err());
        assert_eq!(std::fs::read_dir(profile.directory()).unwrap().count(), 0);
    }
    let mut profile = profile();
    profile.model_api = None;
    assert!(configure(&mut profile, "http://127.0.0.1:12345").is_err());
    assert_eq!(std::fs::read_dir(profile.directory()).unwrap().count(), 0);
}

#[test]
fn acp_model_source_opencode_rotation_keeps_history_and_revokes_old_credentials() {
    let state = tempfile::tempdir().unwrap();
    let mut first = profile();
    first.state_root = Some(state.path().to_owned());
    configure(&mut first, "http://127.0.0.1:12345").unwrap();
    let data = first.config().env["XDG_DATA_HOME"].clone();
    std::fs::write(std::path::Path::new(&data).join("history-probe"), b"history").unwrap();
    let private_config = first.config().env["OPENCODE_CONFIG_DIR"].clone();
    let binding = first.binding.clone();
    let token = binding.bearer_token().to_owned();
    drop(first);
    assert!(!binding.accepts(&token, binding.identity()));
    let mut next = profile();
    next.state_root = Some(state.path().to_owned());
    configure(&mut next, "http://127.0.0.1:12346").unwrap();
    assert_eq!(next.config().env["XDG_DATA_HOME"], data);
    assert_ne!(next.config().env["OPENCODE_CONFIG_DIR"], private_config);
    assert!(!std::path::Path::new(&private_config).exists());
    assert_eq!(
        std::fs::read(std::path::Path::new(&data).join("history-probe")).unwrap(),
        b"history"
    );
    assert_ne!(next.binding.bearer_token(), token);
}

#[test]
fn acp_model_source_opencode_model_references_do_not_expand_native_configuration() {
    for model in ["{env:VW_OPENCODE_RELAY_TOKEN}", "{file:account.json}"] {
        let mut profile = profile();
        profile.model_alias = model.into();
        assert!(configure(&mut profile, "http://127.0.0.1:12345").is_err());
        assert_eq!(std::fs::read_dir(profile.directory()).unwrap().count(), 0);
    }
}

#[test]
fn acp_model_source_opencode_managed_configuration_is_rejected_without_loading_it() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("managed.json");
    assert!(reject_existing_configuration(std::slice::from_ref(&path)).is_ok());
    std::fs::write(&path, b"not-json private-policy-content").unwrap();
    let error = reject_existing_configuration(&[path]).unwrap_err().to_string();
    assert!(error.contains("system-managed configuration"));
    assert!(!error.contains("private-policy-content"));
}

#[test]
fn model_protocol_023_opencode_factory_uses_declared_chat_without_mutating_sdk() {
    let mut profile = profile();
    let api = profile.model_api.as_mut().unwrap();
    api.adapter = "cohere".into();
    api.protocols=Some(vec![serde_json::from_value(json!({
        "protocol":"chat","url":"https://example.invalid/v1/chat/completions","deployment":"reviewed",
        "authentication":{"kind":"bearer"},"models":{"kind":"models","ids":["selected-model"]},
        "evidence":[{"kind":"configuration","source":"test","reference":"explicit compatible HTTP contract","checked_at":"2026-11-12"}]
    })).unwrap()]);
    configure(&mut profile, "http://127.0.0.1:12345").unwrap();
    let settings: serde_json::Value =
        serde_json::from_str(&profile.config().env["OPENCODE_CONFIG_CONTENT"]).unwrap();
    assert_eq!(settings["provider"]["session-provider"]["npm"], "@ai-sdk/openai-compatible");
    assert_eq!(profile.model_api.as_ref().unwrap().adapter, "cohere");
    assert!(!settings.to_string().contains("example.invalid"));
}
