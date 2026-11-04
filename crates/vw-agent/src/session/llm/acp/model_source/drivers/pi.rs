use super::AcpLaunchProfile;
use serde_json::json;
use sha2::{Digest, Sha256};
use std::{collections::HashMap, io::Write, path::Path};

pub(super) const PATCHED_ADAPTER_SHA256: &str =
    "a78e867c410e7b29c93196586a20b383ecd9f852d865c57af01f8970bc739e76";

pub(super) fn configure(profile: &mut AcpLaunchProfile, endpoint: &str) -> anyhow::Result<()> {
    let command = Path::new(&profile.config().command);
    anyhow::ensure!(command.is_absolute(), "Pi requires the pinned patched adapter executable");
    let bytes =
        std::fs::read(command).map_err(|_| anyhow::anyhow!("cannot read pinned Pi adapter"))?;
    anyhow::ensure!(
        hex::encode(Sha256::digest(bytes)) == PATCHED_ADAPTER_SHA256,
        "Pi adapter state-directory patch is missing or differs from the reviewed build"
    );
    let engine = std::env::var_os("PATH")
        .and_then(|paths| {
            std::env::split_paths(&paths).map(|path| path.join("pi")).find(|path| path.is_file())
        })
        .ok_or_else(|| anyhow::anyhow!("pinned native Pi executable is not installed"))?;
    let engine = engine.canonicalize()?;
    anyhow::ensure!(
        hex::encode(Sha256::digest(std::fs::read(&engine)?))
            == "e959f463b06ddd15ed882783ac02f39ecaeef950ef967da86380f39dda6595fa",
        "native Pi executable differs from reviewed 0.73.1 build"
    );
    configure_native(profile, endpoint)?;
    profile.overlay_environment(HashMap::from([(
        "PI_ACP_PI_COMMAND".into(),
        engine.to_string_lossy().into_owned(),
    )]))
}

/// Configuration boundary is exercised separately from supplier artifact identity.
fn configure_native(profile: &mut AcpLaunchProfile, endpoint: &str) -> anyhow::Result<()> {
    anyhow::ensure!(
        profile.config().args.is_empty(),
        "Pi adapter accepts no model-source arguments"
    );
    anyhow::ensure!(!profile.model_alias.is_empty(), "Pi requires a selected model");
    anyhow::ensure!(
        !profile.config().env.keys().any(|key| {
            let key = key.to_ascii_uppercase();
            key.starts_with("PI_")
                || key.starts_with("VW_PI_")
                || matches!(
                    key.as_str(),
                    "HOME" | "USERPROFILE" | "PATH" | "NODE_PATH" | "NODE_OPTIONS" | "BUN_OPTIONS"
                )
        }),
        "conflicting Pi model-source environment"
    );
    let url =
        reqwest::Url::parse(endpoint).map_err(|_| anyhow::anyhow!("invalid Pi relay endpoint"))?;
    anyhow::ensure!(
        url.scheme() == "http"
            && url.host_str() == Some("127.0.0.1")
            && url.port().is_some()
            && url.username().is_empty()
            && url.password().is_none()
            && url.path() == "/"
            && url.query().is_none()
            && url.fragment().is_none(),
        "Pi requires an owned loopback relay"
    );
    let root = profile.state_root.as_deref().unwrap_or(profile.directory());
    anyhow::ensure!(root.is_absolute() && root.is_dir(), "invalid Pi state root");
    let identity = &profile.binding.identity;
    // Model rotation retains native history, while other sources/sessions never share it.
    let reference = hex::encode(Sha256::digest(serde_json::to_vec(&json!({
        "tenant": identity.tenant, "project": identity.project, "session": identity.session,
    }))?));
    let parent = root.join("acp-pi-session-model");
    private_directory(&parent)?;
    let state = parent.join(reference);
    private_directory(&state)?;
    let native = profile.directory().join("native");
    let sessions = state.join("sessions");
    private_directory(&sessions)?;
    let acp = state.join("acp");
    private_directory(&native)?;
    private_directory(&acp)?;
    let mut environment = HashMap::from([
        ("PI_CODING_AGENT_DIR".into(), native.to_string_lossy().into_owned()),
        ("PI_ACP_STATE_DIR".into(), acp.to_string_lossy().into_owned()),
        ("VW_PI_RELAY_TOKEN".into(), profile.binding.bearer_token().to_owned()),
        ("NODE_OPTIONS".into(), String::new()),
        ("NODE_PATH".into(), String::new()),
        ("PI_PACKAGE_DIR".into(), String::new()),
        ("PI_OFFLINE".into(), "1".into()),
        ("PI_SKIP_VERSION_CHECK".into(), "1".into()),
        ("BUN_OPTIONS".into(), String::new()),
    ]);
    // Pi's built-in providers must not discover an ambient account alongside our model.
    for key in [
        "OPENAI_API_KEY",
        "AZURE_OPENAI_API_KEY",
        "GEMINI_API_KEY",
        "GROQ_API_KEY",
        "CEREBRAS_API_KEY",
        "XAI_API_KEY",
        "OPENROUTER_API_KEY",
        "AI_GATEWAY_API_KEY",
        "ZAI_API_KEY",
        "MISTRAL_API_KEY",
        "MINIMAX_API_KEY",
        "MINIMAX_CN_API_KEY",
        "HF_TOKEN",
        "OPENCODE_API_KEY",
        "KIMI_API_KEY",
        "COPILOT_GITHUB_TOKEN",
        "GH_TOKEN",
        "GITHUB_TOKEN",
        "ANTHROPIC_OAUTH_TOKEN",
        "ANTHROPIC_API_KEY",
        "HTTP_PROXY",
        "HTTPS_PROXY",
        "ALL_PROXY",
        "http_proxy",
        "https_proxy",
        "all_proxy",
    ] {
        environment.insert(key.into(), String::new());
    }
    environment.insert("NO_PROXY".into(), "127.0.0.1,localhost".into());
    environment.insert("no_proxy".into(), "127.0.0.1,localhost".into());
    profile.overlay_environment(environment)?;
    publish(
        &native,
        "models.json",
        &json!({"providers": {"session-provider": {
            "baseUrl": format!("{endpoint}/v1"), "api": "openai-completions",
            "apiKey": "VW_PI_RELAY_TOKEN", "models": [{
                "id": profile.model_alias, "reasoning": false, "input": ["text"],
                "contextWindow": profile.model_context_limit.max(1),
                "maxTokens": profile.model_output_limit.max(1),
                "compat": {"supportsDeveloperRole": false, "supportsReasoningEffort": false}
            }]
        }}}),
    )?;
    publish(
        &native,
        "settings.json",
        &json!({
            "defaultProvider": "session-provider", "defaultModel": profile.model_alias,
            "quietStartup": true, "enableSkillCommands": false,
        "sessionDir": sessions,
        }),
    )?;
    profile.session_model_id = Some(format!("session-provider/{}", profile.model_alias));
    Ok(())
}

fn publish(directory: &Path, name: &str, value: &serde_json::Value) -> anyhow::Result<()> {
    let path = directory.join(name);
    if let Ok(metadata) = path.symlink_metadata() {
        anyhow::ensure!(
            metadata.is_file() && !metadata.file_type().is_symlink(),
            "unsafe Pi config file"
        );
    }
    let mut file = tempfile::NamedTempFile::new_in(directory)?;
    file.write_all(&serde_json::to_vec(value)?)?;
    file.as_file().sync_all()?;
    file.persist(path).map_err(|_| anyhow::anyhow!("cannot publish Pi configuration"))?;
    Ok(())
}

fn private_directory(path: &Path) -> anyhow::Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::{DirBuilderExt, MetadataExt, PermissionsExt};
        match std::fs::DirBuilder::new().mode(0o700).create(path) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
            Err(error) => return Err(error.into()),
        }
        let metadata = path.symlink_metadata()?;
        anyhow::ensure!(
            metadata.is_dir()
                && !metadata.file_type().is_symlink()
                && metadata.permissions().mode() & 0o777 == 0o700
                && metadata.uid() == unsafe { libc::geteuid() },
            "insecure Pi state directory"
        );
        Ok(())
    }
    #[cfg(not(unix))]
    {
        let _ = path;
        anyhow::bail!("Pi private state ACL verification unavailable")
    }
}

#[cfg(test)]
#[path = "pi_tests.rs"]
mod tests;
