//! Resolve the probe destination and account from the execution host's saved state.

use reqwest::{Url, header};
use serde_json::Value;
use sha2::{Digest, Sha256};
use vw_agent_apps::{
    auth::{copilot::CopilotAuth, flow::AuthAccountScope},
    models::edits,
    providers::validation,
};
use vw_api_types::{
    agent_apps::AppId,
    agent_apps_auth::{AuthErrorCode, AuthService},
    agent_apps_diagnostics::DiagnosticFailureCode,
    id::{ModelId, ProviderId},
    model_catalog::ModelCatalogIdentity,
};

use super::{DiagnosticJobOwner, GlobalDatabase};
use crate::{
    agent_apps::auth_accounts::{AuthAccountMetadata, AuthAccountStore},
    app_providers::AppProviderStore,
    global_database::run_sync_database,
    model_catalog::ModelCatalogStore,
    model_relay::generation::{GenerationProtocol, GenerationTransport},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModelPrepareError {
    NotFound,
    Unavailable,
    Unsupported,
    Account(AuthErrorCode),
}

type Result<T> = std::result::Result<T, ModelPrepareError>;

pub(in crate::gateway::agent_apps_diagnostic_executors) struct ModelTarget {
    database: GlobalDatabase,
    owner: DiagnosticJobOwner,
    revision: [u8; 32],
    pub(in crate::gateway::agent_apps_diagnostic_executors) provider_id: ProviderId,
    pub(in crate::gateway::agent_apps_diagnostic_executors) model_id: ModelId,
    pub(in crate::gateway::agent_apps_diagnostic_executors) protocol: GenerationProtocol,
    destination: Destination,
}

enum Destination {
    Saved { url: Url, headers: header::HeaderMap },
    Copilot(Box<CopilotBinding>),
}

struct CopilotBinding {
    account_id: String,
    metadata: AuthAccountMetadata,
    credential_generation: [u8; 32],
}

impl ModelTarget {
    pub(in crate::gateway::agent_apps_diagnostic_executors) fn prepare(
        database: &GlobalDatabase,
        accounts: &AuthAccountStore,
        owner: DiagnosticJobOwner,
        provider_id: ProviderId,
        model_id: ModelId,
    ) -> Result<Self> {
        edits::validate_model_id(&model_id).map_err(|_| ModelPrepareError::Unsupported)?;
        let row = record(database, &owner, &provider_id)?;
        let app = owner.scope.app_id;
        let configured =
            super::super::reachability::configured_url(&owner, provider_id.as_ref(), &row);
        let (endpoint, configured_model, protocol, destination) =
            if super::super::reachability::is_copilot(&row, configured.as_deref()) {
                let endpoint = configured.ok_or(ModelPrepareError::Unsupported)?;
                validation::endpoint(&endpoint).map_err(|_| ModelPrepareError::Unsupported)?;
                if row.pointer("/meta/isFullUrl") == Some(&Value::Bool(true)) {
                    return Err(ModelPrepareError::Unsupported);
                }
                let account_id = super::super::reachability::copilot_binding(&row)
                    .map_err(|_| ModelPrepareError::Account(AuthErrorCode::InvalidInput))?
                    .ok_or(ModelPrepareError::Account(AuthErrorCode::NotFound))?
                    .to_owned();
                let scope = copilot_scope(&owner);
                let (metadata, credential_generation) = accounts
                    .copilot_diagnostic_generation(&scope, &account_id)
                    .map_err(ModelPrepareError::Account)?;
                let format = match app {
                    AppId::Codex => row.pointer("/meta/codexCopilotApiFormat"),
                    AppId::Claude => row.pointer("/meta/apiFormat"),
                    _ => return Err(ModelPrepareError::Unsupported),
                };
                let protocol = match format.and_then(Value::as_str) {
                    Some("openai_chat") => GenerationProtocol::Chat,
                    Some("openai_responses") => GenerationProtocol::Responses,
                    _ => return Err(ModelPrepareError::Unsupported),
                };
                // Managed deployments authorize models through the scoped catalog.
                let destination = Destination::Copilot(Box::new(CopilotBinding {
                    account_id,
                    metadata,
                    credential_generation,
                }));
                (endpoint, None, protocol, destination)
            } else {
                let profile = crate::app_providers::generation_target::SavedProfile::read_for_host(
                    app,
                    &owner.scope.target_host_id,
                    &provider_id,
                    &row,
                )
                .map_err(|_| ModelPrepareError::Unsupported)?;
                let protocol = profile.protocol;
                let full_url = row.pointer("/meta/isFullUrl") == Some(&Value::Bool(true));
                let url = protocol
                    .url(&profile.endpoint, model_id.as_ref(), full_url)
                    .map_err(|_| ModelPrepareError::Unsupported)?;
                let headers = protocol
                    .headers(&profile.credential, profile.messages_bearer)
                    .ok_or(ModelPrepareError::Account(AuthErrorCode::InvalidInput))?;
                (profile.endpoint, profile.model, protocol, Destination::Saved { url, headers })
            };

        let deployment = ModelCatalogIdentity {
            target_host_id: owner.scope.target_host_id.clone(),
            app_id: app,
            provider_id: provider_id.as_ref().to_owned(),
            endpoint,
        };
        let catalog = edits::read(owner.scope.project.tenant_scope(), &deployment, &row)
            .map_err(|_| ModelPrepareError::Unavailable)?;
        let native_catalog = if app == AppId::Codex && row.get("codex").is_some() {
            let revision = match row.get("modelCatalogRevision") {
                None | Some(Value::Null) => None,
                Some(revision) => Some(revision.as_str().ok_or(ModelPrepareError::Unavailable)?),
            };
            ModelCatalogStore::new(database.clone())
                .and_then(|store| {
                    store.read(owner.scope.project.tenant_scope(), &deployment, revision)
                })
                .map_err(|_| ModelPrepareError::Unavailable)?
        } else {
            None
        };
        match catalog.items.iter().find(|item| item.identity.model_id == model_id) {
            Some(item)
                if item.metadata.input_modalities.as_ref().is_some_and(|modalities| {
                    !modalities.iter().any(|kind| kind.eq_ignore_ascii_case("text"))
                }) =>
            {
                return Err(ModelPrepareError::Unsupported);
            }
            Some(_) => {}
            None if native_catalog.as_ref().is_some_and(|catalog| {
                catalog.models.iter().any(|model| model.slug == model_id.as_ref())
            }) => {}
            None if configured_model.as_deref() == Some(model_id.as_ref()) => {}
            None => return Err(ModelPrepareError::NotFound),
        }
        let revision = revision(&row)?;
        Ok(Self {
            database: database.clone(),
            owner,
            revision,
            provider_id,
            model_id,
            protocol,
            destination,
        })
    }

    pub(in crate::gateway::agent_apps_diagnostic_executors) fn check_current(
        &self,
        accounts: &AuthAccountStore,
    ) -> std::result::Result<(), DiagnosticFailureCode> {
        let valid = run_sync_database(|| Ok(self.is_current(accounts)))
            .map_err(|_| DiagnosticFailureCode::Transport)?
            .map_err(ModelPrepareError::failure)?;
        if !valid {
            return Err(DiagnosticFailureCode::Unauthorized);
        }
        Ok(())
    }

    fn is_current(&self, accounts: &AuthAccountStore) -> Result<bool> {
        let row = record(&self.database, &self.owner, &self.provider_id)?;
        if revision(&row)? != self.revision {
            return Ok(false);
        }
        if let Destination::Copilot(binding) = &self.destination {
            let (metadata, generation) = accounts
                .copilot_diagnostic_generation(&copilot_scope(&self.owner), &binding.account_id)
                .map_err(ModelPrepareError::Account)?;
            return Ok(binding.metadata == metadata && binding.credential_generation == generation);
        }
        Ok(true)
    }

    pub(in crate::gateway::agent_apps_diagnostic_executors) async fn destination(
        &self,
        accounts: &AuthAccountStore,
        copilot: &CopilotAuth,
    ) -> std::result::Result<(Url, header::HeaderMap), DiagnosticFailureCode> {
        match &self.destination {
            Destination::Saved { url, headers } => Ok((url.clone(), headers.clone())),
            Destination::Copilot(binding) => {
                let session = accounts
                    .copilot_session(copilot, copilot_scope(&self.owner), &binding.account_id)
                    .await
                    .map_err(|error| ModelPrepareError::Account(error).failure())?;
                if !session.matches_credential_generation(&binding.credential_generation) {
                    return Err(DiagnosticFailureCode::Unauthorized);
                }
                let path = match self.protocol {
                    GenerationProtocol::Chat => "/chat/completions",
                    GenerationProtocol::Responses => "/responses",
                    _ => return Err(DiagnosticFailureCode::Unsupported),
                };
                let mut url = session.endpoint;
                url.set_path(path);
                let mut headers = header::HeaderMap::new();
                headers.insert(header::AUTHORIZATION, session.authorization);
                Ok((url, headers))
            }
        }
    }
}

impl ModelPrepareError {
    fn failure(self) -> DiagnosticFailureCode {
        match self {
            Self::Unsupported | Self::Account(AuthErrorCode::Unsupported) => {
                DiagnosticFailureCode::Unsupported
            }
            Self::Unavailable
            | Self::Account(
                AuthErrorCode::StorageUnavailable
                | AuthErrorCode::ExecutionHostUnavailable
                | AuthErrorCode::UpstreamFailure,
            ) => DiagnosticFailureCode::Transport,
            Self::NotFound | Self::Account(_) => DiagnosticFailureCode::Unauthorized,
        }
    }
}

fn record(
    database: &GlobalDatabase,
    owner: &DiagnosticJobOwner,
    provider_id: &ProviderId,
) -> Result<Value> {
    AppProviderStore::preview_record(
        database,
        owner.scope.project.tenant_scope(),
        owner.scope.app_id,
        provider_id.as_ref(),
    )
    .map_err(|error| match error.as_str() {
        "application provider not found" => ModelPrepareError::NotFound,
        "application provider storage unavailable" => ModelPrepareError::Unavailable,
        _ => ModelPrepareError::Unsupported,
    })
}

fn revision(row: &Value) -> Result<[u8; 32]> {
    serde_json::to_vec(row)
        .map(|bytes| Sha256::digest(bytes).into())
        .map_err(|_| ModelPrepareError::Unavailable)
}

fn copilot_scope(owner: &DiagnosticJobOwner) -> AuthAccountScope<'_> {
    AuthAccountScope {
        tenant: owner.scope.project.tenant_scope(),
        principal: &owner.principal,
        host: &owner.scope.target_host_id,
        service: AuthService::GithubCopilot,
    }
}
