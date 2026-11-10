//! Wire identifiers published by the three aggregation clients.

use vw_api_types::agent_apps::AppId;

const CLAUDE_PREFIX: &str = "ccs-claude-";
const CODEX_PREFIX: &str = "ccs-";
// Desktop filters vendor names from its selector. Six digits also avoid date-suffix stripping.
const DESKTOP_PREFIX: &str = "claude-fable-ccs-";
pub(super) const ONE_M_MARKER: &str = "[1M]";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModelIdError {
    UnsupportedApplication,
    InvalidModelId,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Decoded<'a> {
    /// The identifier has no reserved prefix for this application.
    Plain,
    Stack {
        key: &'a str,
        model: &'a str,
        one_m: bool,
    },
    /// Resolve these digits against the published Desktop catalog.
    Code(&'a str),
}

/// Encode a model with its registered member key. Pass Claude's 1M marker via `one_m`.
pub fn encode(app: AppId, key: &str, model: &str, one_m: bool) -> Result<String, ModelIdError> {
    match app {
        AppId::Claude | AppId::Codex | AppId::ClaudeDesktop
            if !valid_key(key) || model.trim().is_empty() =>
        {
            Err(ModelIdError::InvalidModelId)
        }
        AppId::Claude => {
            if strip_one_m(model).is_some() || (one_m && model.trim_end() != model) {
                return Err(ModelIdError::InvalidModelId);
            }
            let marker = if one_m { ONE_M_MARKER } else { "" };
            Ok(format!("{CLAUDE_PREFIX}{key}--{model}{marker}"))
        }
        AppId::Codex => Ok(format!("{CODEX_PREFIX}{key}/{model}")),
        AppId::ClaudeDesktop => Ok(format!("{DESKTOP_PREFIX}{:06}", desktop_code(key, model))),
        _ => Err(ModelIdError::UnsupportedApplication),
    }
}

/// Decode an application-specific identifier. Malformed reserved IDs cannot enter default routing.
pub fn decode(app: AppId, id: &str) -> Result<Decoded<'_>, ModelIdError> {
    match app {
        AppId::Claude => {
            let Some(rest) = id.strip_prefix(CLAUDE_PREFIX) else {
                return Ok(Decoded::Plain);
            };
            let unmarked = strip_one_m(rest);
            decode_stack(unmarked.unwrap_or(rest), "--", unmarked.is_some())
        }
        AppId::Codex => {
            let Some(rest) = id.strip_prefix(CODEX_PREFIX) else {
                return Ok(Decoded::Plain);
            };
            decode_stack(rest, "/", false)
        }
        AppId::ClaudeDesktop => {
            let Some(rest) = id.strip_prefix(DESKTOP_PREFIX) else {
                return Ok(Decoded::Plain);
            };
            let code = strip_one_m(rest).unwrap_or(rest);
            if code.len() == 6 && code.bytes().all(|byte| byte.is_ascii_digit()) {
                Ok(Decoded::Code(code))
            } else {
                Err(ModelIdError::InvalidModelId)
            }
        }
        _ => Err(ModelIdError::UnsupportedApplication),
    }
}

fn decode_stack<'a>(
    body: &'a str,
    separator: &str,
    one_m: bool,
) -> Result<Decoded<'a>, ModelIdError> {
    let (key, model) = body
        .split_once(separator)
        .filter(|(key, model)| valid_key(key) && !model.trim().is_empty())
        .ok_or(ModelIdError::InvalidModelId)?;
    Ok(Decoded::Stack { key, model, one_m })
}

fn valid_key(key: &str) -> bool {
    key.split('-').all(|part| {
        !part.is_empty()
            && part.bytes().all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit())
    })
}

pub(super) fn strip_one_m(text: &str) -> Option<&str> {
    let trimmed = text.trim_end();
    let start = trimmed.len().checked_sub(ONE_M_MARKER.len())?;
    trimmed.as_bytes()[start..]
        .eq_ignore_ascii_case(ONE_M_MARKER.as_bytes())
        .then(|| trimmed[..start].trim_end())
}

// Preserve the upstream FNV-1a calculation across toolchain versions.
fn desktop_code(key: &str, model: &str) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325u64;
    for byte in key.bytes().chain([0]).chain(model.bytes()) {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash % 1_000_000
}
