//! Claude Desktop discovery and request mapping from authorized, configured providers.
use std::collections::{HashMap, HashSet};

use vw_api_types::agent_apps::AppId::ClaudeDesktop;

use crate::aggregation::model_ids::{Decoded, decode, encode};

use super::{DesktopMapping, DesktopModelRoute, ROLES, claude_role, model_id, validate};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DesktopMappingError {
    InvalidConfiguration,
    InvalidModelId,
    UnknownModel,
}

/// The caller supplies only eligible third-party providers in authorized publication order.
pub struct DesktopMappingMember<'a> {
    pub provider_id: &'a str,
    pub key: &'a str,
    pub name: &'a str,
    pub mapping: &'a DesktopMapping,
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
    default_codes: Vec<DesktopModel>,
}

impl DesktopModelCatalog {
    pub fn new(
        default: DesktopMappingMember<'_>,
        members: &[DesktopMappingMember<'_>],
    ) -> Result<Self, DesktopMappingError> {
        let mut provider_ids = HashSet::from([default.provider_id]);
        let mut keys = HashSet::from([default.key]);
        let default_routes = proxy_model_routes(default.mapping)?;
        let default_aliases =
            default_routes.iter().map(|route| route.configured_id.into()).collect();
        let mut models = default_routes
            .iter()
            .map(|route| desktop_model(&default, route, route.id.into(), false))
            .collect::<Vec<_>>();

        let mut seen = HashSet::new();
        for member in members {
            if !provider_ids.insert(member.provider_id) || !keys.insert(member.key) {
                return Err(DesktopMappingError::InvalidConfiguration);
            }
            for model in desktop_models(member, &proxy_model_routes(member.mapping)?)? {
                if seen.insert(model.id.clone()) {
                    models.push(model);
                }
            }
        }
        // Cached numeric IDs from before this provider became the default still resolve,
        // after checking every currently published member for the same code.
        let default_codes = desktop_models(&default, &default_routes)?;
        Ok(Self { models, default_aliases, default_codes })
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
            Decoded::Code(_) => self.models[self.default_aliases.len()..]
                .iter()
                .chain(&self.default_codes)
                .find(|model| model.id == requested)
                .ok_or(DesktopMappingError::UnknownModel),
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
    // Empty native mappings may be used for direct login or routing bootstrap.
    // Aggregation publishes only explicitly configured models.
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

fn desktop_models(
    member: &DesktopMappingMember<'_>,
    routes: &[ProxyRoute<'_>],
) -> Result<Vec<DesktopModel>, DesktopMappingError> {
    let mut models: Vec<DesktopModel> = Vec::new();
    let mut upstream_indices = HashMap::<&str, usize>::new();
    for route in routes {
        let upstream = route.mapping.model.as_str();
        if let Some(&index) = upstream_indices.get(upstream) {
            models[index].supports_1m |= route.mapping.supports_1m;
            continue;
        }
        let id = encode(ClaudeDesktop, member.key, upstream, false)
            .map_err(|_| DesktopMappingError::InvalidConfiguration)?;
        upstream_indices.insert(upstream, models.len());
        models.push(desktop_model(member, route, id, true));
    }
    Ok(models)
}

fn desktop_model(
    member: &DesktopMappingMember<'_>,
    route: &ProxyRoute<'_>,
    id: String,
    aggregate: bool,
) -> DesktopModel {
    let upstream_model = route.mapping.model.clone();
    let name = route.mapping.label_override.as_deref().unwrap_or(&upstream_model);
    let label = if aggregate { format!("{} / {name}", member.name) } else { name.into() };
    DesktopModel {
        id,
        provider_id: member.provider_id.into(),
        upstream_model,
        label,
        supports_1m: route.mapping.supports_1m,
    }
}

fn is_reserved(model: &str) -> bool {
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
