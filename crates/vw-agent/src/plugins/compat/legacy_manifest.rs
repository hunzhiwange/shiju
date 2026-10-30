//! Codex、Claude 与 Cursor 旧版清单的声明归一化；不执行声明内容。

use anyhow::{Context, Result, bail};
use serde::Deserialize;
use serde_json::Value;
use std::path::{Component, Path};

use super::super::audit::resolve_component_path;
use super::super::manifest::{RawPluginInterface, normalize_interface};
use super::super::types::{
    PluginHookSource, PluginInterface, PluginManifest, PluginMcpServers, PluginPath,
};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct LegacyManifest {
    #[serde(default)]
    name: String,
    version: Option<String>,
    description: Option<String>,
    #[serde(default)]
    keywords: Vec<String>,
    skills: Option<Value>,
    mcp_servers: Option<Value>,
    hooks: Option<Value>,
    apps: Option<String>,
    interface: Option<Value>,
    #[serde(default)]
    extensions: Value,
}

pub(crate) fn parse_legacy_manifest(
    root: &Path,
    path: &Path,
    bytes: &[u8],
) -> Result<PluginManifest> {
    let raw: LegacyManifest = serde_json::from_slice(bytes)
        .with_context(|| format!("failed to parse plugin manifest: {}", path.display()))?;
    let name = if raw.name.trim().is_empty() {
        root.file_name()
            .and_then(|name| name.to_str())
            .context("legacy plugin root has no valid basename")?
            .to_string()
    } else {
        raw.name
    };
    let interface = raw
        .interface
        .map(|mut interface| {
            let object = interface.as_object_mut().context("plugin interface must be an object")?;
            for field in ["capabilities", "screenshots"] {
                if object.get(field).is_some_and(Value::is_null) {
                    bail!("plugin interface field {field} must be a string array, not null");
                }
            }
            let prompts = normalize_default_prompts(object.remove("defaultPrompt"));
            object.insert("defaultPrompt".to_owned(), serde_json::to_value(prompts)?);
            let interface: RawPluginInterface =
                serde_json::from_value(interface).with_context(|| {
                    format!("failed to parse plugin manifest interface: {}", path.display())
                })?;
            normalize_interface(root, interface)
        })
        .transpose()?
        .filter(|interface| interface != &PluginInterface::default());
    let mcp_servers = match raw.mcp_servers {
        Some(Value::String(path)) => {
            Some(PluginMcpServers::Path(legacy_path(root, "mcpServers", &path)?))
        }
        Some(Value::Object(servers)) => Some(PluginMcpServers::Inline(servers)),
        None => None,
        Some(_) => {
            tracing::warn!("ignoring mcpServers: expected a string or object");
            None
        }
    };
    let onboarding_skill = raw
        .extensions
        .get("com.openai")
        .and_then(|extension| extension.get("onboardingSkill"))
        .and_then(Value::as_str)
        .map(|path| {
            let path = format!("./{}", path.strip_prefix("./").unwrap_or(path));
            legacy_path(root, "extensions.com.openai.onboardingSkill", &path)
        })
        .transpose()?;
    Ok(PluginManifest {
        root: root.to_path_buf(),
        manifest_path: path.to_path_buf(),
        name,
        version: raw
            .version
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty()),
        description: raw.description,
        author: None,
        homepage: None,
        repository: None,
        license: None,
        keywords: raw.keywords,
        skills: paths(root, "skills", raw.skills)?,
        mcp_servers,
        hooks: hooks(root, raw.hooks)?,
        apps: raw.apps.map(|path| legacy_path(root, "apps", &path)).transpose()?,
        onboarding_skill,
        interface,
    })
}

fn paths(root: &Path, field: &str, value: Option<Value>) -> Result<Vec<PluginPath>> {
    let paths = match value {
        Some(Value::String(path)) => vec![path],
        Some(Value::Array(paths)) if paths.iter().all(Value::is_string) => paths
            .into_iter()
            .map(|path| path.as_str().expect("string checked").to_owned())
            .collect(),
        None => return Ok(Vec::new()),
        Some(_) => {
            tracing::warn!(field, "ignoring plugin component: expected a string or string array");
            return Ok(Vec::new());
        }
    };
    paths.iter().map(|path| legacy_path(root, field, path)).collect()
}

fn legacy_path(root: &Path, field: &str, path: &str) -> Result<PluginPath> {
    let relative = path.strip_prefix("./").unwrap_or(path);
    if relative.is_empty()
        || Path::new(relative).components().any(|part| {
            matches!(part, Component::ParentDir | Component::RootDir | Component::Prefix(_))
        })
    {
        bail!(
            "Plugin manifest field {field} path escapes plugin root or contains a parent component: {path}"
        );
    }
    resolve_component_path(root, field, path)
}

fn hooks(root: &Path, value: Option<Value>) -> Result<Vec<PluginHookSource>> {
    match value {
        Some(Value::Object(hooks)) => Ok(vec![PluginHookSource::Inline(hooks)]),
        Some(Value::Array(hooks)) if hooks.iter().all(Value::is_object) => Ok(hooks
            .into_iter()
            .map(|hook| PluginHookSource::Inline(hook.as_object().expect("object checked").clone()))
            .collect()),
        other => paths(root, "hooks", other)
            .map(|paths| paths.into_iter().map(PluginHookSource::Path).collect()),
    }
}

fn normalize_default_prompts(value: Option<serde_json::Value>) -> Vec<String> {
    let values = match value {
        Some(serde_json::Value::String(prompt)) => vec![serde_json::Value::String(prompt)],
        Some(serde_json::Value::Array(prompts)) => prompts,
        None => return Vec::new(),
        Some(_) => {
            tracing::warn!("ignoring interface.defaultPrompt: expected a string or array");
            return Vec::new();
        }
    };
    let mut prompts = Vec::new();
    for value in values {
        if prompts.len() == 3 {
            tracing::warn!("ignoring interface.defaultPrompt entries after three valid prompts");
            break;
        }
        let Some(prompt) = value.as_str() else {
            tracing::warn!("ignoring interface.defaultPrompt entry: expected a string");
            continue;
        };
        let prompt = prompt.split_whitespace().collect::<Vec<_>>().join(" ");
        if prompt.is_empty() || prompt.chars().count() > 128 {
            tracing::warn!("ignoring interface.defaultPrompt entry: expected 1 to 128 characters");
            continue;
        }
        prompts.push(prompt);
    }
    prompts
}
