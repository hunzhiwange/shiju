//! Runtime-only CLI model launch boundary; local CLI configuration bypasses this module.
pub(super) mod drivers;
mod gateway;
mod mimo_mcp;
pub(crate) mod profile;
use crate::{
    config::schema::AcpAgentConfig,
    model_relay::{ProviderRelayBackend, RelayIdentity, RelayServer},
    project::instance,
    provider::provider,
};
use profile::AcpLaunchProfile;

#[cfg(any(test, target_arch = "wasm32"))]
pub(super) use llm_runtime::acp::model_source::fingerprint;

pub(super) async fn prepare(
    agent: &str,
    config: &AcpAgentConfig,
    model: &provider::Model,
    session: &str,
    openclaw_mcp: &serde_json::Map<String, serde_json::Value>,
) -> anyhow::Result<AcpLaunchProfile> {
    prepare_with_mounts(agent, config, model, session, openclaw_mcp, None).await
}

pub(super) async fn prepare_stdio_mcp(
    agent: &str,
    config: &AcpAgentConfig,
    model: &provider::Model,
    session: &str,
    cwd: &std::path::Path,
    servers: &[acp_types::AcpMcpServer],
) -> anyhow::Result<AcpLaunchProfile> {
    prepare_with_mounts(agent, config, model, session, &Default::default(), Some((cwd, servers))).await
}

async fn prepare_with_mounts(
    agent: &str,
    config: &AcpAgentConfig,
    model: &provider::Model,
    session: &str,
    openclaw_mcp: &serde_json::Map<String, serde_json::Value>,
    mounts: Option<(&std::path::Path, &[acp_types::AcpMcpServer])>,
) -> anyhow::Result<AcpLaunchProfile> {
    let project = instance::project()
        .ok_or_else(|| anyhow::anyhow!("ACP launch requires project context"))?;
    let context = instance::Context {
        directory: instance::directory(),
        worktree: instance::worktree(),
        project,
    };
    // Resolve the current catalog on every issue/reissue; persisted launch records
    // cannot supply a Provider snapshot or resurrect removed models.
    let resolved = provider::get_model(&model.provider_id, &model.id).await?;
    let identity = launch_identity(context.project.id.clone(), agent, model, session);
    let mut copilot_launch = None;
    if agent == "copilot" {
        use llm_runtime::acp::model_source::drivers::copilot::PROJECT_SKILLS_ENV;
        if let Some(grant) = config.env.get(PROJECT_SKILLS_ENV) {
            let tenant = crate::session::ui_store::current_tenant_scope();
            let tenant_config = crate::config::get_global_for_tenant_without_env(&tenant).await?;
            anyhow::ensure!(
                tenant_config.acp.get("copilot")
                    .and_then(|entry| entry.env.get(PROJECT_SKILLS_ENV)) == Some(grant),
                "Copilot native skills require a tenant-owned project grant"
            );
            let project = std::path::Path::new(&context.directory).canonicalize()?;
            if std::path::Path::new(grant) != project {
                let mut launch = config.clone();
                launch.env.remove(PROJECT_SKILLS_ENV);
                copilot_launch = Some(launch);
            }
        }
    }
    let config = copilot_launch.as_ref().unwrap_or(config);
    let state_root = || std::env::var_os("VIBEWINDOW_CONFIG_DIR").map(std::path::PathBuf::from)
        .unwrap_or_else(|| crate::global::paths().state.clone());
    let mut profile = llm_runtime::acp::model_source::prepare_profile(agent, config, &resolved, identity, state_root)?;
    let directory = match mounts {
        Some((cwd, _)) => cwd.to_str().ok_or_else(|| anyhow::anyhow!("Non-UTF-8 MCP workspace"))?.to_owned(),
        None => context.directory.clone(),
    };
    if agent == "vtcode" && mounts.is_some() {
        acp_agent_registry::verify_vtcode_mcp_executable(&config.command).map_err(anyhow::Error::msg)?;
    }
    let (projection, review_is_current) = match mounts {
        Some((_, servers)) => {
            prepare_mcp_projection(agent, servers, &directory, profile.binding.clone()).await?
        }
        None => (None, None),
    };
    let mut routes = projection.as_ref()
        .map_or_else(axum::Router::new, |projection| projection.router.clone());
    if let Some(current) = review_is_current.clone() {
        routes = routes.layer(axum::middleware::from_fn(
            move |request: axum::extract::Request, next: axum::middleware::Next| {
                let current = current.clone();
                async move {
                    if current() {
                        next.run(request).await
                    } else {
                        axum::response::IntoResponse::into_response(axum::http::StatusCode::FORBIDDEN)
                    }
                }
            },
        ));
    }
    let catalog = projection.as_ref().filter(|_| agent == "vtcode")
        .map(|projection| projection.tools.clone()).unwrap_or_default();
    let backend = std::sync::Arc::new(ProviderRelayBackend::new()?);
    #[cfg(target_os = "linux")]
    let server = if agent == "vtcode" {
        // The launch and mount validators above must have accepted the fixed
        // Linux artifacts before their request can acquire a listener.
        RelayServer::start_vtcode_unix(profile.binding.clone(), backend, context, routes, catalog).await?
    } else {
        RelayServer::start_with_routes_and_vtcode_catalog(profile.binding.clone(), backend, context, routes, catalog).await?
    };
    #[cfg(not(target_os = "linux"))]
    let server = RelayServer::start_with_routes_and_vtcode_catalog(
        profile.binding.clone(), backend, context, routes, catalog,
    ).await?;
    let endpoint = format!("http://{}", server.address());
    profile.attach_server(server);
    if let Some(projection) = projection.as_ref().filter(|_| matches!(agent, "vtcode" | "hermes" | "auggie" | "droid" | "autohand" | "iflow" | "glm-acp-agent")) {
        llm_runtime::acp::model_source::configure_profile(agent, &mut profile, &endpoint, &directory, openclaw_mcp).await?;
        if agent == "auggie" {
            let tools = projection.tools.iter().map(|name| {
                name.strip_prefix("mcp__auggie__").ok_or_else(|| anyhow::anyhow!("Invalid Auggie MCP approval name"))
            }).collect::<anyhow::Result<Vec<_>>>()?;
            let flags = tools.iter().flat_map(|tool| ["--permission".to_owned(), format!("{tool}:ask-user")])
                .collect::<Vec<_>>();
            profile.append_arguments(&flags)?;
            profile.overlay_environment(std::collections::HashMap::from([
                ("SHIJU_AUGGIE_MCP_TOOLS".into(), serde_json::to_string(&tools)?),
            ]))?;
        }
        let (servers, mut approve) = projection.approval_config(&endpoint)?;
        if agent == "iflow" {
            profile.overlay_environment(std::collections::HashMap::from([
                ("SHIJU_IFLOW_MCP_RELAY".into(), endpoint.clone()),
                ("SHIJU_IFLOW_MCP_TOKEN".into(), projection.token().to_owned()),
            ]))?;
        }
        profile.managed_mcp_current = review_is_current.clone();
        if let Some(current) = review_is_current {
            let issue = approve;
            approve = std::sync::Arc::new(move |name, call_id, input, valid| {
                if !current() {
                    return false;
                }
                let current = current.clone();
                issue(name, call_id, input, Box::new(move || current() && valid()))
            });
        }
        profile.managed_mcp = Some((servers, approve));
    } else if let Some(mimo) = projection {
        llm_runtime::acp::model_source::drivers::launch_route(agent, &profile)?;
        llm_runtime::acp::model_source::drivers::mimo_code::configure_with_mcp(
            &mut profile, &endpoint, &directory, mimo.native_config(&endpoint), &mimo.tools,
        )?;
        profile.overlay_environment(std::collections::HashMap::from([
            ("SHIJU_MIMO_MCP_TOKEN".to_owned(), mimo.token().to_owned()),
        ]))?;
    } else {
        let project_directory = instance::directory();
        let configuration_directory = if agent == "copilot" { &project_directory } else { &directory };
        llm_runtime::acp::model_source::configure_profile(agent, &mut profile, &endpoint, configuration_directory, openclaw_mcp).await?;
    }
    Ok(profile)
}

type ReviewValidity = std::sync::Arc<dyn Fn() -> bool + Send + Sync>;

#[cfg(all(target_os = "linux", target_env = "gnu", any(target_arch = "x86_64", target_arch = "aarch64")))]
async fn prepare_reviewed_mcp_batch(
    agent: &str,
    servers: &[acp_types::AcpMcpServer],
    directory: &str,
    binding: std::sync::Arc<crate::model_relay::RelayBinding>,
) -> anyhow::Result<(Option<mimo_mcp::Projection>, Option<ReviewValidity>)> {
    use crate::agent_apps::mcp::acp_mount::{ReviewedStdioSource, reviewed_linux_stdio_list};
    use mimo_mcp::{LinuxMcpOwner, LinuxMcpSource, ReviewedLinuxStdio};

    let selected = reviewed_linux_stdio_list(
        binding.clone(), std::path::Path::new(directory), servers,
    ).await?;
    let Some(first) = selected.first() else {
        anyhow::ensure!(servers.is_empty(), "Missing reviewed Linux MCP selection");
        return Ok((None, None));
    };
    let current = first.current.clone();
    let policy_key = if agent == "iflow" { "shiju.iflow.toolPolicy" } else { "shiju.glm.toolPolicy" };
    let policies = selected.iter().map(|selection| {
        let acp_types::AcpMcpServer::Stdio(service) = &selection.server else {
            anyhow::bail!("Managed Linux MCP requires reviewed stdio services");
        };
        let mut policy = acp_config::McpServerManagement::default_for_server(&service.name);
        if selection.source == ReviewedStdioSource::Project {
            let narrowing = service.meta.as_ref().and_then(|meta| meta.get(policy_key));
            anyhow::ensure!(
                agent != "iflow" || narrowing.is_some(),
                "Managed iFlow Linux MCP requires a host-selected tool policy"
            );
            if let Some(narrowing) = narrowing {
                policy.allowed_tools = serde_json::from_value(narrowing["allow"].clone())?;
                policy.denied_tools = serde_json::from_value(narrowing["deny"].clone())?;
            }
        }
        let mut allowed = Vec::new();
        for name in selection.review.allowed_tools() {
            let id = tool_mcp::mcp_tool::mcp_tool_id(&service.name, name)?;
            if mcp_client::mcp_tool_allowed(&policy, name, &id) {
                allowed.push(name.clone());
            }
        }
        policy.allowed_tools = allowed;
        Ok(policy)
    }).collect::<anyhow::Result<Vec<_>>>()?;
    let declarations = selected.iter().map(|selection| selection.server.clone()).collect::<Vec<_>>();
    let pins = selected.iter().zip(&policies).map(|(selection, policy)| {
        let source = match selection.source {
            ReviewedStdioSource::Project => LinuxMcpSource::Project,
            ReviewedStdioSource::Mounted => LinuxMcpSource::Mounted,
        };
        ReviewedLinuxStdio {
            owner: if agent == "iflow" { LinuxMcpOwner::IFlow(source) } else { LinuxMcpOwner::Glm(source) },
            executable: selection.review.executable(),
            sha256: selection.review.sha256(),
            args: selection.review.args(),
            policy,
        }
    }).collect::<Vec<_>>();
    let projection = mimo_mcp::prepare_linux_stdio_many(
        &declarations, &pins, binding, current.clone(),
    ).await?;
    Ok((Some(projection), Some(current)))
}

async fn prepare_mcp_projection(
    agent: &str,
    servers: &[acp_types::AcpMcpServer],
    directory: &str,
    binding: std::sync::Arc<crate::model_relay::RelayBinding>,
) -> anyhow::Result<(Option<mimo_mcp::Projection>, Option<ReviewValidity>)> {
    #[cfg(all(target_os = "linux", target_env = "gnu", any(target_arch = "x86_64", target_arch = "aarch64")))]
    if matches!(agent, "iflow" | "glm-acp-agent") {
        return prepare_reviewed_mcp_batch(agent, servers, directory, binding).await;
    }
    #[cfg(all(target_os = "linux", target_env = "gnu", any(target_arch = "x86_64", target_arch = "aarch64")))]
    if agent == "vtcode" {
        use crate::agent_apps::mcp::acp_mount::{ReviewedStdioSource, reviewed_linux_stdio};

        let Some(selected) = reviewed_linux_stdio(
            binding.clone(), std::path::Path::new(directory), servers,
        ).await? else {
            anyhow::ensure!(servers.is_empty(), "Missing reviewed Linux MCP selection");
            return Ok((None, None));
        };
        let acp_types::AcpMcpServer::Stdio(service) = &selected.server else {
            anyhow::bail!("Managed Linux MCP requires one reviewed stdio service");
        };
        anyhow::ensure!(
            selected.source == ReviewedStdioSource::Project,
            "This Linux adapter does not support the MCP selection source"
        );
        let review = &selected.review;
        let mut policy = acp_config::McpServerManagement::default_for_server(&service.name);
        let selected_policy = service.meta.as_ref()
            .and_then(|meta| meta.get("shiju.vtcode.toolPolicy"))
            .ok_or_else(|| anyhow::anyhow!("Managed Linux MCP requires a host-selected tool policy"))?;
        policy.allowed_tools = serde_json::from_value(selected_policy["allow"].clone())?;
        policy.denied_tools = serde_json::from_value(selected_policy["deny"].clone())?;
        // Request policy may only narrow the independent review.
        let mut allowed = Vec::new();
        for name in review.allowed_tools() {
            let id = tool_mcp::mcp_tool::mcp_tool_id(&service.name, name)?;
            if mcp_client::mcp_tool_allowed(&policy, name, &id) {
                allowed.push(name.clone());
            }
        }
        policy.allowed_tools = allowed;
        let current = selected.current;
        anyhow::ensure!(current(), "Managed Linux MCP review was revoked");
        let projection = mimo_mcp::prepare_linux_stdio(
            std::slice::from_ref(&selected.server),
            mimo_mcp::ReviewedLinuxStdio {
                owner: mimo_mcp::LinuxMcpOwner::VtCode,
                executable: review.executable(),
                sha256: review.sha256(),
                args: review.args(),
                policy: &policy,
            },
            binding.clone(),
        )
        .await?;
        anyhow::ensure!(current(), "Managed Linux MCP review changed during discovery");
        let current = move || {
            if current() {
                true
            } else {
                binding.revoke();
                false
            }
        };
        return Ok((Some(projection), Some(std::sync::Arc::new(current))));
    }
    let _ = agent;
    Ok((Some(mimo_mcp::prepare(servers, directory, binding).await?), None))
}

pub(super) fn launch_identity(
    project: String,
    agent: &str,
    model: &provider::Model,
    session: &str,
) -> RelayIdentity {
    llm_runtime::acp::model_source::launch_identity(
        crate::session::ui_store::current_tenant_scope(), project, agent, model, session,
    )
}

#[cfg(all(test, unix))]
#[path = "profile_tests.rs"]
mod tests;

#[cfg(test)]
pub(super) mod acceptance_tests;

#[cfg(all(test, target_os = "linux", target_env = "gnu", any(target_arch = "x86_64", target_arch = "aarch64")))]
mod iflow_project_tests;
