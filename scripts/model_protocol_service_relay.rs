//! Private acceptance host for the public ACP production relay API.
//! Credentials and relay capabilities are read/written only inside the caller's root.
use anyhow::{Context, Result, ensure};
use serde::Deserialize;
use serde_json::json;
use std::{
    fs::OpenOptions, os::unix::fs::OpenOptionsExt, path::PathBuf, sync::Arc, time::Duration,
};
use vw_agent::{
    auth::{ApiInfo, Info},
    model_relay::{ProviderRelayBackend, RelayBinding, RelayIdentity, RelayServer},
    project::instance::Context as ProjectContext,
};
use vw_api_types::id::TenantScope;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Seed {
    provider_id: String,
    model_key: String,
    credential: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    let root =
        PathBuf::from(std::env::var_os("VIBEWINDOW_TEST_ROOT").context("private root required")?);
    ensure!(root.is_absolute() && root.is_dir(), "private root must exist and be absolute");
    let seed_file = root.join("seed.json");
    let seed: Seed = serde_json::from_slice(&std::fs::read(seed_file)?)?;
    let tenant = TenantScope::default();
    let project_root = root.join("project");
    std::fs::create_dir_all(&project_root)?;
    let (project, _) = vw_agent::project::from_directory(&tenant, &project_root).await?;
    vw_agent::auth::set(&seed.provider_id, &Info::Api(ApiInfo { key: seed.credential }))?;
    let context = ProjectContext {
        directory: project_root.to_string_lossy().into(),
        worktree: project.worktree.clone(),
        project: project.clone(),
    };
    let identity = RelayIdentity {
        tenant,
        project: project.id,
        session: "model-protocol-service:codex".into(),
        provider: seed.provider_id,
        model: seed.model_key,
        generation: 1,
    };
    let binding = RelayBinding::new(identity, Duration::from_secs(900))
        .map_err(|_| anyhow::anyhow!("relay binding failed"))?;
    let server =
        RelayServer::start(binding.clone(), Arc::new(ProviderRelayBackend::new()?), context)
            .await
            .map_err(|_| anyhow::anyhow!("relay startup failed"))?;
    let ready = root.join("ready.json");
    let staging = root.join("ready.tmp");
    let mut output = OpenOptions::new().write(true).create_new(true).mode(0o600).open(&staging)?;
    serde_json::to_writer(
        &mut output,
        &json!({"base_url":format!("http://{}",server.address()),"token":binding.bearer_token()}),
    )?;
    output.sync_all()?;
    std::fs::rename(staging, ready)?;
    let mut terminated = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())?;
    tokio::select! {
        _ = tokio::signal::ctrl_c() => {},
        _ = terminated.recv() => {},
    }
    server.shutdown().await.map_err(|_| anyhow::anyhow!("relay shutdown failed"))?;
    Ok(())
}
