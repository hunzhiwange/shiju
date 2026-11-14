//! Desktop's discovery and role/numeric model projection on the single listener.
use super::{
    RouteCommitError, RouteRevocation, RoutingListener, VerifiedRouteBinding,
    aggregation_routing::{
        AggregationModel, AggregationPublication, AggregationSnapshot, AggregationTarget,
    },
    dispatch::JsonTarget,
};
use crate::model_relay::generation::GenerationProtocol;
use axum::http::StatusCode;
use serde_json::{Value, json};
use std::{collections::HashMap, net::SocketAddr};
use vw_agent_apps::providers::{
    desktop_mapping::catalog::{DesktopMappingError, DesktopModel, DesktopModelCatalog},
    live::commit::AppState,
};
use vw_api_types::agent_apps::AppId;

/// Protocol identity comes from the authorized saved profile, not a URL heuristic.
pub(crate) struct DesktopRouteTarget {
    pub(crate) route: AggregationTarget,
    pub(crate) protocol: GenerationProtocol,
}

/// Uses the same immutable targets, revision and commit callback as other apps.
pub(crate) struct DesktopPublication {
    publication: AggregationPublication,
    catalog: DesktopModelCatalog,
    protocols: HashMap<String, GenerationProtocol>,
}
impl DesktopPublication {
    pub(crate) fn new(
        binding: VerifiedRouteBinding,
        revision: String,
        state: AppState,
        catalog: DesktopModelCatalog,
        default: DesktopRouteTarget,
        members: Vec<DesktopRouteTarget>,
    ) -> Result<Self, StatusCode> {
        if binding.app() != AppId::ClaudeDesktop
            || catalog
                .models()
                .first()
                .is_none_or(|model| model.provider_id != default.route.provider_id)
        {
            return Err(StatusCode::UNPROCESSABLE_ENTITY);
        }
        // A published discovery snapshot must fit the listener's response budget.
        // Reject it before the native transaction can publish client configuration.
        drop(
            super::body_limits::encode_json(
                &discovery(&catalog),
                super::body_limits::MAX_JSON_BYTES,
            )
            .map_err(|_| StatusCode::UNPROCESSABLE_ENTITY)?,
        );
        let mut protocols = HashMap::new();
        let check = |target: &DesktopRouteTarget| {
            matches!(target.protocol, GenerationProtocol::Messages | GenerationProtocol::Chat)
                && (!target.route.messages_api_key
                    || target.protocol == GenerationProtocol::Messages)
        };
        if !check(&default) {
            return Err(StatusCode::UNPROCESSABLE_ENTITY);
        }
        protocols.insert(default.route.provider_id.clone(), default.protocol);
        let default = default.route;
        let mut routes = Vec::new();
        for member in members {
            if !check(&member)
                || protocols.insert(member.route.provider_id.clone(), member.protocol).is_some()
            {
                return Err(StatusCode::UNPROCESSABLE_ENTITY);
            }
            routes.push(member.route);
        }
        let members = routes;
        // Ordinary role IDs belong to the default; only numeric member IDs use
        // the shared exact-aggregation selector. DesktopCatalog owns aliases.
        let models = catalog
            .models()
            .iter()
            .filter(|model| model.provider_id != default.provider_id)
            .map(|model| AggregationModel {
                id: model.id.clone(),
                provider_id: model.provider_id.clone(),
                upstream_model: model.upstream_model.clone(),
            })
            .collect();
        Ok(Self {
            publication: AggregationPublication {
                binding,
                revision,
                state,
                models,
                default,
                members,
            },
            catalog,
            protocols,
        })
    }
    pub(super) fn validate(self) -> Result<(VerifiedRouteBinding, DesktopSnapshot), StatusCode> {
        let binding = self.publication.binding.clone();
        self.publication
            .state
            .require_project(binding.project())
            .map_err(|_| StatusCode::FORBIDDEN)?;
        let snapshot = self.publication.validate_desktop()?;
        Ok((
            binding,
            DesktopSnapshot { snapshot, catalog: self.catalog, protocols: self.protocols },
        ))
    }
}

pub(super) struct DesktopSnapshot {
    snapshot: AggregationSnapshot,
    catalog: DesktopModelCatalog,
    protocols: HashMap<String, GenerationProtocol>,
}
impl DesktopSnapshot {
    pub(super) fn revision(&self) -> &str {
        &self.snapshot.revision
    }
    fn lookup<'a>(
        &'a self,
        model: Option<&str>,
    ) -> Result<(&'a DesktopModel, &'a str), StatusCode> {
        let model = self.catalog.resolve(model.ok_or(StatusCode::BAD_REQUEST)?).map_err(
            |error| match error {
                DesktopMappingError::InvalidModelId => StatusCode::BAD_REQUEST,
                DesktopMappingError::UnknownModel => StatusCode::NOT_FOUND,
                DesktopMappingError::InvalidConfiguration => StatusCode::UNPROCESSABLE_ENTITY,
            },
        )?;
        let selected = if self
            .catalog
            .models()
            .first()
            .is_some_and(|default| default.provider_id == model.provider_id)
        {
            ""
        } else {
            &model.id
        };
        Ok((model, selected))
    }
    pub(super) fn select<'a>(
        &'a self,
        binding: &VerifiedRouteBinding,
        model: Option<&str>,
    ) -> Result<(&'a JsonTarget, Option<&'a str>), StatusCode> {
        let (model, selected) = self.lookup(model)?;
        let (target, _) = self.snapshot.select(binding, Some(selected))?;
        Ok((target, Some(&model.upstream_model)))
    }
    pub(super) fn messages_api_key(
        &self,
        binding: &VerifiedRouteBinding,
        model: Option<&str>,
    ) -> Result<bool, StatusCode> {
        let (_, selected) = self.lookup(model)?;
        self.snapshot.messages_api_key(binding, Some(selected))
    }
    pub(super) fn native_messages(&self, model: Option<&str>) -> Result<bool, StatusCode> {
        let (row, _) = self.lookup(model)?;
        match self.protocols.get(&row.provider_id) {
            Some(GenerationProtocol::Messages) => Ok(true),
            Some(GenerationProtocol::Chat) => Ok(false),
            _ => Err(StatusCode::UNPROCESSABLE_ENTITY),
        }
    }
    pub(super) fn discovery(&self) -> Value {
        discovery(&self.catalog)
    }
}
impl RoutingListener {
    /// The caller holds the project/application write authorization. As with all
    /// publications, the private profile transaction runs before token activation.
    pub(crate) fn publish_desktop(
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
        let (binding, snapshot) = publication.validate()?;
        self.auth
            .publish_desktop(binding, snapshot, expected_revision, |token| commit(address, token))
    }
}

pub(super) fn request(body: &Value, native: bool) -> Result<Value, StatusCode> {
    if native { Ok(body.clone()) } else { crate::model_relay::messages_chat_request::to_chat(body) }
}
pub(super) fn headers(native: bool, outgoing: &mut axum::http::HeaderMap) {
    if native {
        outgoing.insert("anthropic-version", axum::http::HeaderValue::from_static("2023-06-01"));
    }
}

impl RoutingListener {
    pub(crate) fn desktop_revision(&self, binding: &VerifiedRouteBinding) -> Option<String> {
        self.auth.desktop_revision(binding)
    }
}

fn discovery(catalog: &DesktopModelCatalog) -> Value {
    let models = catalog.models();
    json!({"data":models.iter().map(|model|json!({"id":model.id,"type":"model","display_name":model.label,"description":model.label,"created_at":"2024-01-01T00:00:00Z","supports1m":model.supports_1m})).collect::<Vec<_>>(),
            "has_more":false,"first_id":models.first().map(|m|&m.id),"last_id":models.last().map(|m|&m.id)})
}
