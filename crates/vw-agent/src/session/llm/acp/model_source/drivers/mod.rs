//! Host directory injection for CLI launch configuration.
use super::profile::AcpLaunchProfile;
#[cfg(test)]
mod claude;
#[cfg(test)]
mod codex;
#[cfg(test)]
mod gemini;
#[cfg(test)]
mod goose;
#[cfg(test)]
mod hermes;
#[cfg(test)]
mod kimi;
#[cfg(test)]
mod kiro;
#[cfg(test)]
mod opencode;
#[cfg(test)]
mod pi;

pub(super) fn configure(agent: &str, profile: &mut AcpLaunchProfile, endpoint: &str) -> anyhow::Result<()> {
    llm_runtime::acp::model_source::drivers::configure(agent, profile, endpoint, &crate::project::instance::directory())
}
pub(in crate::session::llm::acp) use llm_runtime::acp::model_source::drivers::{capabilities, verified_release};
