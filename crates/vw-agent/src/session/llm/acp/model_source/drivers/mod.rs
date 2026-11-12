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
    launch_route(agent, profile)?;
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

/// This advertises reviewed CLI binding support, not availability for every model.
/// Each selected model must pass launch_route before private configuration is written.
/// Only routes with actual Darwin arm64 model-request evidence are published.
/// Patched adapters stay unavailable until their reviewed artifact is installed.
pub(in crate::session::llm::acp) fn capabilities(
    agent: &str,
) -> Vec<vw_api_types::acp_model_source::AcpModelSourceCapability> {
    use vw_api_types::acp_model_source::{
        AcpModelSource as Source, AcpModelSourceCapability as Capability,
        AcpModelSourceRejectionReason as Reason,
    };
    let patched = matches!(
        agent,
        "auggie"
            | "trae"
            | "qoder"
            | "droid"
            | "codex"
            | "gemini"
            | "iflow"
            | "qwen"
            | "pi"
            | "goose"
            | "vibe"
            | "hermes"
    );
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
        "claude" => {
            "0.26.0 (SDK 0.2.96 / Claude Code 2.1.96); 0.31.0 (SDK 0.2.119 / Claude Code 2.1.119)"
        }
        "auggie" => "0.37.1",
        "qoder" => "1.1.88",
        "trae" => "0.121.3",
        "droid" => "1.9.0",
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

/// The source is the reviewed CLI's actual wire format, never the upstream target.
fn source_protocol(
    agent: &str,
    api: &crate::provider::provider::ApiInfo,
) -> anyhow::Result<vw_shared::provider::protocols::ModelProtocol> {
    use vw_shared::provider::protocols::ModelProtocol as P;
    Ok(match agent {
        "claude" => P::Messages,
        "codex" | "kiro" => P::Responses,
        "gemini" => P::Gemini,
        "auggie" | "droid" | "codebuddy" | "goose" | "hermes" | "iflow" | "kilocode"
        | "openclaw" | "pi" | "qoder" | "qwen" | "trae" => P::Chat,
        "opencode" => crate::model_relay::routing::client_protocol(api)
            .map_err(|_| anyhow::anyhow!("ACP selected model has no executable HTTP contract"))?,
        "snow" | "kimi" => crate::model_relay::routing::configured_api_protocol(api)
            .map_err(|_| anyhow::anyhow!("ACP selected model protocol is unsupported"))?,
        "copilot" => match api.adapter.as_str() {
            "anthropic" => P::Messages,
            "openai" | "openai-compatible" => {
                let protocol = crate::model_relay::routing::configured_api_protocol(api)
                    .map_err(|_| anyhow::anyhow!("ACP selected model protocol is unsupported"))?;
                anyhow::ensure!(
                    protocol == P::Chat,
                    "Copilot Responses conversion is not verified"
                );
                P::Chat
            }
            _ => anyhow::bail!("Copilot selected model protocol is unsupported"),
        },
        "vibe" => match crate::model_relay::routing::configured_api_protocol(api)
            .map_err(|_| anyhow::anyhow!("ACP selected model protocol is unsupported"))?
        {
            P::Gemini => P::Chat,
            protocol => protocol,
        },
        _ => anyhow::bail!("ACP session_model driver is not verified"),
    })
}

/// Startup uses the same deployment/authentication selector as the relay. Dynamic
/// payload capabilities are checked again when each actual request is dispatched.
pub(in crate::session::llm::acp) fn launch_route(
    agent: &str,
    profile: &AcpLaunchProfile,
) -> anyhow::Result<vw_provider_resolver::protocol_routes::ProtocolRoute> {
    use vw_provider_resolver::protocol_routes::RouteCapability as C;
    let api = profile
        .model_api
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("ACP requires selected model transport metadata"))?;
    let source = source_protocol(agent, api)?;
    let required = [C::Text, C::Streaming, C::Tools];
    let available = if profile.model_supports_tool_call { &required[..] } else { &required[..2] };
    let conversions = crate::model_relay::routing::relay_conversions(agent == "codex");
    crate::model_relay::routing::select_api(
        api,
        &profile.binding.identity.provider,
        source,
        &required,
        available,
        &conversions,
        crate::model_relay::routing::Execution::Relay,
    )
    .map_err(anyhow::Error::from)
}

#[cfg(all(test, unix))]
#[path = "drivers_tests.rs"]
mod tests;
