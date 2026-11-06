//! Identity projections from formal tenant owners, authorized for the selected execution host.
use super::launch::Context;
use crate::{
    agent_apps::auth_accounts::AuthAccountStore, app_providers::AppProviderStore,
    global_database::GlobalDatabase,
};
use axum::{
    Extension, Json,
    extract::Query,
    http::{HeaderMap, StatusCode},
};
use serde::Deserialize;
use vw_api_types::{agent_apps::*, id::TenantScope};
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct Target {
    target_host_id: String,
    app_id: AppId,
}
pub(super) async fn list(
    Extension(context): Extension<Context>,
    Extension(tenant): Extension<TenantScope>,
    database: Option<Extension<GlobalDatabase>>,
    headers: HeaderMap,
    Query(target): Query<Target>,
) -> Result<Json<Vec<LaunchIdentity>>, StatusCode> {
    tokio::task::spawn_blocking(move || {
        let authorization = super::access::authorize_identity_launch(
            context.policy.as_deref(),
            &headers,
            &tenant,
            &target.target_host_id,
        )?;
        let host = &authorization.host;
        let database = match database {
            Some(Extension(db)) => db,
            None => GlobalDatabase::for_home(&crate::global::paths().home)
                .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?,
        };
        let mut identities: Vec<_> = AppProviderStore::new(database.clone())
            .and_then(|store| store.list(&tenant, target.app_id))
            .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?
            .into_iter()
            .filter(|provider| {
                authorization.permits(target.app_id, LaunchIdentityKind::Provider, &provider.id)
            })
            .map(|provider| LaunchIdentity {
                kind: LaunchIdentityKind::Provider,
                id: provider.id,
                label: provider.name,
            })
            .collect();
        if let Some(root) = &host.credential_root {
            let accounts = AuthAccountStore::new(database, root)
                .and_then(|store| store.list(&tenant, &host.id, target.app_id))
                .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
            identities.extend(
                accounts
                    .into_iter()
                    .filter(|id| {
                        authorization.permits(
                            target.app_id,
                            LaunchIdentityKind::OfficialAccount,
                            id,
                        )
                    })
                    .map(|id| LaunchIdentity {
                        kind: LaunchIdentityKind::OfficialAccount,
                        label: id.clone(),
                        id,
                    }),
            );
        }
        Ok(Json(identities))
    })
    .await
    .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?
}
