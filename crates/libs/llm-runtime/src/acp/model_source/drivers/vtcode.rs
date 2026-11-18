//! Bind the reviewed native VT Code to the request-owned Chat relay.
use super::{super::profile::REFERENCE_ENV, AcpLaunchProfile};
use std::{collections::HashMap, path::Path};

pub fn configure(
    profile: &mut AcpLaunchProfile,
    endpoint: &str,
    directory: &str,
) -> anyhow::Result<()> {
    let config = profile.config();
    anyhow::ensure!(
        config.args == ["acp"]
            && config.env.iter().all(|(key, value)| {
                key == REFERENCE_ENV
                    || (matches!(key.as_str(), "VT_ACP_ENABLED" | "VT_ACP_ZED_ENABLED")
                        && value == "1")
            }),
        "VT Code requires reviewed ACP arguments and environment"
    );
    acp_agent_registry::verify_reviewed_agent_executable("vtcode", &config.command)
        .map_err(anyhow::Error::msg)?;
    let native = Path::new(&config.command).canonicalize()?;
    let workspace = Path::new(directory).canonicalize()?;
    anyhow::ensure!(
        workspace.is_dir() && workspace.parent().is_some(),
        "Invalid VT Code workspace"
    );
    for ancestor in workspace.ancestors() {
        for name in ["vtcode.toml", ".vtcode", ".env"] {
            match ancestor.join(name).symlink_metadata() {
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                _ => anyhow::bail!("VT Code project configuration overrides are not verified"),
            }
        }
    }
    let url = reqwest::Url::parse(endpoint)?;
    let port = url.port().ok_or_else(|| anyhow::anyhow!("Missing VT Code relay port"))?;
    anyhow::ensure!(
        url.scheme() == "http"
            && url.host_str() == Some("127.0.0.1")
            && url.path() == "/"
            && url.username().is_empty()
            && url.password().is_none()
            && url.query().is_none()
            && url.fragment().is_none(),
        "VT Code requires a private loopback relay"
    );
    let provider = &profile.binding.identity().provider;
    let model = profile.model_alias.clone();
    anyhow::ensure!(
        profile.model_supports_tool_call && !model.is_empty() && !provider.is_empty(),
        "VT Code requires a selected tool-capable model"
    );
    let quote = serde_json::to_string::<str>;
    let config = format!(
        "providers_whitelist = [{provider}]\n[agent]\nprovider = {provider}\ndefault_model = {model}\nreasoning_effort = \"none\"\n\
         [[custom_providers]]\nname = {provider}\ndisplay_name = {provider}\nbase_url = {url}\n\
         api_key_env = \"SHIJU_VTCODE_RELAY_TOKEN\"\nmodel = {model}\nmodels = [{model}]\n\
         api_format = \"openai-chat\"\nsupports_tools = true\n\
         [tools.policies]\nexec_command = \"deny\"\nwrite_stdin = \"deny\"\napply_patch = \"prompt\"\n",
        provider = quote(provider)?,
        model = quote(&model)?,
        url = quote(url.join("v1")?.as_str())?,
    );
    let config = profile.write_config("vtcode.toml", config.as_bytes())?;
    let state = profile.directory().join("state");
    std::fs::create_dir(&state)?;
    let state = state.canonicalize()?;
    let forbidden =
        [workspace.join("vtcode.toml"), workspace.join(".vtcode"), workspace.join(".env")];
    let path = |path: &Path| -> anyhow::Result<String> {
        Ok(quote(path.to_str().ok_or_else(|| anyhow::anyhow!("Non-UTF-8 VT Code path"))?)?)
    };
    let home = std::fs::canonicalize(
        std::env::var_os("HOME").ok_or_else(|| anyhow::anyhow!("Missing VT Code host home"))?,
    )?;
    anyhow::ensure!(!home.starts_with(&workspace), "VT Code workspace includes the host home");
    let policy = format!(
        "(version 1)\n(allow default)\n(deny file-read-data (subpath {}))\n\
         (allow file-read-data (subpath {}) (subpath {}) (literal {}) (literal {}))\n\
         (deny file-write*)\n\
         (allow file-write* (subpath {}) (subpath {}) (literal \"/dev/null\"))\n\
         (deny file-write* (literal {}) (subpath {}) (literal {}))\n\
         (deny process-fork process-exec)\n(allow process-exec (literal {}))\n\
         (deny network*)\n(allow network-outbound (remote ip \"localhost:{port}\"))\n",
        path(&home)?,
        path(&workspace)?,
        path(&state)?,
        path(&native)?,
        path(&config.canonicalize()?)?,
        path(&workspace)?,
        path(&state)?,
        path(&forbidden[0])?,
        path(&forbidden[1])?,
        path(&forbidden[2])?,
        path(&native)?,
    );
    let policy = profile.write_config("vtcode.sb", policy.as_bytes())?;
    profile.overlay_environment(HashMap::from([
        ("SHIJU_VTCODE_CONFIG".into(), config.to_string_lossy().into_owned()),
        ("SHIJU_VTCODE_SANDBOX".into(), policy.to_string_lossy().into_owned()),
        ("SHIJU_VTCODE_HOME".into(), state.to_string_lossy().into_owned()),
        ("SHIJU_VTCODE_RELAY_TOKEN".into(), profile.binding.bearer_token().into()),
        ("NO_PROXY".into(), "127.0.0.1,localhost".into()),
    ]))?;
    profile.session_model_id = Some(model);
    Ok(())
}
