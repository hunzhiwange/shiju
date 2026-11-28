//! Desktop's discovery and role/numeric model projection on the single listener.
use super::{
    RouteCommitError, RouteRevocation, RoutingListener, VerifiedRouteBinding,
    route_target::RouteTarget,
    dispatch::JsonTarget,
};
use model_protocol::generation::GenerationProtocol;
use axum::http::StatusCode;
use serde_json::{Value, json};
use std::net::SocketAddr;
use vw_agent_apps::providers::{
    desktop_mapping::catalog::{DesktopMappingError, DesktopModel, DesktopModelCatalog},
    live::commit::AppState,
};
use vw_api_app_identity::{AppId};

/// Protocol identity comes from the authorized saved profile, not a URL heuristic.
pub struct DesktopRouteTarget {
    pub route: RouteTarget,
    pub protocol: GenerationProtocol,
}

/// A committed Desktop role map and its single provider.
pub struct DesktopPublication {
    binding: VerifiedRouteBinding,
    snapshot: DesktopSnapshot,
}
impl DesktopPublication {
    pub fn new(
        binding: VerifiedRouteBinding,
        revision: String,
        state: AppState,
        catalog: DesktopModelCatalog,
        provider: DesktopRouteTarget,
    ) -> Result<Self, StatusCode> {
        let route = provider.route;
        if binding.app() != AppId::ClaudeDesktop
            || catalog.models().is_empty()
            || catalog.models().iter().any(|model| model.provider_id != route.provider_id)
            || !matches!(provider.protocol, GenerationProtocol::Messages | GenerationProtocol::Chat)
            || route.messages_api_key && provider.protocol != GenerationProtocol::Messages
        {
            return Err(StatusCode::UNPROCESSABLE_ENTITY);
        }
        if route.binding != binding {
            return Err(StatusCode::FORBIDDEN);
        }
        if revision.trim().is_empty() || route.revision != revision {
            return Err(StatusCode::CONFLICT);
        }
        state.require_project(binding.project()).map_err(|_| StatusCode::FORBIDDEN)?;
        if state.stack.enabled || !state.mode.as_ref().is_some_and(|mode| {
            mode.attached
                && mode.mode == vw_agent_apps::providers::live::commit::Mode::Proxy
                && mode.proxy_route.as_deref() == Some(route.provider_id.as_str())
        }) {
            return Err(StatusCode::CONFLICT);
        }
        let url = &route.target.url;
        model_transport::upstream::validate_target(
            url, &reqwest::Request::new(reqwest::Method::POST, url.clone()),
        )?;
        vw_agent_apps::providers::validation::validate(AppId::ClaudeDesktop, &route.target.request_options)
            .map_err(|_| StatusCode::UNPROCESSABLE_ENTITY)?;
        super::body_limits::encode_json(&discovery(&catalog), super::body_limits::MAX_JSON_BYTES)
            .map_err(|_| StatusCode::UNPROCESSABLE_ENTITY)?;
        Ok(Self {
            binding,
            snapshot: DesktopSnapshot {
                revision,
                catalog,
                route,
                protocol: provider.protocol,
            },
        })
    }
}

pub(crate) struct DesktopSnapshot {
    revision: String,
    catalog: DesktopModelCatalog,
    route: RouteTarget,
    protocol: GenerationProtocol,
}
impl DesktopSnapshot {
    pub(crate) fn revision(&self) -> &str {
        &self.revision
    }
    fn lookup(&self, model: Option<&str>) -> Result<&DesktopModel, StatusCode> {
        self.catalog.resolve(model.ok_or(StatusCode::BAD_REQUEST)?).map_err(|error| match error {
            DesktopMappingError::InvalidModelId => StatusCode::BAD_REQUEST,
            DesktopMappingError::UnknownModel => StatusCode::NOT_FOUND,
            DesktopMappingError::InvalidConfiguration => StatusCode::UNPROCESSABLE_ENTITY,
        })
    }
    pub(crate) fn select(
        &self,
        model: Option<&str>,
    ) -> Result<(&JsonTarget, Option<&str>, &Value), StatusCode> {
        let model = self.lookup(model)?;
        Ok((&self.route.target, Some(&model.upstream_model), &self.route.media_settings))
    }
    pub(crate) fn messages_api_key(&self, model: Option<&str>) -> Result<bool, StatusCode> {
        self.lookup(model)?;
        Ok(self.route.messages_api_key)
    }
    pub(crate) fn native_messages(&self, model: Option<&str>) -> Result<bool, StatusCode> {
        self.lookup(model)?;
        Ok(self.protocol == GenerationProtocol::Messages)
    }
    pub(crate) fn discovery(&self) -> Value {
        discovery(&self.catalog)
    }
}
impl RoutingListener {
    /// The caller holds the project/application write authorization. As with all
    /// publications, the private profile transaction runs before token activation.
    pub fn publish_desktop(
        &self,
        publication: DesktopPublication,
        expected_revision: Option<&str>,
        commit: impl FnOnce(SocketAddr, &str) -> Result<(), RouteCommitError>,
    ) -> Result<RouteRevocation, StatusCode> {
        let _permit = self.try_control().map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
        let address = self
            .bound_address()
            .filter(|address| address.ip().is_loopback())
            .ok_or(StatusCode::SERVICE_UNAVAILABLE)?;
        let DesktopPublication { binding, snapshot } = publication;
        self.auth
            .publish_desktop(binding, snapshot, expected_revision, |token| commit(address, token))
    }
}

pub fn headers(native: bool, outgoing: &mut axum::http::HeaderMap) {
    if native {
        outgoing.insert("anthropic-version", axum::http::HeaderValue::from_static("2023-06-01"));
    }
}

pub(crate) fn discovery(catalog: &DesktopModelCatalog) -> Value {
    let models = catalog.models();
    json!({"data":models.iter().map(|model|json!({"id":model.id,"type":"model","display_name":model.label,"description":model.label,"created_at":"2024-01-01T00:00:00Z","supports1m":model.supports_1m})).collect::<Vec<_>>(),
            "has_more":false,"first_id":models.first().map(|m|&m.id),"last_id":models.last().map(|m|&m.id)})
}

pub fn request(body: &Value, native: bool) -> Result<Value, StatusCode> {
    if native { Ok(body.clone()) } else { model_protocol::messages_chat_request::to_chat(body) }
}

impl RoutingListener {
    pub fn desktop_revision(&self, binding: &VerifiedRouteBinding) -> Option<String> {
        self.auth.desktop_revision(binding)
    }
}
