use super::AcpLaunchProfile;
use serde_json::json;
use std::collections::HashMap;

pub(super) fn configure(profile: &mut AcpLaunchProfile, endpoint: &str) -> anyhow::Result<()> {
    let url = reqwest::Url::parse(endpoint)?;
    anyhow::ensure!(
        url.scheme() == "http"
            && url.host_str() == Some("127.0.0.1")
            && url.port().is_some()
            && url.username().is_empty()
            && url.password().is_none()
            && url.path() == "/"
            && url.query().is_none()
            && url.fragment().is_none(),
        "Qwen requires an owned loopback relay"
    );
    anyhow::ensure!(
        profile.config().args == ["--acp", "--experimental-skills", "--approval-mode", "yolo"],
        "Qwen requires the pinned ACP arguments"
    );
    anyhow::ensure!(
        !profile.config().env.keys().any(|key| {
            let key = key.to_ascii_uppercase();
            ["QWEN_", "OPENAI_", "NODE_", "SHIJU_QWEN_"]
                .iter()
                .any(|prefix| key.starts_with(prefix))
        }),
        "conflicting Qwen model environment"
    );
    let reference = profile.config().env[super::super::profile::REFERENCE_ENV]
        .strip_prefix("vw-session-model:")
        .ok_or_else(|| anyhow::anyhow!("missing Qwen session identity"))?;
    let mut nonce = [0u8; 16];
    getrandom::fill(&mut nonce).map_err(|_| anyhow::anyhow!("Qwen alias entropy unavailable"))?;
    let alias = format!("shiju-{}", hex::encode(nonce));
    let home = profile.directory().to_string_lossy().into_owned();
    let runtime = profile
        .directory()
        .parent()
        .ok_or_else(|| anyhow::anyhow!("missing Qwen runtime parent"))?
        .join("shiju-acp-qwen-history")
        .join(reference);
    #[cfg(unix)]
    {
        use std::os::unix::fs::{DirBuilderExt, MetadataExt, PermissionsExt};
        for path in [runtime.parent().unwrap(), runtime.as_path()] {
            match std::fs::DirBuilder::new().mode(0o700).create(path) {
                Ok(()) => {}
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
                Err(error) => return Err(error.into()),
            }
            let metadata = path.symlink_metadata()?;
            anyhow::ensure!(
                metadata.is_dir()
                    && !metadata.file_type().is_symlink()
                    && metadata.uid() == unsafe { libc::geteuid() }
                    && metadata.permissions().mode() & 0o777 == 0o700,
                "insecure Qwen runtime"
            );
        }
    }
    #[cfg(not(unix))]
    anyhow::bail!("Qwen runtime ACL verification unavailable");
    #[cfg(target_os = "macos")]
    let system = std::path::Path::new("/Library/Application Support/QwenCode/settings.json");
    #[cfg(not(target_os = "macos"))]
    let system = std::path::Path::new("/etc/qwen-code/settings.json");
    let defaults = system.with_file_name("system-defaults.json");
    let mut administrator = json!({});
    for path in [&defaults, &system.to_path_buf()] {
        match std::fs::read(path) {
            Ok(bytes) => {
                let value: serde_json::Value = serde_json::from_slice(&bytes)
                    .map_err(|_| anyhow::anyhow!("invalid Qwen administrator settings"))?;
                anyhow::ensure!(
                    value.is_object()
                        && value.get("model").is_none()
                        && value.get("modelProviders").is_none()
                        && value.get("modelFallbacks").is_none()
                        && value.pointer("/security/auth").is_none(),
                    "Qwen administrator routing conflicts with session_model"
                );
                if path == system {
                    administrator = value;
                }
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(_) => anyhow::bail!("Qwen administrator settings unavailable"),
        }
    }
    let generation = json!({"model":alias,"contextWindowSize":profile.model_context_limit,
        "samplingParams":{"max_tokens":profile.model_output_limit},
        "extra_body":{"model":profile.model_alias}});
    let managed = json!({
        "security":{"auth":{"selectedType":"openai"}},
        "model":{"name":alias,"enableOpenAILogging":false},
        "modelProviders":{"openai":[{"id":alias,"baseUrl":format!("{}/v1",endpoint.trim_end_matches('/')),
            "envKey":"OPENAI_API_KEY","generationConfig":{
                "contextWindowSize":profile.model_context_limit,
                "samplingParams":{"max_tokens":profile.model_output_limit},
                "extra_body":{"model":profile.model_alias}
            }}]},
        "general":{"enableAutoUpdate":false},
        "telemetry":{"enabled":false},
        "advanced":{"runtimeOutputDir":runtime}
    });
    for (key, value) in managed.as_object().unwrap() {
        if matches!(key.as_str(), "security" | "general" | "advanced" | "tools") {
            let section = administrator.as_object_mut().unwrap().entry(key).or_insert(json!({}));
            anyhow::ensure!(section.is_object(), "invalid Qwen administrator section");
            section.as_object_mut().unwrap().extend(value.as_object().unwrap().clone());
        } else {
            administrator[key] = value.clone();
        }
    }
    let settings = administrator;
    let path = profile.write_config("settings.json", &serde_json::to_vec(&settings)?)?;
    let mut environment = HashMap::from([
        ("QWEN_HOME".into(), home),
        ("QWEN_CODE_SYSTEM_DEFAULTS_PATH".into(), defaults.to_string_lossy().into_owned()),
        ("QWEN_RUNTIME_DIR".into(), runtime.to_string_lossy().into_owned()),
        ("QWEN_CODE_SYSTEM_SETTINGS_PATH".into(), path.to_string_lossy().into_owned()),
        ("OPENAI_API_KEY".into(), profile.binding.bearer_token().into()),
        ("OPENAI_BASE_URL".into(), format!("{}/v1", endpoint.trim_end_matches('/'))),
        ("OPENAI_MODEL".into(), alias.clone()),
        ("SHIJU_QWEN_MODEL_LOCK".into(), profile.model_alias.clone()),
        ("SHIJU_QWEN_GENERATION_CONFIG".into(), serde_json::to_string(&generation)?),
    ]);
    for key in [
        "QWEN_CODE_RELAUNCH_ARGS",
        "NODE_OPTIONS",
        "NODE_PATH",
        "HTTP_PROXY",
        "HTTPS_PROXY",
        "ALL_PROXY",
        "NO_PROXY",
        "http_proxy",
        "https_proxy",
        "all_proxy",
        "no_proxy",
    ] {
        environment.insert(key.into(), String::new());
    }
    profile.overlay_environment(environment)?;
    profile.append_arguments(&[
        "--shiju-model-source".into(),
        "--auth-type".into(),
        "openai".into(),
        "--model".into(),
        alias,
        "--openai-base-url".into(),
        format!("{}/v1", endpoint.trim_end_matches('/')),
    ])?;
    Ok(())
}

#[cfg(test)]
#[path = "qwen_tests.rs"]
mod tests;
