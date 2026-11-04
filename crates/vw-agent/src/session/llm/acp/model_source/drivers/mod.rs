//! Explicit dispatch. A driver is disabled until its real CLI acceptance task passes.
use super::profile::AcpLaunchProfile;
mod auggie;
mod claude;
mod codebuddy;
mod codex;
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
