//! Claude Desktop discovery and request mapping from authorized, configured providers.
use vw_api_app_identity::{AppId::ClaudeDesktop};

use crate::aggregation::model_ids::{Decoded, decode};

use super::{DesktopMapping, DesktopModelRoute, ROLES, claude_role, model_id, validate};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DesktopMappingError {
    InvalidConfiguration,
    InvalidModelId,
    UnknownModel,
}

#[derive(Debug, PartialEq, Eq)]
pub struct DesktopModel {
    pub id: String,
    pub provider_id: String,
    pub upstream_model: String,
    pub label: String,
    pub supports_1m: bool,
}

/// Discovery and resolution use the same ordered snapshot so collisions cannot choose
/// different providers in the model picker and the request path.
#[derive(Debug)]
pub struct DesktopModelCatalog {
    models: Vec<DesktopModel>,
    // Stored keys of the leading default models, in the same order as those models.
    default_aliases: Vec<String>,
}

impl DesktopModelCatalog {
    pub fn new(provider_id: &str, mapping: &DesktopMapping) -> Result<Self, DesktopMappingError> {
        let routes = proxy_model_routes(mapping)?;
        let default_aliases = routes.iter().map(|route| route.configured_id.into()).collect();
        let models = routes.iter().map(|route| {
            let upstream_model = route.mapping.model.clone();
            let label = route.mapping.label_override.clone().unwrap_or_else(|| upstream_model.clone());
            DesktopModel {
                id: route.id.into(),
                provider_id: provider_id.into(),
                upstream_model,
                label,
                supports_1m: route.mapping.supports_1m,
            }
        }).collect();
        Ok(Self { models, default_aliases })
    }

    pub fn models(&self) -> &[DesktopModel] {
        &self.models
    }

    pub fn resolve(&self, requested: &str) -> Result<&DesktopModel, DesktopMappingError> {
        let requested = strip_context_marker(requested);
        if !model_id(requested) {
            return Err(DesktopMappingError::InvalidModelId);
        }
        match decode(ClaudeDesktop, requested).map_err(|_| DesktopMappingError::InvalidModelId)? {
            Decoded::Code(_) => Err(DesktopMappingError::UnknownModel),
            Decoded::Plain if !is_reserved(requested) => {
                self.resolve_default(requested).ok_or(DesktopMappingError::UnknownModel)
            }
            _ => Err(DesktopMappingError::InvalidModelId),
        }
    }

    fn resolve_default(&self, requested: &str) -> Option<&DesktopModel> {
        let defaults = &self.models[..self.default_aliases.len()];
        if let Some(exact) = defaults.iter().find(|model| model.id == requested) {
            return Some(exact);
        }
        if let Some(index) = self.default_aliases.iter().position(|alias| alias == requested) {
            return Some(&defaults[index]);
        }
        let legacy_opus = match requested {
            "claude-opus-5" => Some("claude-opus-4-8"),
            "claude-opus-4-8" => Some("claude-opus-5"),
            _ => None,
        };
        if let Some(alias) = legacy_opus.and_then(|id| defaults.iter().find(|model| model.id == id))
        {
            return Some(alias);
        }
        let (requested_role, version) = claude_role(requested)?;
        if !version
            .split(['-', '.'])
            .all(|part| !part.is_empty() && part.bytes().all(|byte| byte.is_ascii_digit()))
        {
            return None;
        }
        let find_role = |wanted| {
            defaults
                .iter()
                .find(|model| claude_role(&model.id).is_some_and(|(role, _)| role == wanted))
        };
        find_role(requested_role)
            // Older mappings have three roles. Desktop uses Opus for an unconfigured Fable role.
            .or_else(|| if requested_role == "fable" { find_role("opus") } else { None })
    }
}

struct ProxyRoute<'a> {
    id: &'a str,
    configured_id: &'a str,
    mapping: &'a DesktopModelRoute,
}

fn proxy_model_routes(
    mapping: &DesktopMapping,
) -> Result<Vec<ProxyRoute<'_>>, DesktopMappingError> {
    validate(mapping).map_err(|_| DesktopMappingError::InvalidConfiguration)?;
    // Require explicit role mappings before publishing a Desktop route.
    if mapping.routes.is_empty() || mapping.routes.keys().any(|id| is_reserved(id)) {
        return Err(DesktopMappingError::InvalidConfiguration);
    }
    let mut routes = mapping
        .routes
        .iter()
        .map(|(configured_id, route)| {
            let id = preferred_client_id(configured_id, &route.model)
                .filter(|id| !mapping.routes.contains_key(*id))
                .unwrap_or(configured_id);
            ProxyRoute { id, configured_id, mapping: route }
        })
        .collect::<Vec<_>>();
    routes.sort_by_key(|route| {
        let rank =
            claude_role(route.id).and_then(|(role, _)| ROLES.iter().position(|&tier| tier == role));
        (rank.unwrap_or(ROLES.len()), route.id)
    });
    Ok(routes)
}

pub(crate) fn is_reserved(model: &str) -> bool {
    let lower = model.to_ascii_lowercase();
    let normalized = lower.strip_prefix("anthropic/").unwrap_or(&lower);
    !matches!(decode(ClaudeDesktop, normalized), Ok(Decoded::Plain))
}

fn strip_context_marker(model: &str) -> &str {
    let model = model.trim();
    let marker_start = model.len().saturating_sub(4);
    if model.as_bytes()[marker_start..].eq_ignore_ascii_case(b"[1m]") {
        model[..marker_start].trim_end()
    } else {
        model
    }
}

fn preferred_client_id(configured: &str, upstream: &str) -> Option<&'static str> {
    let upstream = upstream.to_ascii_lowercase();
    let (stem, client) = match configured {
        "claude-opus-5" => ("opus-5", "claude-opus-5-5"),
        "claude-sonnet-5" => ("sonnet-5", "claude-sonnet-5-5"),
        "claude-haiku-4-5" => {
            return (!["haiku-4", "haiku-3", "-3-haiku", "-3-5-haiku", "-3.5-haiku"]
                .iter()
                .any(|old| upstream.contains(old)))
            .then_some("claude-haiku-5-5");
        }
        _ => return None,
    };
    [".5", "-5"]
        .iter()
        .any(|minor| {
            upstream.split_once(&format!("{stem}{minor}")).is_some_and(|(_, tail)| {
                !tail.starts_with(|c: char| c.is_ascii_digit() || c == '.')
            })
        })
        .then_some(client)
}
