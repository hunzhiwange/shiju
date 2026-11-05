use super::catalog::*;
use vw_api_types::agent_apps::AppId;

#[test]
fn cc_apps_catalog_capabilities_exact_identities_and_cli_boundary() {
    let apps = catalog();
    let expected = [
        (AppId::Claude, "Claude", Some("claude")),
        (AppId::ClaudeDesktop, "Claude Desktop", None),
        (AppId::Codex, "Codex", Some("codex")),
        (AppId::Gemini, "Gemini", Some("gemini")),
        (AppId::GrokBuild, "Grok Build", Some("grok")),
        (AppId::OpenCode, "OpenCode", Some("opencode")),
        (AppId::OpenClaw, "OpenClaw", Some("openclaw")),
        (AppId::Hermes, "Hermes", Some("hermes")),
        (AppId::Pi, "Pi", Some("pi")),
        (AppId::Mcode, "MiniMax Code", Some("mcode")),
        (AppId::Kimi, "Kimi Code", Some("kimi")),
        (AppId::Dsh, "DeepSeek Harness", Some("dsh")),
        (AppId::Agy, "Antigravity", Some("agy")),
        (AppId::Omp, "Oh My Pi", Some("omp")),
    ];
    assert_eq!(apps.len(), expected.len());
    for (app, (id, name, cli)) in apps.iter().zip(expected) {
        assert_eq!(
            (app.metadata.id, app.metadata.name.as_str(), app.metadata.cli_command.as_deref()),
            (id, name, cli)
        );
    }
    assert_eq!(apps.iter().filter(|a| a.metadata.cli_command.is_some()).count(), 13);
}

#[test]
fn cc_apps_catalog_capabilities_registration_never_promises_execution() {
    let apps = catalog();
    assert_eq!(apps.len(), 14);
    for app in apps {
        for operation in Operation::ALL {
            let capability = app.capability(operation);
            assert!(!capability.reason.is_empty());
            if app.metadata.id == AppId::ClaudeDesktop
                && matches!(
                    operation,
                    Operation::Install
                        | Operation::Update
                        | Operation::Uninstall
                        | Operation::Launch
                )
            {
                assert_eq!(capability.state, SupportState::Unsupported);
            } else {
                assert_eq!(capability.state, SupportState::Unverified);
            }
        }
    }
}
