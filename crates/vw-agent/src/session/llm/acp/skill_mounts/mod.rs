//! Explicit supplier gates. Unverified directory contracts must fail closed.
use super::{Error, acp_option_error};
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

pub(super) fn validate(agent: &str, selected: bool) -> Result<(), Error> {
    if !selected {
        return Ok(());
    }
    let reason = match agent.trim().to_ascii_lowercase().as_str() {
        "codex" => return codex::validate(),
        "claude" | "claude code" => return Ok(()),
        "qwen" => return Ok(()),
        "gemini" => return Ok(()),
        "codebuddy" => return Ok(()),
        "goose" => return Ok(()),
        "vibe" => return Ok(()),
        "hermes" => return Ok(()),
        "snow" => return Ok(()),
        "copilot" => return Ok(()),
        "kilocode" => return Ok(()),
        "kimi" => kimi::rejection(),
        "kiro" => return Ok(()),
        "opencode" => return Ok(()),
        "openclaw" => openclaw::rejection(),
        "pi" => pi::rejection(),
        "auggie" => auggie::rejection(),
        "cursor" => cursor::rejection(),
        "droid" => droid::rejection(),
        "qoder" => qoder::rejection(),
        "trae" => trae::rejection(),
        "iflow" => iflow::rejection(),
        _ => "no verified skill directory mount branch",
    };
    Err(acp_option_error(format!("{agent}: {reason}")))
}

#[derive(Clone)]
pub(crate) struct MountedSkills(
    pub std::sync::Arc<crate::skills::session_mounts::SessionSkillView>,
);
impl PartialEq for MountedSkills {
    fn eq(&self, other: &Self) -> bool {
        self.0.fingerprint() == other.0.fingerprint()
    }
}
impl Eq for MountedSkills {}
impl MountedSkills {
    pub(super) fn prepare(
        mounts: &[vw_acp::SessionSkillMount],
        cwd: &std::path::Path,
        session: &str,
        security: &vw_config_types::skills::SkillSecurityConfig,
    ) -> Result<Option<Self>, Error> {
        if mounts.is_empty() {
            return Ok(None);
        }
        let binding = crate::skills::session_mounts::SessionSkillBinding {
            tenant: serde_json::to_string(&crate::session::ui_store::current_tenant_scope())
                .map_err(super::to_api_error)?,
            project: cwd.to_path_buf(),
            session: session.to_string(),
            revision: 0,
        };
        let view =
            crate::skills::session_mounts::SessionSkillView::prepare(binding, mounts, security)
                .map_err(super::to_api_error)?;
        Ok(Some(Self(std::sync::Arc::new(view))))
    }
}

/// Native invocation snapshots are expanded bodies, not mount authorization.
/// Reject them before prompt assembly until a directory-backed invocation exists.
pub(super) fn reject_expanded_invocations(
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

#[cfg(test)]
mod acceptance_tests;

pub(super) fn prepare_supplier(agent: &str, view: Option<&MountedSkills>) -> Result<(), Error> {
    if matches!(agent, "claude" | "claude code")
        && let Some(view) = view
    {
        claude::prepare(&view.0)?;
    }
    Ok(())
}

/// Apply reviewed scanner contracts; model and credential settings stay independent.
pub(super) fn configure_native_scanner(
    agent: &str,
    config: &mut crate::config::schema::AcpAgentConfig,
    view: Option<&MountedSkills>,
) -> Result<(), Error> {
    if agent == "opencode" {
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
