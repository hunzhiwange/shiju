#![allow(unused_imports)]
use crate::acp::model_source::profile::AcpLaunchProfile;
use std::{collections::HashMap, io::Write, path::Path};
use serde_json::json;
use model_reasoning::{ReasoningFormat, reasoning_levels, reasoning_profile};
use crate::acp::model_source::drivers::droid::*;
use agent_config::schema::AcpAgentConfig;
use model_transport::RelayIdentity;
use model_types::ApiInfo;


fn profile() -> AcpLaunchProfile {
    AcpLaunchProfile::new(
        &AcpAgentConfig {
            command: "droid".into(),
            args: vec!["exec".into(), "--output-format".into(), "acp".into()],
            ..Default::default()
        },
        RelayIdentity {
            tenant: vw_api_transport::TenantScope::default(),
            project: "droid-fixture".into(),
            session: "isolated".into(),
            provider: "fixture".into(),
            model: "fixture-model".into(),
            generation: 0,
        },
    )
    .unwrap()
}

#[test]
fn acp_model_source_droid_rejects_unreviewed_executable_and_remote_relay() {
    let mut launch = profile();
    assert!(configure(&mut launch, "http://127.0.0.1:12345").is_err());
    for endpoint in
        ["https://example.com", "http://127.0.0.1:12345/v1", "http://user@127.0.0.1:12345/"]
    {
        let error = configure(&mut launch, endpoint).unwrap_err().to_string();
        assert!(error.contains("loopback"));
    }
}

#[test]
fn acp_model_source_droid_rejects_native_model_and_environment_overrides() {
    let mut launch = profile();
    launch.append_arguments(&["--model".into(), "other".into()]).unwrap();
    assert!(
        configure(&mut launch, "http://127.0.0.1:12345")
            .unwrap_err()
            .to_string()
            .contains("without overrides")
    );
    let mut config = AcpAgentConfig {
        command: "droid".into(),
        args: vec!["exec".into(), "--output-format".into(), "acp".into()],
        ..Default::default()
    };
    config.env.insert("FACTORY_API_KEY".into(), "fixture-private".into());
    let mut launch = AcpLaunchProfile::new(&config, profile().binding.identity().clone()).unwrap();
    let error = configure(&mut launch, "http://127.0.0.1:12345").unwrap_err().to_string();
    assert!(error.contains("conflicting"));
    assert!(!error.contains("fixture-private"));
}
