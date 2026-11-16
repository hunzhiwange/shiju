use crate::acp::model_source::{drivers, profile::AcpLaunchProfile};
fn identity(session: &str) -> model_transport::RelayIdentity {
    model_transport::RelayIdentity { tenant: Default::default(), project: "p".into(), session: session.into(), provider: "provider".into(), model: "model".into(), generation: 0 }
}
#[test]
fn acp_model_source_launch_profile_all_drivers_fail_closed_without_secret_errors() {
    let mut profile = AcpLaunchProfile::new(&Default::default(), identity("one")).unwrap();
    for key in [
        "auggie",
        "codex",
        "codebuddy",
        "goose",
        "vibe",
        "hermes",
        "snow",
        "copilot",
        "cursor",
        "droid",
        "gemini",
        "iflow",
        "kilocode",
        "kimi",
        "kiro",
        "opencode",
        "openclaw",
        "pi",
        "qoder",
        "qwen",
        "trae",
    ] {
        let error =
            drivers::configure(key, &mut profile, "http://127.0.0.1:1").unwrap_err().to_string();
        assert!(!error.is_empty());
        assert!(!error.contains(profile.binding.bearer_token()));
    }
}
