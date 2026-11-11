//! Immutable model and target publication for the application listener.

use super::{dispatch::JsonTarget, route_auth::VerifiedRouteBinding};
use crate::model_relay::upstream::validate_target as validate_upstream;
use axum::http::StatusCode;
use std::collections::{HashMap, HashSet};
use vw_agent_apps::{
    aggregation::resolve::{PublishedModel, ResolveError, Resolved, resolve},
    providers::{live::commit::AppState, mode_state::observe, validation},
};
use vw_api_types::agent_apps::AppId;

pub(crate) struct AggregationModel {
    pub(crate) id: String,
    pub(crate) provider_id: String,
    pub(crate) upstream_model: String,
}

impl AggregationModel {
    fn published(&self) -> PublishedModel<'_> {
        PublishedModel {
            id: &self.id,
            provider_id: &self.provider_id,
            upstream_model: &self.upstream_model,
        }
    }
}

/// The transaction resolves the target from the same authorized scope and revision.
pub(crate) struct AggregationTarget {
    pub(crate) messages_api_key: bool,
    pub(crate) binding: VerifiedRouteBinding,
    pub(crate) revision: String,
    pub(crate) provider_id: String,
    pub(crate) target: JsonTarget,
}

pub(crate) struct AggregationPublication {
    pub(crate) binding: VerifiedRouteBinding,
    pub(crate) revision: String,
    pub(crate) state: AppState,
    pub(crate) models: Vec<AggregationModel>,
    pub(crate) default: AggregationTarget,
    pub(crate) members: Vec<AggregationTarget>,
}

pub(super) struct AggregationSnapshot {
    pub(super) revision: String,
    state: AppState,
    models: Vec<AggregationModel>,
    default: String,
    members: HashMap<String, JsonTarget>,
    api_keys: HashSet<String>,
}

impl AggregationPublication {
    pub(super) fn validate(self) -> Result<AggregationSnapshot, StatusCode> {
        let app = self.binding.app();
        if !matches!(app, AppId::Codex | AppId::Claude) {
            return Err(StatusCode::UNPROCESSABLE_ENTITY);
        }
        if self.revision.trim().is_empty() {
            return Err(StatusCode::BAD_REQUEST);
        }
        let tenant = self.binding.project().tenant_scope();
        let mode = observe(&self.state, tenant).map_err(|_| StatusCode::FORBIDDEN)?;
        if !mode.stack.active || !mode.attached {
            return Err(StatusCode::CONFLICT);
        }
        let validate_target = |target: &AggregationTarget| {
            if target.binding != self.binding {
                return Err(StatusCode::FORBIDDEN);
            }
            if target.revision != self.revision {
                return Err(StatusCode::CONFLICT);
            }
            let url = &target.target.url;
            validate_upstream(url, &reqwest::Request::new(reqwest::Method::POST, url.clone()))?;
            validation::validate(app, &target.target.request_options)
                .map_err(|_| StatusCode::UNPROCESSABLE_ENTITY)
        };
        validate_target(&self.default)?;
        if mode.proxy_route.as_deref() != Some(&self.default.provider_id) {
            return Err(StatusCode::CONFLICT);
        }

        let mut members = HashMap::with_capacity(self.members.len());
        let mut api_keys = HashSet::new();
        if self.default.messages_api_key {
            api_keys.insert(self.default.provider_id.clone());
        }
        for member in self.members {
            validate_target(&member)?;
            if member.messages_api_key {
                api_keys.insert(member.provider_id.clone());
            }
            if !self.state.stack.members.contains(&member.provider_id)
                || member.provider_id == self.default.provider_id
                || members.insert(member.provider_id, member.target).is_some()
            {
                return Err(StatusCode::UNPROCESSABLE_ENTITY);
            }
        }
        let mut ids = HashSet::with_capacity(self.models.len());
        for model in &self.models {
            if !ids.insert(&model.id)
                || !(members.contains_key(&model.provider_id)
                    || model.provider_id == self.default.provider_id)
            {
                return Err(StatusCode::UNPROCESSABLE_ENTITY);
            }
            if !matches!(
                resolve(app, tenant, &self.state, &model.id, [model.published()]),
                Ok(Resolved::Exact(_))
            ) {
                return Err(StatusCode::UNPROCESSABLE_ENTITY);
            }
        }
        if self
            .state
            .stack
            .members
            .iter()
            .any(|member| member != &self.default.provider_id && !members.contains_key(member))
        {
            return Err(StatusCode::UNPROCESSABLE_ENTITY);
        }

        // The default can also have an explicitly published ID.
        members.insert(self.default.provider_id, self.default.target);
        let default = mode.proxy_route.ok_or(StatusCode::CONFLICT)?;
        Ok(AggregationSnapshot {
            revision: self.revision,
            state: self.state,
            models: self.models,
            default,
            members,
            api_keys,
        })
    }
}

impl AggregationSnapshot {
    pub(super) fn messages_api_key(
        &self,
        binding: &VerifiedRouteBinding,
        model: Option<&str>,
    ) -> Result<bool, StatusCode> {
        let provider = match resolve(
            binding.app(),
            binding.project().tenant_scope(),
            &self.state,
            model.unwrap_or_default(),
            self.models.iter().map(AggregationModel::published),
        )
        .map_err(resolve_status)?
        {
            Resolved::Plain => self.default.as_str(),
            Resolved::Exact(row) => row.provider_id,
        };
        Ok(self.api_keys.contains(provider))
    }
    pub(super) fn select<'a>(
        &'a self,
        binding: &VerifiedRouteBinding,
        model: Option<&str>,
    ) -> Result<(&'a JsonTarget, Option<&'a str>), StatusCode> {
        let (provider, upstream) = match resolve(
            binding.app(),
            binding.project().tenant_scope(),
            &self.state,
            model.unwrap_or_default(),
            self.models.iter().map(AggregationModel::published),
        )
        .map_err(resolve_status)?
        {
            Resolved::Plain => (self.default.as_str(), None),
            Resolved::Exact(row) => (row.provider_id, Some(row.upstream_model)),
        };
        let target = self.members.get(provider).ok_or(StatusCode::BAD_GATEWAY)?;
        Ok((target, upstream))
    }
}

fn resolve_status(error: ResolveError) -> StatusCode {
    match error {
        ResolveError::InvalidModelId => StatusCode::BAD_REQUEST,
        ResolveError::Forbidden => StatusCode::FORBIDDEN,
        ResolveError::UnknownMember
        | ResolveError::RemovedMember
        | ResolveError::UnpublishedModel => StatusCode::NOT_FOUND,
        ResolveError::Inactive | ResolveError::UnsupportedApplication => {
            StatusCode::UNPROCESSABLE_ENTITY
        }
        ResolveError::InvalidCatalog => StatusCode::CONFLICT,
    }
}
