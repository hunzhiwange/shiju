//! The reviewed scanner runs in the owned Gateway, before the ACP bridge starts.
use agent_config::schema::AcpAgentConfig;

pub(super) const ROOT_ENV: &str = "SHIJU_OPENCLAW_SKILL_ROOT";

pub(super) fn configure(
    config: &mut AcpAgentConfig,
    view: Option<&super::MountedSkills>,
) -> Result<(), super::Error> {
    if config.env.contains_key(ROOT_ENV) || std::env::var_os(ROOT_ENV).is_some() {
        return Err(super::super::acp_option_error(
            "conflicting OpenClaw skill directory environment",
        ));
    }
    let root = if let Some(view) = view {
        view.0.validate().map_err(super::super::to_api_error)?;
        acp_agent_registry::verify_reviewed_agent_executable("openclaw", &config.command)
            .map_err(super::super::to_api_error)?;
        view.0.root().to_string_lossy().into_owned()
    } else {
        String::new()
    };
    config.env.insert(ROOT_ENV.into(), root);
    Ok(())
}
#[cfg(test)]
#[path = "openclaw_tests.rs"]
mod tests;

pub(in crate::acp) fn restrict(
    profile: &super::super::model_source::profile::AcpLaunchProfile,
    cwd: &std::path::Path,
    view: Option<&impl super::super::host::SkillLease>,
    mounts: &[agent_client_protocol::McpServer],
) -> anyhow::Result<acp_client::OpenClawSandbox> {
    use serde_json::json;
    use std::path::Path;

    let workspace = cwd.canonicalize()?;
    let directory = profile.directory().canonicalize()?;
    let executable = Path::new(&profile.config().command).canonicalize()?;
    let runtime = executable
        .parent()
        .and_then(Path::parent)
        .and_then(Path::parent)
        .ok_or_else(|| anyhow::anyhow!("Missing restricted OpenClaw runtime directory"))?;
    let disjoint = |a: &Path, b: &Path| !a.starts_with(b) && !b.starts_with(a);
    anyhow::ensure!(
        disjoint(&workspace, &directory)
            && disjoint(&workspace, runtime)
            && workspace.parent().is_some(),
        "Restricted OpenClaw requires a workspace separate from its runtime"
    );
    let mut sources = Vec::new();
    let mut root = String::new();
    if let Some(view) = view {
        view.validate().map_err(|error| anyhow::anyhow!(error.to_string()))?;
        let path = view.root().canonicalize()?;
        anyhow::ensure!(
            disjoint(&path, &directory) && disjoint(&path, runtime),
            "Invalid OpenClaw session view"
        );
        for entry in std::fs::read_dir(&path)? {
            let entry = entry?;
            anyhow::ensure!(entry.file_type()?.is_symlink(), "Invalid OpenClaw skill projection");
            let source = entry.path().canonicalize()?;
            anyhow::ensure!(
                source.is_dir()
                    && source.parent().is_some()
                    && disjoint(&source, &directory)
                    && disjoint(&source, runtime),
                "OpenClaw skill source overlaps private state or runtime"
            );
            sources.push(source);
        }
        root = path.to_str().ok_or_else(|| anyhow::anyhow!("Invalid OpenClaw skill path"))?.into();
        view.validate().map_err(|error| anyhow::anyhow!(error.to_string()))?;
    }
    anyhow::ensure!(
        mounts.iter().all(|mount| matches!(mount, agent_client_protocol::McpServer::Stdio(_))),
        "Restricted OpenClaw supports only host-approved stdio MCP"
    );
    let config: serde_json::Value =
        serde_json::from_slice(&std::fs::read(directory.join("openclaw-draft.json"))?)?;
    let node = shell_command::resolve_executable("node")
        .ok_or_else(|| anyhow::anyhow!("OpenClaw requires Node.js"))?
        .canonicalize()?;
    anyhow::ensure!(disjoint(&workspace, &node), "OpenClaw Node.js must be outside the workspace");
    std::fs::create_dir(directory.join("native"))?;
    profile.write_config("openclaw-launch.json", &serde_json::to_vec(&json!({
        "workspace": workspace, "config": config, "skill_root":root, "skill_sources": sources,
        "session": format!("agent:main:acp-{}", directory.file_name().unwrap().to_string_lossy()),
        "mounts": mounts,
    }))?)?;
    for (name, body) in [
        (
            "openclaw-launcher.py",
            include_str!("../../../../../../patches/acp-model-source/openclaw/launcher.py"),
        ),
        (
            "omp-mcp-bridge.py",
            include_str!("../../../../../../patches/acp-model-source/omp/mcp_bridge.py"),
        ),
        (
            "omp-mcp-stdio.py",
            include_str!("../../../../../../patches/acp-model-source/cortex-code/mcp_bridge.py"),
        ),
    ] {
        profile.write_config(name, body.as_bytes())?;
    }
    Ok(acp_client::OpenClawSandbox { directory, workspace, node })
}

pub(in crate::acp) fn restricted_launch(
    source: vw_api_acp_model::acp_model_source::AcpModelSource,
    mode: Option<acp_types::PermissionMode>,
    full_access: bool,
    non_interactive: bool,
    disable_tools: bool,
) -> bool {
    use acp_types::PermissionMode;
    use vw_api_acp_model::acp_model_source::AcpModelSource;
    cfg!(all(target_os = "macos", target_arch = "aarch64"))
        && source == AcpModelSource::SessionModel
        && mode == Some(PermissionMode::ApproveReads)
        && !full_access
        && !non_interactive
        && !disable_tools
}
