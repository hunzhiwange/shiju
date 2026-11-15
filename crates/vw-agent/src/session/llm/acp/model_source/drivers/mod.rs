//! Explicit dispatch. A driver is disabled until its real CLI acceptance task passes.
use super::profile::AcpLaunchProfile;
mod amp_acp;
mod auggie;
mod claude;
mod codebuddy;
pub(in crate::session::llm::acp) mod codex;
mod copilot;
mod cursor;
mod dirac;
mod droid;
mod gemini;
mod glm_acp_agent;
mod goose;
mod grok;
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
mod sigit;
mod snow;
mod trae;
mod vibe;

pub(super) fn configure(
    agent: &str,
    profile: &mut AcpLaunchProfile,
    endpoint: &str,
) -> anyhow::Result<()> {
    launch_route(agent, profile)?;
    match agent {
        "amp-acp" => amp_acp::configure(profile, endpoint),
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
        "glm-acp-agent" => glm_acp_agent::configure(profile, endpoint),
        "grok" => grok::configure(profile, endpoint),
        "sigit" => sigit::configure(profile, endpoint),
        "dirac" => dirac::configure(profile, endpoint),
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

pub(in crate::session::llm::acp) use llm_runtime::acp::model_source::drivers::{capabilities, verified_release, launch_route};
#[cfg(test)]
use llm_runtime::acp::model_source::drivers::source_protocol;

#[cfg(all(test, unix))]
#[path = "drivers_tests.rs"]
mod tests;
