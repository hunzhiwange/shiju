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

#[test]
fn acp_onboarding_autohand_installer_contract() {
    use super::{AcpInstallPlatform, acp_install_command_for_platform};
    let (program, args) =
        acp_install_command_for_platform("autohand", AcpInstallPlatform::Unix).unwrap();
    assert_eq!(program, "env");
    assert_eq!(
        args,
        [
            "AUTOHAND_SKIP_AHTRACES_INSTALL=1",
            "AUTOHAND_SKIP_COMPUTER_CONTROL_INSTALL=1",
            "npm",
            "install",
            "-g",
            "autohand-cli@0.9.11"
        ]
    );
    let (program, args) =
        acp_install_command_for_platform("autohand", AcpInstallPlatform::Windows).unwrap();
    assert_eq!(program, "powershell");
    assert_eq!(&args[..3], ["-NoProfile", "-NonInteractive", "-Command"]);
    assert!(args[3].contains("npm install -g autohand-cli@0.9.11; exit $LASTEXITCODE"));
    assert!(args[3].contains("AUTOHAND_SKIP_AHTRACES_INSTALL='1'"));
    assert!(args[3].contains("AUTOHAND_SKIP_COMPUTER_CONTROL_INSTALL='1'"));
    assert!(acp_install_command("autohand; echo unsafe").is_none());
}

#[test]
fn acp_onboarding_deepagents_installer_contract() {
    use super::{AcpInstallPlatform, acp_install_command_for_platform};
    for platform in [AcpInstallPlatform::Unix, AcpInstallPlatform::Windows] {
        let (_, args) = acp_install_command_for_platform("deepagents", platform).unwrap();
        assert_eq!(args, ["install", "-g", "deepagents-acp@0.1.42"]);
    }
    assert!(acp_install_command("deepagents; echo unsafe").is_none());
}
#[test]
fn acp_onboarding_dimcode_installer_contract() {
    use super::{AcpInstallPlatform, acp_install_command_for_platform};
    for platform in [AcpInstallPlatform::Unix, AcpInstallPlatform::Windows] {
        let (_, args) = acp_install_command_for_platform("dimcode", platform).unwrap();
        assert_eq!(args, ["install", "-g", "dimcode@0.5.28"]);
    }
    assert!(acp_install_command("dimcode; echo unsafe").is_none());
}

#[test]
fn acp_onboarding_dirac_installer_contract() {
    use super::{AcpInstallPlatform, acp_install_command_for_platform};
    for platform in [AcpInstallPlatform::Unix, AcpInstallPlatform::Windows] {
        let (_, args) = acp_install_command_for_platform("dirac", platform).unwrap();
        assert_eq!(args, ["install", "-g", "dirac-cli@0.5.24"]);
    }
    assert!(acp_install_command("dirac; echo unsafe").is_none());
}

#[test]
fn acp_onboarding_glm_acp_agent_installer_contract() {
    use super::{AcpInstallPlatform, acp_install_command_for_platform};
    for platform in [AcpInstallPlatform::Unix, AcpInstallPlatform::Windows] {
        let (_, args) = acp_install_command_for_platform("glm-acp-agent", platform).unwrap();
        assert_eq!(args, ["install", "-g", "glm-acp-agent@1.17.1"]);
    }
    assert!(acp_install_command("glm-acp-agent; echo unsafe").is_none());
}

#[test]
fn acp_onboarding_grok_installer_contract() {
    use super::{AcpInstallPlatform, acp_install_command_for_platform};
    for platform in [AcpInstallPlatform::Unix, AcpInstallPlatform::Windows] {
        let (_, args) = acp_install_command_for_platform("grok", platform).unwrap();
        assert_eq!(args, ["install", "-g", "@xai-official/grok@1.0.77"]);
    }
    assert!(acp_install_command("grok; echo unsafe").is_none());
}

#[test]
fn acp_onboarding_nova_installer_contract() {
    use super::{AcpInstallPlatform, acp_install_command_for_platform};
    for platform in [AcpInstallPlatform::Unix, AcpInstallPlatform::Windows] {
        let (_, args) = acp_install_command_for_platform("nova", platform).unwrap();
        assert_eq!(args, ["install", "-g", "@compass-ai/nova@1.1.65"]);
    }
    assert!(acp_install_command("nova; echo unsafe").is_none());
}
