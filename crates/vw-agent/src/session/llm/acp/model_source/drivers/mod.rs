//! Explicit dispatch. A driver is disabled until its real CLI acceptance task passes.
use super::profile::AcpLaunchProfile;
mod auggie;
mod claude;
mod codebuddy;
pub(in crate::session::llm::acp) mod codex;
mod copilot;
mod cursor;
mod droid;
mod gemini;
mod goose;
mod hermes;
mod iflow;
mod kilocode;
mod kimi;
mod kiro;
mod openclaw;
mod opencode;
mod pi;
mod qoder;
mod qwen;
mod snow;
mod trae;
mod vibe;

pub(super) fn configure(
    agent: &str,
    profile: &mut AcpLaunchProfile,
    endpoint: &str,
) -> anyhow::Result<()> {
    match agent {
        "auggie" => auggie::configure(profile, endpoint),
        "claude" => claude::configure(profile, endpoint),
        "codex" => codex::configure(profile, endpoint),
        "codebuddy" => codebuddy::configure(profile, endpoint),
        "goose" => goose::configure(profile, endpoint),
        "vibe" => vibe::configure(profile, endpoint),
        "hermes" => hermes::configure(profile, endpoint),
        "snow" => snow::configure(profile, endpoint),
        "copilot" => copilot::configure(profile, endpoint),
        "cursor" => cursor::configure(profile, endpoint),
        "droid" => droid::configure(profile, endpoint),
        "gemini" => gemini::configure(profile, endpoint),
        "iflow" => iflow::configure(profile, endpoint),
        "kilocode" => kilocode::configure(profile, endpoint),
        "kimi" => kimi::configure(profile, endpoint),
        "kiro" => kiro::configure(profile, endpoint),
        "opencode" => opencode::configure(profile, endpoint),
        "openclaw" => openclaw::configure(profile, endpoint),
        "pi" => pi::configure(profile, endpoint),
        "qoder" => qoder::configure(profile, endpoint),
        "qwen" => qwen::configure(profile, endpoint),
        "trae" => trae::configure(profile, endpoint),
        _ => anyhow::bail!("unknown ACP model-source driver"),
    }
}

/// Only native routes with actual Darwin arm64 model-request evidence are published.
/// Patched adapters stay unavailable until their reviewed artifact is installed.
pub(in crate::session::llm::acp) fn capabilities(
    agent: &str,
) -> Vec<vw_api_types::acp_model_source::AcpModelSourceCapability> {
    use vw_api_types::acp_model_source::{
        AcpModelSource as Source, AcpModelSourceCapability as Capability,
        AcpModelSourceRejectionReason as Reason,
    };
    let patched =
        matches!(agent, "codex" | "gemini" | "iflow" | "qwen" | "pi" | "goose" | "vibe" | "hermes");
    let verified = cfg!(all(target_os = "macos", target_arch = "aarch64"))
        && verified_release(agent).is_some()
        && (!patched
            || vw_acp::reviewed_agent_command(agent).is_ok_and(|command| command.is_some()));
    vec![
        if verified {
            Capability::Supported { source: Source::SessionModel }
        } else {
            Capability::Unsupported { source: Source::SessionModel, reason: Reason::UnsupportedCli }
        },
        Capability::Supported { source: Source::CliLocal },
    ]
}

/// These are evidence bounds, never a compatibility claim for a newer release.
pub(in crate::session::llm::acp) fn verified_release(agent: &str) -> Option<&'static str> {
    Some(match agent {
        "claude" => "0.26.0 (SDK 0.2.96 / Claude Code 2.1.96)",
        "codex" => "2.5.0 (Codex 0.165.0)",
        "gemini" => "0.66.0",
        "iflow" => "0.5.19",
        "qwen" => "0.26.7",
        "pi" => "0.0.22 (Pi 0.73.1)",
        "goose" => "1.57.0",
        "vibe" => "3.0.4",
        "hermes" => "0.19.0",
        "codebuddy" => "2.179.3",
        "snow" => "0.9.14",
        "copilot" => "1.0.105",
        "kilocode" => "7.8.18",
        "kimi" => "2.5.0",
        "kiro" => "3.0.0",
        "opencode" => "1.19.5",
        "openclaw" => "2026.10.10",
        _ => return None,
    })
}
