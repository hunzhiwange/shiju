//! Desktop projection for the shared provider-record publication executor.
use super::aggregation_resync::RecordPublicationContext;
use crate::{
    app_providers::AppProviderStore,
    app_routing::{
        RouteCommitError, SharedTransport, VerifiedRouteBinding,
        aggregation_routing::AggregationTarget,
        desktop_routing::{DesktopPublication, DesktopRouteTarget},
        dispatch::JsonTarget,
    },
    model_relay::generation::{GenerationProtocol, GenerationTransport},
};
use axum::http::{StatusCode, header};
use serde_json::Value;
use vw_agent_apps::providers::{
    claude_desktop::publication::{self, DesktopInstallation, DesktopTarget},
    desktop_mapping::{
        self,
        catalog::{DesktopMappingMember, DesktopModelCatalog},
    },
    live::{
        commit::{AppState, PendingTarget, ProviderTransactions},
        record_publication::ProviderRecordChange,
        write_guard::lock_app,
    },
};
fn unavailable<T>(_: T) -> StatusCode {
    StatusCode::SERVICE_UNAVAILABLE
}
fn invalid<T>(_: T) -> StatusCode {
    StatusCode::UNPROCESSABLE_ENTITY
}
#[allow(clippy::too_many_arguments)]
pub(crate) fn replace(
    context: &RecordPublicationContext,
    tx: &ProviderTransactions,
    store: &AppProviderStore,
    state: &AppState,
    candidate: &AppState,
    rows: &[(String, Value)],
    change: &ProviderRecordChange,
    target: PendingTarget,
    active: bool,
) -> Result<bool, StatusCode> {
    if context.app != vw_api_types::agent_apps::AppId::ClaudeDesktop {
        return Err(StatusCode::UNPROCESSABLE_ENTITY);
    }
    let live_member = if candidate.stack.enabled {
        candidate.stack.members.iter().any(|id| id == &change.provider_id)
    } else {
        candidate.mode.as_ref().and_then(|mode| mode.proxy_route.as_deref())
            == Some(change.provider_id.as_str())
    };
    if !active || !live_member {
        if !active && state.mode.as_ref().is_some_and(|mode| mode.attached) {
            return Err(StatusCode::UNPROCESSABLE_ENTITY);
        }
        let mut guard = lock_app(context.app, &context.home).map_err(unavailable)?;
        tx.commit_project_record(
            &context.project,
            &mut guard,
            state,
            &[],
            target,
            change,
            store,
            || validate_inputs(context, &change.provider_id),
        )
        .map_err(|error| match error {
            vw_agent_apps::providers::live::write_guard::WriteError::Io => StatusCode::BAD_GATEWAY,
            vw_agent_apps::providers::live::write_guard::WriteError::InvalidResource => {
                StatusCode::UNPROCESSABLE_ENTITY
            }
            _ => StatusCode::CONFLICT,
        })?;
        return Ok(true);
    }
    let routing = context.routing.as_ref().ok_or(StatusCode::SERVICE_UNAVAILABLE)?;
    let default = candidate
        .mode
        .as_ref()
        .and_then(|mode| mode.proxy_route.as_deref())
        .ok_or(StatusCode::CONFLICT)?;
    let rows = if candidate.stack.enabled {
        rows
    } else {
        std::slice::from_ref(rows.iter().find(|(id, _)| id == default).ok_or(StatusCode::CONFLICT)?)
    };
    let mappings = rows
        .iter()
        .map(|(_, row)| desktop_mapping::form_view(row).map(|form| form.mapping).map_err(invalid))
        .collect::<Result<Vec<_>, _>>()?;
    let default_index =
        rows.iter().position(|(id, _)| id == default).ok_or(StatusCode::CONFLICT)?;
    let member = |index: usize| -> Result<DesktopMappingMember<'_>, StatusCode> {
        let (id, row) = &rows[index];
        let key = candidate
            .stack
            .keys
            .iter()
            .find_map(|(key, owner)| (owner == id).then_some(key.as_str()))
            .ok_or(StatusCode::CONFLICT)?;
        Ok(DesktopMappingMember {
            provider_id: id,
            key,
            name: row["name"].as_str().unwrap_or(id),
            mapping: &mappings[index],
        })
    };
    let members = rows
        .iter()
        .enumerate()
        .filter(|(index, _)| *index != default_index)
        .map(|(index, _)| member(index))
        .collect::<Result<Vec<_>, _>>()?;
    let catalog = DesktopModelCatalog::new(member(default_index)?, &members).map_err(invalid)?;
    let listener_catalog =
        DesktopModelCatalog::new(member(default_index)?, &members).map_err(invalid)?;
    let binding = VerifiedRouteBinding::new(
        context.project.clone(),
        context.host.clone(),
        context.app,
        "saved-aggregation".into(),
        1,
    );
    let previous = routing.desktop_revision(&binding);
    let revision = uuid::Uuid::new_v4().to_string();
    let target = |(id, row): &(String, Value)| -> Result<DesktopRouteTarget, StatusCode> {
        let profile = crate::app_providers::generation_target::SavedProfile::read_for_host(
            context.app,
            &context.host,
            &id.clone().into(),
            row,
        )
        .map_err(invalid)?;
        if !matches!(profile.protocol, GenerationProtocol::Chat | GenerationProtocol::Messages) {
            return Err(StatusCode::UNPROCESSABLE_ENTITY);
        }
        let url = profile.protocol.url(
            &profile.endpoint,
            "route",
            row.pointer("/meta/isFullUrl") == Some(&Value::Bool(true)),
        )?;
        let headers = profile
            .protocol
            .headers(&profile.credential, profile.messages_bearer)
            .ok_or(StatusCode::UNPROCESSABLE_ENTITY)?;
        let authorization = headers
            .get(header::AUTHORIZATION)
            .or_else(|| headers.get("x-api-key"))
            .cloned()
            .ok_or(StatusCode::UNPROCESSABLE_ENTITY)?;
        let request_options = row
            .get("requestOptions")
            .cloned()
            .map(serde_json::from_value)
            .transpose()
            .map_err(invalid)?
            .unwrap_or_default();
        Ok(DesktopRouteTarget {
            protocol: profile.protocol,
            route: AggregationTarget {
                messages_api_key: profile.protocol == GenerationProtocol::Messages
                    && !profile.messages_bearer,
                binding: binding.clone(),
                revision: revision.clone(),
                provider_id: id.clone(),
                target: JsonTarget {
                    url,
                    authorization,
                    transport: SharedTransport::new().map_err(unavailable)?,
                    request_options,
                },
            },
        })
    };
    let publication = DesktopPublication::new(
        binding.clone(),
        revision.clone(),
        candidate.clone(),
        listener_catalog,
        target(&rows[default_index])?,
        rows.iter().filter(|(id, _)| id != default).map(target).collect::<Result<Vec<_>, _>>()?,
    )?;
    routing.publish_desktop(publication, previous.as_deref(), |address, token| {
        let gateway = vw_api_types::app_providers::claude_desktop::ClaudeDesktopProfile {
            name: "Gateway".into(),
            config: vw_api_types::app_providers::claude_desktop::ClaudeDesktopConfig::ThirdParty {
                base_url: format!("http://{address}/claude-desktop"),
                auth_token: token.into(),
            },
        };
        publication::run_record(
            &context.home,
            DesktopInstallation::Installed,
            tx,
            context.project.tenant_scope(),
            state,
            DesktopTarget::Route {
                gateway: &gateway,
                catalog: &catalog,
                stack: Some(candidate.stack.clone()),
            },
            || validate_inputs(context, &change.provider_id).is_ok(),
            change,
            store,
        )
        .map_err(|error| RouteCommitError {
            status: match error {
                publication::DesktopPublicationError::Forbidden => StatusCode::FORBIDDEN,
                publication::DesktopPublicationError::RevisionConflict
                | publication::DesktopPublicationError::RecoveryPending => StatusCode::CONFLICT,
                publication::DesktopPublicationError::InvalidConfiguration
                | publication::DesktopPublicationError::UnsupportedTarget => {
                    StatusCode::UNPROCESSABLE_ENTITY
                }
                publication::DesktopPublicationError::TargetFailed => StatusCode::BAD_GATEWAY,
            },
            recovery_pending: tx
                .pending(context.app)
                .map_or(true, |pending| pending.is_some_and(|pending| pending.published))
                || tx.state(context.app).map_or(true, |current| current != *state),
        })
    })?;
    Ok(true)
}

fn validate_inputs(
    context: &RecordPublicationContext,
    provider: &str,
) -> Result<(), vw_agent_apps::providers::live::write_guard::WriteError> {
    use crate::gateway::api::handlers::aggregation_read_snapshot::{load, revision};
    use vw_agent_apps::providers::live::write_guard::WriteError;
    // Form revisions additionally include the selected record even while it is
    // not a stack member. Existing catalog writers use the common stack revision
    // plus the participant's exact record preimage CAS.
    for candidate in [Some(provider), None] {
        let snapshot = load(
            &context.database,
            &context.project,
            &context.host,
            context.app,
            &context.home,
            &context.expected,
            candidate,
        )
        .map_err(|_| WriteError::Conflict)?;
        let current = revision(
            &context.project,
            &context.host,
            context.app,
            &context.principal,
            &context.expected,
            &snapshot,
        )
        .map_err(|_| WriteError::Conflict)?;
        if current == context.revision {
            return Ok(());
        }
    }
    Err(WriteError::Conflict)
}
