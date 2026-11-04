use super::acp_install_command;

#[test]
fn installation_commands_are_structured_and_reject_custom_input() {
    assert_eq!(
        acp_install_command("codex"),
        Some(("npm", &["install", "-g", "@agentclientprotocol/codex-acp@latest"][..]))
    );
    assert_eq!(
        acp_install_command("kimi"),
        Some(("npm", &["install", "-g", "@moonshot-ai/kimi-code@2.5.0"][..]))
    );
    for name in ["custom", "codex; echo unsafe", "../codex"] {
        assert!(acp_install_command(name).is_none());
    }
}

#[test]
fn all_builtin_agents_have_unix_and_windows_installers() {
    use super::{AcpInstallPlatform, acp_install_command_for_platform};
    for agent in [
        "auggie",
        "claude",
        "codex",
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
        "codebuddy",
        "goose",
        "vibe",
        "hermes",
        "snow",
    ] {
        for platform in [AcpInstallPlatform::Unix, AcpInstallPlatform::Windows] {
            let (program, args) = acp_install_command_for_platform(agent, platform).expect(agent);
            assert!(!args.is_empty());
            if ["cursor", "droid", "goose", "trae", "kiro"].contains(&agent) {
                assert_eq!(
                    program,
                    if platform == AcpInstallPlatform::Windows { "powershell" } else { "bash" }
                );
                let script = args.last().unwrap();
                assert!(script.contains("https://"));
                if platform == AcpInstallPlatform::Unix {
                    assert!(script.contains("set -euo pipefail"));
                }
                if agent == "goose" {
                    assert!(script.contains("false"));
                }
            }
        }
    }
}
