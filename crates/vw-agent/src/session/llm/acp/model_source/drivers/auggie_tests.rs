use super::*;
use crate::{config::schema::AcpAgentConfig, model_relay::RelayIdentity};

#[test]
fn auggie_rejects_remote_endpoints_and_conflicting_launches_before_writing_files() {
    for (endpoint, args, env, expected) in [
        ("https://example.invalid/", vec!["--acp"], HashMap::new(), "loopback"),
        ("http://127.0.0.1:1234/", vec!["--acp", "--model", "other"], HashMap::new(), "arguments"),
        (
            "http://127.0.0.1:1234/",
            vec!["--acp"],
            HashMap::from([("AUGMENT_API_TOKEN".into(), "private-fixture-token".into())]),
            "conflicting",
        ),
    ] {
        let config = AcpAgentConfig {
            command: "auggie".into(),
            args: args.into_iter().map(str::to_owned).collect(),
            env,
            ..Default::default()
        };
        let mut profile = AcpLaunchProfile::new(
            &config,
            RelayIdentity {
                tenant: crate::session::ui_store::current_tenant_scope(),
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
