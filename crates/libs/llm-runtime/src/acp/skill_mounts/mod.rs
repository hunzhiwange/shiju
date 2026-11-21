//! Explicit supplier gates. Unverified directory contracts must fail closed.
use super::{Error, acp_option_error};
pub(super) mod aionrs;
pub(super) mod amp_acp;
pub(super) mod auggie;
mod claude;
mod codebuddy;
mod codex;
mod copilot;
pub(super) mod corust_agent;
pub(super) mod cortex_code;
mod cursor;
pub(super) mod dimcode;
mod dirac;
pub(super) mod droid;
#[cfg(test)]
mod deepagents_tests;
mod gemini;
pub(super) mod glm_acp_agent;
mod goose;
mod grok;
pub(super) mod hermes;
mod iflow;
pub(super) mod junie;
pub(super) mod kilocode;
mod kimi;
mod kiro;
pub(super) mod mimo_code;
mod nova;
pub(super) mod omp;
mod openclaw;
pub(super) mod opencode;
pub(super) mod pi;
pub(super) mod poolside;
pub(super) mod qoder;
mod qwen;
pub(super) mod sigit;
mod snow;
pub(super) mod trae;
mod vibe;

pub fn validate(agent: &str, selected: bool) -> Result<(), Error> {
    if !selected {
        return Ok(());
    }
    let reason = match agent.trim().to_ascii_lowercase().as_str() {
        "codex" => return codex::validate(),
        "claude" | "claude code" => return Ok(()),
        "qwen" => return Ok(()),
        "gemini" => return Ok(()),
        "glm-acp-agent" if cfg!(all(target_os = "macos", target_arch = "aarch64")) => return Ok(()),
        "codebuddy" => return Ok(()),
        "goose" => return Ok(()),
        "grok" if cfg!(all(target_os = "macos", target_arch = "aarch64")) => return Ok(()),
        "vibe" => return Ok(()),
        "hermes" => return Ok(()),
        "snow" => return Ok(()),
        "aionrs" if cfg!(all(target_os = "macos", target_arch = "aarch64")) => return Ok(()),
        "sigit" if cfg!(all(target_os = "macos", target_arch = "aarch64")) => return Ok(()),
        "copilot" => return Ok(()),
        "corust-agent" if cfg!(all(target_os = "macos", target_arch = "aarch64")) => return Ok(()),
        "junie" if cfg!(all(target_os = "macos", target_arch = "aarch64")) => return Ok(()),
        "kilocode" => return Ok(()),
        "kimi" => return Ok(()),
        "kiro" => return Ok(()),
        "opencode" => return Ok(()),
        "mimo-code" if cfg!(all(target_os = "macos", target_arch = "aarch64")) => return Ok(()),
        "nova" if cfg!(all(target_os = "macos", target_arch = "aarch64")) => return Ok(()),
        "omp" if cfg!(all(target_os = "macos", target_arch = "aarch64")) => return Ok(()),
        "openclaw" => return Ok(()),
        "pi" => return Ok(()),
        "poolside" if cfg!(all(target_os = "macos", target_arch = "aarch64")) => return Ok(()),
        "amp-acp" if cfg!(all(target_os = "macos", target_arch = "aarch64")) => return Ok(()),
        "auggie" if cfg!(all(target_os = "macos", target_arch = "aarch64")) => return Ok(()),
        "cursor" => return Ok(()),
        "deepagents" if cfg!(all(target_os = "macos", target_arch = "aarch64")) => return Ok(()),
        "cortex-code" if cfg!(all(target_os = "macos", target_arch = "aarch64")) => return Ok(()),
        "dirac" if cfg!(all(target_os = "macos", target_arch = "aarch64")) => return Ok(()),
        "dimcode" if cfg!(all(target_os = "macos", target_arch = "aarch64")) => return Ok(()),
        "droid" if cfg!(all(target_os = "macos", target_arch = "aarch64")) => return Ok(()),
        "qoder" if cfg!(all(target_os = "macos", target_arch = "aarch64")) => return Ok(()),
        "trae" if cfg!(all(target_os = "macos", target_arch = "aarch64")) => return Ok(()),
        "iflow" => return Ok(()),
        _ => "no verified skill directory mount branch",
    };
    Err(acp_option_error(format!("{agent}: {reason}")))
}

#[derive(Clone)]
pub struct MountedSkills(
    pub std::sync::Arc<agent_skills::session_mounts::SessionSkillView>,
);
impl PartialEq for MountedSkills {
    fn eq(&self, other: &Self) -> bool {
        self.0.fingerprint() == other.0.fingerprint()
    }
}
impl Eq for MountedSkills {}
impl super::host::SkillLease for MountedSkills {
    fn fingerprint(&self) -> &str { self.0.fingerprint() }
    fn root(&self) -> &std::path::Path { self.0.root() }
    fn validate(&self) -> Result<(), Error> { self.0.validate().map_err(super::to_api_error) }
}
impl MountedSkills {
    pub fn prepare(
        mounts: &[acp_types::SessionSkillMount],
        cwd: &std::path::Path,
        session: &str,
        security: &config_skills::SkillSecurityConfig,
        tenant: &vw_api_transport::TenantScope,
    ) -> Result<Option<Self>, Error> {
        if mounts.is_empty() {
            return Ok(None);
        }
        let binding = agent_skills::session_mounts::SessionSkillBinding {
            tenant: serde_json::to_string(tenant)
                .map_err(super::to_api_error)?,
            project: cwd.to_path_buf(),
            session: session.to_string(),
            revision: 0,
        };
        let view =
            agent_skills::session_mounts::SessionSkillView::prepare(binding, mounts, security)
                .map_err(super::to_api_error)?;
        Ok(Some(Self(std::sync::Arc::new(view))))
    }
}

/// Native invocation snapshots are expanded bodies, not mount authorization.
/// Reject them before prompt assembly until a directory-backed invocation exists.
pub fn reject_expanded_invocations(
    agent: &str,
    messages: &serde_json::Value,
) -> Result<(), Error> {
    for message in messages.as_array().into_iter().flatten() {
        let content = &message["content"];
        let expanded = |text: &str| text.trim_start().starts_with("<system-reminder>\nSkill \"");
        if content.as_str().is_some_and(expanded)
            || content.as_array().is_some_and(|blocks| {
                blocks.iter().any(|block| block["text"].as_str().is_some_and(expanded))
            })
        {
            return Err(acp_option_error(format!(
                "{agent}: expanded native skill invocation cannot be forwarded; directory mounting requires adapter acceptance"
            )));
        }
    }
    Ok(())
}

#[cfg(test)]
#[path = "outbound_tests.rs"]
mod outbound_tests;



pub fn prepare_supplier(agent: &str, view: Option<&MountedSkills>) -> Result<(), Error> {
    if matches!(agent, "claude" | "claude code")
        && let Some(view) = view
    {
        claude::prepare(&view.0)?;
    }
    Ok(())
}

/// Apply reviewed scanner contracts; model and credential settings stay independent.
pub fn configure_native_scanner(
    agent: &str,
    config: &mut agent_config::schema::AcpAgentConfig,
    view: Option<&MountedSkills>,
) -> Result<(), Error> {
    if agent == "grok" {
        grok::configure(config, view)?;
    } else if agent == "kimi" {
        kimi::configure(config, view)?;
    } else if agent == "dirac" {
        dirac::configure(config, view)?;
    } else if agent == "cursor" {
        cursor::configure(config, view)?;
    } else if agent == "iflow" {
        iflow::configure(config, view)?;
    } else if agent == "pi" {
        pi::configure(config, view)?;
    } else if agent == "nova" {
        nova::configure(config, view)?;
    } else if agent == "openclaw" {
        openclaw::configure(config, view)?;
    } else if agent == "opencode" {
        opencode::configure(config, view)?;
    } else if agent == "kiro" {
        kiro::configure(config, view)?;
    } else if agent == "kilocode" {
        kilocode::configure(config, view)?;
    } else if agent == "copilot" {
        copilot::configure(config, view)?;
    } else if agent == "snow" {
        snow::configure(config, view)?;
    } else if agent == "hermes" {
        hermes::configure(config, view)?;
    } else if agent == "vibe" {
        vibe::configure(config, view)?;
    } else if agent == "goose" {
        goose::configure(config, view)?;
    } else if agent == "qwen" {
        qwen::configure(config, view)?;
    } else if agent == "codebuddy" {
        codebuddy::configure(config, view)?;
    } else if agent == "gemini" {
        gemini::configure(config, view)?;
    }
    Ok(())
}
