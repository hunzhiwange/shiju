#![allow(unused_imports)]
use crate::acp::model_source::profile::AcpLaunchProfile;
use std::{collections::HashMap, io::Write, path::Path};
use serde_json::json;
use model_reasoning::{ReasoningFormat, reasoning_levels, reasoning_profile};
use crate::acp::model_source::drivers::cursor::*;
use agent_config::schema::AcpAgentConfig;
use model_transport::RelayIdentity;
use model_types::ApiInfo;


#[test]
fn cursor_rejects_remote_endpoints_and_conflicting_launches_before_writing_files() {
    for (endpoint, args, env, expected) in [
        ("https://example.invalid/", vec!["acp"], HashMap::new(), "loopback"),
        ("http://127.0.0.1:1234/", vec!["acp", "--model", "other"], HashMap::new(), "arguments"),
        (
            "http://127.0.0.1:1234/",
            vec!["acp"],
            HashMap::from([("CURSOR_API_KEY".into(), "private-fixture-token".into())]),
            "conflicting",
        ),
    ] {
        let config = AcpAgentConfig {
            command: "cursor".into(),
            args: args.into_iter().map(str::to_owned).collect(),
            env,
            ..Default::default()
        };
        let mut profile = AcpLaunchProfile::new(
            &config,
            RelayIdentity {
                tenant: vw_api_transport::TenantScope::default(),
                project: "fixture".into(),
                session: "fixture".into(),
                provider: "fixture".into(),
                model: "selected".into(),
                generation: 0,
            },
        )
        .unwrap();
        let error = configure(&mut profile, endpoint).unwrap_err().to_string();
        assert!(error.contains(expected), "{error}");
        assert!(!error.contains("private-fixture-token"));
        assert!(!error.contains(profile.binding.bearer_token()));
        assert_eq!(std::fs::read_dir(profile.directory()).unwrap().count(), 0);
    }
}
