//! Bind the installed fixed-version aionrs bridge to the selected request model.
use super::{super::profile::REFERENCE_ENV, AcpLaunchProfile};
use std::{collections::HashMap, path::Path};

pub fn configure(
    profile: &mut AcpLaunchProfile,
    endpoint: &str,
    directory: &str,
) -> anyhow::Result<()> {
    let config = profile.config();
    let (managed, _) = acp_agent_registry::reviewed_agent_command("aionrs")
        .map_err(anyhow::Error::msg)?
        .ok_or_else(|| anyhow::anyhow!("Install the reviewed aionrs runtime"))?;
    anyhow::ensure!(
        config.args.is_empty()
            && config.env.keys().all(|key| key == REFERENCE_ENV)
            && Path::new(&config.command).canonicalize()? == Path::new(&managed).canonicalize()?,
        "aionrs requires the installed bridge and its fixed launch configuration"
    );
    let workspace = Path::new(directory).canonicalize()?;
    let write_root = match std::env::var_os("SHIJU_AIONRS_WRITE_ROOT") {
        Some(root) => std::fs::canonicalize(root)?,
        None => workspace.clone(),
    };
    let engine = Path::new(&managed).canonicalize()?.with_file_name("aionrs");
    let root = profile.directory().canonicalize()?;
    anyhow::ensure!(
        workspace.parent().is_some()
            && write_root.is_dir()
            && write_root.starts_with(&workspace)
            && !engine.starts_with(&workspace)
            && !root.starts_with(&workspace)
            && !workspace.starts_with(&root),
        "Invalid aionrs workspace, write root or private runtime path"
    );
    let url = reqwest::Url::parse(endpoint)?;
    let port = url.port().ok_or_else(|| anyhow::anyhow!("Missing aionrs relay port"))?;
    anyhow::ensure!(
        url.scheme() == "http"
            && url.host_str() == Some("127.0.0.1")
            && url.path() == "/"
            && url.username().is_empty()
            && url.password().is_none()
            && url.query().is_none()
            && url.fragment().is_none(),
        "aionrs requires a private loopback relay"
    );
    let model = profile.model_alias.clone();
    anyhow::ensure!(
        profile.model_supports_tool_call && !model.is_empty(),
        "aionrs requires a tool-capable selected model"
    );
    let state = root.join("state");
    std::fs::create_dir(&state)?;
    let path = |value: &Path| -> anyhow::Result<String> {
        Ok(serde_json::to_string(
            value.to_str().ok_or_else(|| anyhow::anyhow!("Invalid aionrs runtime path"))?,
        )?)
    };
    let policy = format!(
        "(version 1)\n(allow default)\n(deny file-read-data (subpath \"/Users\") (subpath \"/Volumes\") (subpath \"/private/tmp\") (subpath \"/private/var/folders\"))\n\
         (allow file-read-data (subpath \"/System\") (subpath \"/usr/lib\") (subpath \"/usr/share\")\n\
          (subpath \"/dev\") (subpath {workspace}) (subpath {state}) (subpath {skills}) (literal {engine}))\n\
         (deny file-write*)\n\
         (allow file-write* (subpath {state}) (subpath {write_root}) (literal \"/dev/null\"))\n\
         (deny file-write* (subpath {git}) (literal {local_config}))\n\
         (deny process-fork process-exec)\n(allow process-exec (literal {engine}))\n\
         (deny network*)\n(allow network-outbound (remote ip \"localhost:{port}\"))\n",
        workspace = path(&workspace)?,
        state = path(&state)?,
        engine = path(&engine)?,
        skills = path(&root.join(".aionrs/skills"))?,
        write_root = path(&write_root)?,
        git = path(&workspace.join(".git"))?,
        local_config = path(&workspace.join(".aionrs.toml"))?,
    );
    let policy = profile.write_config("aionrs.sb", policy.as_bytes())?;
    let mcp_socket = root.join("m");
    let mut token = [0; 32];
    getrandom::fill(&mut token).map_err(|_| anyhow::anyhow!("aionrs MCP entropy unavailable"))?;
    let token = hex::encode(token);
    profile.bind_aionrs_mcp(mcp_socket.clone(), token.clone())?;
    profile.overlay_environment(HashMap::from([
        ("SHIJU_AIONRS_SANDBOX".into(), policy.to_string_lossy().into_owned()),
        ("SHIJU_AIONRS_MCP".into(), mcp_socket.to_string_lossy().into_owned()),
        ("SHIJU_AIONRS_MCP_TOKEN".into(), token),
        ("SHIJU_AIONRS_STATE".into(), state.to_string_lossy().into_owned()),
        ("SHIJU_AIONRS_SKILL_ROOT".into(), root.to_string_lossy().into_owned()),
        ("SHIJU_AIONRS_WORKSPACE".into(), workspace.to_string_lossy().into_owned()),
        ("API_KEY".into(), profile.binding.bearer_token().into()),
        ("BASE_URL".into(), url.join("v1")?.to_string()),
        ("MODEL".into(), model.clone()),
        ("PROVIDER".into(), "openai".into()),
    ]))?;
    profile.session_model_id = Some(model);
    Ok(())
}
