//! Desktop projection for the shared provider-record publication executor.
use crate::publication_context::{PublicationContext as RecordPublicationContext, PublicationHost};
use {
    app_provider_services::AppProviderStore,
    app_routing_services::{
        RouteCommitError, SharedTransport, VerifiedRouteBinding,
        route_target::RouteTarget,
        desktop_routing::{DesktopPublication, DesktopRouteTarget},
        dispatch::JsonTarget,
    },
    model_protocol::generation::{GenerationProtocol, GenerationTransport},
};
use axum::http::{StatusCode, header};
use serde_json::Value;
use vw_agent_apps::providers::{
    claude_desktop::publication::{self, DesktopInstallation, DesktopTarget},
    desktop_mapping::{
        self,
        catalog::DesktopModelCatalog,
    },
    live::{
        commit::{AppState, ProviderTransactions},
        record_publication::ProviderRecordChange,
    },
};
fn unavailable<T>(_: T) -> StatusCode {
    StatusCode::SERVICE_UNAVAILABLE
}
fn invalid<T>(_: T) -> StatusCode {
    StatusCode::UNPROCESSABLE_ENTITY
}
#[allow(clippy::too_many_arguments)]
pub fn replace(
    context: &RecordPublicationContext,
    host: &impl PublicationHost,
    tx: &ProviderTransactions,
    store: &AppProviderStore,
    state: &AppState,
    row: &Value,
    change: &ProviderRecordChange,
) -> Result<bool, StatusCode> {
    if context.app != vw_api_app_identity::AppId::ClaudeDesktop || state.stack.enabled {
        return Err(StatusCode::UNPROCESSABLE_ENTITY);
    }
    let routing = context.routing.as_ref().ok_or(StatusCode::SERVICE_UNAVAILABLE)?;
    let id = &change.provider_id;
    let mapping = desktop_mapping::form_view(row).map_err(invalid)?.mapping;
    let catalog = || DesktopModelCatalog::new(id, &mapping).map_err(invalid);
    let listener_catalog = catalog()?;
    let catalog = catalog()?;
    let binding = VerifiedRouteBinding::new(
        context.project.clone(),
        context.host.clone(),
        context.app,
        "saved-aggregation".into(),
        1,
    );
    let previous = routing.desktop_revision(&binding);
    let revision = uuid::Uuid::new_v4().to_string();
    let target = || -> Result<DesktopRouteTarget, StatusCode> {
        let profile = app_provider_services::generation_target::SavedProfile::read_for_host(
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
            route: RouteTarget {
                messages_api_key: profile.protocol == GenerationProtocol::Messages
                    && !profile.messages_bearer,
                binding: binding.clone(),
                revision: revision.clone(),
                provider_id: id.clone(),
                media_settings: RouteTarget::media_settings_for(row),
                target: JsonTarget {
                    #[cfg(target_os = "macos")]
                    managed: None,
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
        state.clone(),
        listener_catalog,
        target()?,
    )?;
    routing.publish_desktop(publication, previous.as_deref(), |address, token| {
        let gateway = vw_api_app_provider_profiles::claude_desktop::ClaudeDesktopProfile {
            name: "Gateway".into(),
            config: vw_api_app_provider_profiles::claude_desktop::ClaudeDesktopConfig::ThirdParty {
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
            },
            || validate_inputs(context, host, &change.provider_id).is_ok(),
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

fn validate_inputs(context: &RecordPublicationContext, host: &impl PublicationHost, provider: &str)
    -> Result<(), vw_agent_apps::providers::live::write_guard::WriteError> {
    use vw_agent_apps::providers::live::write_guard::WriteError;
    for candidate in [Some(provider), None] {
        let current = host.revision(context, candidate).map_err(|_| WriteError::Conflict)?;
        if current == context.revision { return Ok(()); }
    }
    Err(WriteError::Conflict)
}
