use super::AcpLaunchProfile;
use serde_json::json;
use std::{collections::HashMap, path::Path};
use vw_shared::provider::reasoning::{ReasoningFormat, reasoning_levels, reasoning_profile};

/// OpenCode 1.19.5 merges inline configuration after project and home agents.
/// Isolate both XDG state and its dedicated home lookup; never change the tool HOME.
pub(super) fn configure(profile: &mut AcpLaunchProfile, endpoint: &str) -> anyhow::Result<()> {
    anyhow::ensure!(
        profile.config().args == ["acp"],
        "OpenCode requires native ACP without overrides"
    );
    // OpenCode substitutes environment/file references before parsing inline JSON.
    anyhow::ensure!(
        !profile.model_alias.is_empty() && !profile.model_alias.contains(['{', '}', '\0']),
        "OpenCode requires a literal selected model"
    );
    anyhow::ensure!(
        !profile.config().env.keys().any(|key| {
            let key = key.to_ascii_uppercase();
            key.starts_with("OPENCODE_")
                || key.starts_with("VW_OPENCODE_")
                || key.starts_with("XDG_")
                || matches!(
                    key.as_str(),
                    "NODE_OPTIONS" | "BUN_OPTIONS" | "HTTP_PROXY" | "HTTPS_PROXY" | "ALL_PROXY"
                )
        }),
        "conflicting OpenCode model environment"
    );
    reject_managed_configuration()?;
    let url = reqwest::Url::parse(endpoint)
        .map_err(|_| anyhow::anyhow!("invalid OpenCode relay endpoint"))?;
    anyhow::ensure!(
        url.scheme() == "http"
            && url.host_str() == Some("127.0.0.1")
            && url.port().is_some()
            && url.username().is_empty()
            && url.password().is_none()
            && url.path() == "/"
            && url.query().is_none()
            && url.fragment().is_none(),
        "OpenCode requires an owned loopback relay"
    );
    let api = profile
        .model_api
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("OpenCode requires selected model transport"))?;
    let (npm, suffix) = match api.adapter.as_str() {
        "openai-compatible" => ("@ai-sdk/openai-compatible", "/v1"),
        "openai-responses" => ("@ai-sdk/openai", "/v1"),
        "openai" => {
            let upstream = reqwest::Url::parse(&api.url)
                .map_err(|_| anyhow::anyhow!("invalid OpenCode upstream transport"))?;
            if upstream.path().trim_end_matches('/').ends_with("/responses") {
                ("@ai-sdk/openai", "/v1")
            } else {
                ("@ai-sdk/openai-compatible", "/v1")
            }
        }
        "anthropic" => ("@ai-sdk/anthropic", "/v1"),
        "google" => ("@ai-sdk/google", "/v1beta"),
        _ => anyhow::bail!("OpenCode selected model protocol is unsupported"),
    };
    let root = profile.state_root.as_deref().unwrap_or(profile.directory());
    anyhow::ensure!(root.is_absolute() && root.is_dir(), "invalid OpenCode state root");
    use sha2::Digest;
    let identity = &profile.binding.identity;
    let reference = hex::encode(sha2::Sha256::digest(serde_json::to_vec(&json!({
        "tenant": identity.tenant, "project": identity.project, "session": identity.session,
        "source": "session_model",
    }))?));
    let mut state = root.to_path_buf();
    for component in ["acp-opencode-state", &reference] {
        state.push(component);
        private_directory(&state)?;
    }
    let model = format!("session-provider/{}", profile.model_alias);
    let agents: serde_json::Map<String, serde_json::Value> =
        ["build", "plan", "general", "explore", "title", "summary", "compaction"]
            .into_iter()
            .map(|name| (name.into(), json!({"model": model})))
            .collect();
    // The relay authenticates the catalog key and substitutes the upstream API ID.
    let mut model_config = json!({
        "name": "Session model", "id": profile.model_alias,
        "limit": {"context": profile.model_context_limit.max(1), "output": profile.model_output_limit.max(1)},
        "tool_call": profile.model_supports_tool_call, "temperature": profile.model_supports_temperature,
        "reasoning": profile.model_supports_reasoning,
    });
    if npm == "@ai-sdk/openai-compatible" && crate::model_relay::chat_url(api).is_ok() {
        // A reasoning boolean would otherwise make OpenCode infer Chat variants.
        model_config["reasoning_options"] = json!([]);
        let provider = &profile.binding.identity.provider;
        if profile.model_supports_reasoning
            && let Some(declared) = profile.model_reasoning_options.as_deref()
            && reasoning_profile(&api.id, &api.adapter, provider, &api.url).format
                == ReasoningFormat::OpenAi
        {
            let levels =
                reasoning_levels(&api.id, &api.adapter, provider, &api.url, Some(declared));
            if !levels.is_empty() {
                // The native ACP selects the first variant unless "default" exists.
                let mut variants = serde_json::Map::from_iter([("default".into(), json!({}))]);
                variants.extend(levels.into_iter().map(|level| {
                    let settings = json!({"reasoningEffort": level});
                    (level, settings)
                }));
                model_config["variants"] = variants.into();
            }
        }
    }
    let config = json!({
        "model": model, "small_model": model, "enabled_providers": ["session-provider"],
        "agent": agents, "autoupdate": false, "share": "disabled",
        "provider": {"session-provider": {"npm": npm, "env": [], "whitelist": [profile.model_alias],
            "options": {"baseURL": format!("{endpoint}{suffix}"), "apiKey": "{env:VW_OPENCODE_RELAY_TOKEN}"},
            "models": {profile.model_alias.clone(): model_config}
        }}
    });
    let mut env = HashMap::from([
        ("OPENCODE_CONFIG_CONTENT".into(), serde_json::to_string(&config)?),
        ("VW_OPENCODE_RELAY_TOKEN".into(), profile.binding.bearer_token().to_owned()),
        ("OPENCODE_CONFIG".into(), String::new()),
        ("OPENCODE_DISABLE_PROJECT_CONFIG".into(), "1".into()),
        ("OPENCODE_DISABLE_MODELS_FETCH".into(), "1".into()),
        ("OPENCODE_DISABLE_AUTOUPDATE".into(), "1".into()),
        ("NODE_OPTIONS".into(), String::new()),
        ("BUN_OPTIONS".into(), String::new()),
    ]);
    for (key, name, persistent) in [
        ("XDG_CONFIG_HOME", "xdg-config", false),
        ("XDG_DATA_HOME", "data", true),
        ("XDG_STATE_HOME", "state", true),
        ("XDG_CACHE_HOME", "cache", true),
        ("OPENCODE_CONFIG_DIR", "config", false),
        ("OPENCODE_TEST_HOME", "home", false),
    ] {
        let path = if persistent { state.join(name) } else { profile.directory().join(name) };
        private_directory(&path)?;
        env.insert(key.into(), path.to_string_lossy().into_owned());
    }
    for key in ["HTTP_PROXY", "HTTPS_PROXY", "ALL_PROXY", "http_proxy", "https_proxy", "all_proxy"]
    {
        env.insert(key.into(), String::new());
    }
    env.insert("NO_PROXY".into(), "127.0.0.1,localhost".into());
    env.insert("no_proxy".into(), "127.0.0.1,localhost".into());
    profile.overlay_environment(env)
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
            "insecure OpenCode state directory"
        );
        Ok(())
    }
    #[cfg(not(unix))]
    anyhow::bail!("OpenCode private state ACL verification unavailable")
}

/// System policy wins over inline JSON. Refuse it rather than bypassing policy or
/// allowing its model/account to silently replace the session-owned provider.
#[cfg(unix)]
fn reject_managed_configuration() -> anyhow::Result<()> {
    anyhow::ensure!(
        std::env::var_os("OPENCODE_TEST_MANAGED_CONFIG_DIR").is_none_or(|value| value.is_empty()),
        "OpenCode inherited managed configuration override is unsupported"
    );
    let directory = if cfg!(target_os = "macos") {
        Path::new("/Library/Application Support/opencode")
    } else {
        Path::new("/etc/opencode")
    };
    let mut paths = vec![directory.join("opencode.json"), directory.join("opencode.jsonc")];
    if cfg!(target_os = "macos") {
        // Match Node's OS account lookup, not a spoofable USER environment value.
        let mut account: libc::passwd = unsafe { std::mem::zeroed() };
        let mut found = std::ptr::null_mut();
        let mut buffer = vec![0u8; 65536];
        let status = unsafe {
            libc::getpwuid_r(
                libc::geteuid(),
                &mut account,
                buffer.as_mut_ptr().cast(),
                buffer.len(),
                &mut found,
            )
        };
        anyhow::ensure!(
            status == 0 && !found.is_null() && !account.pw_name.is_null(),
            "OpenCode managed policy account lookup failed"
        );
        let username =
            unsafe { std::ffi::CStr::from_ptr(account.pw_name) }.to_str().map_err(|_| {
                anyhow::anyhow!("OpenCode managed policy account encoding is unsupported")
            })?;
        let preferences = Path::new("/Library/Managed Preferences");
        paths.push(preferences.join("ai.opencode.managed.plist"));
        paths.push(preferences.join(username).join("ai.opencode.managed.plist"));
    }
    reject_existing_configuration(&paths)
}

#[cfg(not(unix))]
fn reject_managed_configuration() -> anyhow::Result<()> {
    anyhow::bail!("OpenCode managed policy isolation is unverified on this platform")
}

#[cfg(any(unix, test))]
fn reject_existing_configuration(paths: &[std::path::PathBuf]) -> anyhow::Result<()> {
    for path in paths {
        anyhow::ensure!(
            !path.try_exists()?,
            "OpenCode system-managed configuration cannot be isolated"
        );
    }
    Ok(())
}

#[cfg(test)]
#[path = "opencode_tests.rs"]
mod tests;
