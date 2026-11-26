use std::{io::Read, path::PathBuf, sync::Arc};

use llm_runtime::acp::model_source::{self, profile::AcpLaunchProfile};

use super::*;
use crate::model_relay::{ProviderRelayBackend, RelayServer};

const ENGINE: &str = "@openai/codex-darwin-arm64/vendor/aarch64-apple-darwin/bin/codex";
// The native binary from @openai/codex@0.165.0-darwin-arm64.
const ENGINE_SHA256: &str = "a45099154d7fb4a5ed8a56f40966a95deb938e508bab23e6985035c709e9a85a";

fn installed_runtime() -> Result<(String, PathBuf), WorkbenchError> {
    let (adapter, _) = acp_agent_registry::reviewed_agent_command("codex")
        .ok()
        .flatten()
        .ok_or_else(unavailable)?;
    let modules =
        FsPath::new(&adapter).parent().and_then(FsPath::parent).ok_or_else(unavailable)?;
    let executable = modules.join(ENGINE);
    if executable.canonicalize().ok().as_deref() != Some(executable.as_path()) {
        return Err(unavailable());
    }
    let mut file = std::fs::File::open(&executable).map_err(|_| unavailable())?;
    let mut digest = Sha256::new();
    let mut buffer = [0; 32 * 1024];
    loop {
        let read = file.read(&mut buffer).map_err(|_| unavailable())?;
        if read == 0 {
            break;
        }
        digest.update(&buffer[..read]);
    }
    if hex::encode(digest.finalize()) != ENGINE_SHA256 {
        return Err(unavailable());
    }
    Ok((adapter, executable))
}

pub(super) async fn prepare(
    scope: &ProjectScope,
    directory: &str,
    id: &str,
    model: &str,
) -> Result<(PathBuf, AcpLaunchProfile), WorkbenchError> {
    let invalid_model = || {
        error(
            StatusCode::BAD_REQUEST,
            ErrorCode::InvalidRequest,
            "gateway model is unavailable for Codex",
        )
    };
    let (provider, model) = model.split_once('/').ok_or_else(invalid_model)?;
    if provider.is_empty()
        || model.is_empty()
        || provider == "acp"
        || provider.chars().chain(model.chars()).any(|c| c.is_whitespace() || c.is_control())
    {
        return Err(invalid_model());
    }
    let (adapter, executable) =
        tokio::task::spawn_blocking(installed_runtime).await.map_err(|_| unavailable())??;
    let tenant = scope.tenant_scope().clone();
    let project_id = scope.project_id().as_ref().to_owned();
    let (provider, model, id) = (provider.to_owned(), model.to_owned(), id.to_owned());
    let directory = directory.to_owned();
    let profile = Host::with_tenant_instance(tenant.clone(), directory.clone(), move || {
        Box::pin(async move {
            let project = agent_project::instance::project()
                .filter(|project| project.id == project_id)
                .ok_or_else(|| gateway_types::ApiError::not_found("project context changed"))?;
            let resolved = crate::provider::provider::get_model(&provider, &model)
                .await
                .map_err(|_| gateway_types::ApiError::bad_request("gateway model unavailable"))?;
            if resolved.provider_id != provider || resolved.id != model {
                return Err(gateway_types::ApiError::bad_request("gateway model unavailable"));
            }
            let context = agent_project::instance::Context {
                directory: directory.clone(),
                worktree: agent_project::instance::worktree(),
                project,
            };
            let config =
                crate::config::schema::AcpAgentConfig { command: adapter, ..Default::default() };
            let identity =
                model_source::launch_identity(tenant, project_id, "codex", &resolved, &id);
            let mut profile =
                model_source::prepare_profile("codex", &config, &resolved, identity, || {
                    agent_paths::paths().state.clone()
                })
                .map_err(|_| gateway_types::ApiError::bad_request("gateway model unavailable"))?;
            let backend = ProviderRelayBackend::new()
                .map_err(|_| gateway_types::ApiError::bad_request("model relay unavailable"))?;
            let relay = RelayServer::start(profile.binding.clone(), Arc::new(backend), context)
                .await
                .map_err(|_| gateway_types::ApiError::bad_request("model relay unavailable"))?;
            let endpoint = format!("http://{}", relay.address());
            profile.attach_server(relay);
            model_source::configure_profile(
                "codex",
                &mut profile,
                &endpoint,
                &directory,
                &Default::default(),
            )
            .await
            .map_err(|_| gateway_types::ApiError::bad_request("gateway model unavailable"))?;
            Ok(profile)
        })
    })
    .await
    .map_err(|_| invalid_model())?;
    Ok((executable, profile))
}
