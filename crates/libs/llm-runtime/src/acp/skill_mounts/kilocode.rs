//! Append the validated request-owned view to Kilocode’s native scanner.
use agent_config::schema::AcpAgentConfig;

const ROOT_ENV: &str = "SHIJU_KILO_SKILL_ROOT";

pub(super) fn configure(
    config: &mut AcpAgentConfig,
    view: Option<&super::MountedSkills>,
) -> Result<(), super::Error> {
    if config.env.contains_key(ROOT_ENV) || std::env::var_os(ROOT_ENV).is_some() {
        return Err(super::super::acp_option_error(
            "conflicting Kilocode skill directory environment",
        ));
    }
    let root = if let Some(view) = view {
        view.0.validate().map_err(super::super::to_api_error)?;
        acp_agent_registry::verify_reviewed_agent_executable("kilocode", &config.command)
            .map_err(super::super::to_api_error)?;
        view.0.root().to_string_lossy().into_owned()
    } else {
        // The reviewed scanner still discovers native skills with an empty selection.
        String::new()
    };
    config.env.insert(ROOT_ENV.into(), root);
    Ok(())
}

#[cfg(test)]
#[path = "kilocode_tests.rs"]
mod tests;

pub(in crate::acp) fn sandbox(
    profile: &super::super::model_source::profile::AcpLaunchProfile,
    config: &mut AcpAgentConfig,
    cwd: &std::path::Path,
    view: Option<&impl super::super::host::SkillLease>,
) -> anyhow::Result<acp_client::KilocodeSandbox> {
    use std::path::Path;
    let workspace = cwd.canonicalize()?;
    let directory = profile.directory().canonicalize()?;
    let state_root = profile
        .state_root
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("Missing Kilocode state root"))?
        .canonicalize()?;
    let disjoint = |path: &Path| !path.starts_with(&workspace) && !workspace.starts_with(path);
    anyhow::ensure!(
        disjoint(&directory) && disjoint(&state_root) && workspace.parent().is_some(),
        "Restricted Kilocode requires a workspace separate from private state"
    );
    let home = directory.join("home");
    std::fs::create_dir(&home)?;
    let (root, sources) = view.map_or_else(
        || Ok::<_, anyhow::Error>((None, Vec::new())),
        |view| {
            view.validate().map_err(|error| anyhow::anyhow!(error.to_string()))?;
            selection(view.root()).map(|(root, sources)| (Some(root), sources))
        },
    )?;
    config.env.insert(
        ROOT_ENV.into(),
        root.as_deref()
            .map(Path::to_str)
            .unwrap_or(Some(""))
            .ok_or_else(|| anyhow::anyhow!("Invalid Kilocode skill root"))?
            .into(),
    );
    config.env.insert("SHIJU_KILO_SKILL_SOURCES".into(), serde_json::to_string(&sources)?);
    for (name, contents) in [
        (
            "kilocode-launcher.py",
            include_str!("../../../../../../patches/acp-model-source/kilocode/launcher.py"),
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
        profile.write_config(name, contents.as_bytes())?;
    }
    Ok(acp_client::KilocodeSandbox { directory, workspace })
}

fn selection(
    root: &std::path::Path,
) -> anyhow::Result<(std::path::PathBuf, Vec<std::path::PathBuf>)> {
    let root = root.canonicalize()?;
    let mut sources = Vec::new();
    for entry in std::fs::read_dir(&root)? {
        let entry = entry?;
        anyhow::ensure!(entry.file_type()?.is_symlink(), "Invalid Kilocode skill projection");
        let source = entry.path().canonicalize()?;
        anyhow::ensure!(
            source.is_dir() && source.parent().is_some(),
            "Invalid Kilocode skill source"
        );
        sources.push(source);
    }
    sources.sort();
    Ok((root, sources))
}

pub(in crate::acp) fn validate_mcp(
    servers: &[acp_types::AcpMcpServer],
) -> Result<(), super::Error> {
    for server in servers {
        let acp_types::AcpMcpServer::Stdio(server) = server else {
            return Err(super::super::acp_option_error(
                "Restricted Kilocode MCP requires host-selected stdio services",
            ));
        };
        let policy = server.meta.as_ref().and_then(|meta| meta.get("shiju.omp.toolPolicy"));
        if !server.env.is_empty()
            || !policy.is_some_and(|policy| {
                ["allow", "deny"].into_iter().all(|key| {
                    policy[key]
                        .as_array()
                        .is_some_and(|names| names.iter().all(serde_json::Value::is_string))
                })
            })
        {
            return Err(super::super::acp_option_error(
                "Restricted Kilocode MCP requires a host tool policy and no environment overrides",
            ));
        }
    }
    Ok(())
}
