//! Agent Plugins 根清单及 Codex overlay 的声明归一化，不执行插件内容。

use anyhow::{Context, Result, bail};
use serde::Deserialize;
use serde_json::{Value, json};
use std::fs;
use std::path::Path;

use super::super::audit::resolve_component_path;
use super::super::types::{PluginInterface, PluginManifest, PluginMcpServers, PluginPath};
use super::legacy_manifest::parse_legacy_manifest;

const SCHEMA: &str = "https://agent-plugins.org/schemas/1.0.0/plugin.schema.json";
pub(super) const SCHEMA_PREFIX: &str = "https://agent-plugins.org/schemas/";
const FIELDS: &[&str] = &[
    "$schema",
    "name",
    "version",
    "description",
    "author",
    "homepage",
    "repository",
    "license",
    "keywords",
    "extensions",
];

#[derive(Deserialize)]
struct PortableManifest {
    name: String,
    version: Option<String>,
    description: Option<String>,
    author: Option<PortableAuthor>,
    homepage: Option<String>,
    #[serde(rename = "repository")]
    _repository: Option<String>,
    #[serde(rename = "license")]
    _license: Option<String>,
    #[serde(default)]
    keywords: Vec<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PortableAuthor {
    name: Option<String>,
    #[serde(rename = "email")]
    _email: Option<String>,
    #[serde(rename = "url")]
    _url: Option<String>,
}

/// Unrelated root JSON may fall back; a selected portable manifest never does.
pub(crate) fn load_portable_manifest(root: &Path) -> Result<Option<PluginManifest>> {
    let path = root.join("plugin.json");
    match fs::symlink_metadata(&path) {
        Ok(metadata) => {
            if metadata.file_type().is_symlink() {
                bail!("Plugin manifest must not be a symlink: {}", path.display());
            }
            if !metadata.is_file() {
                bail!("Plugin manifest must be a file: {}", path.display());
            }
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => {
            return Err(error)
                .with_context(|| format!("failed to inspect plugin manifest: {}", path.display()));
        }
    }
    let bytes = fs::read(&path)
        .with_context(|| format!("failed to read plugin manifest: {}", path.display()))?;
    let Ok(Value::Object(object)) = serde_json::from_slice::<Value>(&bytes) else {
        return Ok(None);
    };
    let Some(schema) = object.get("$schema").and_then(Value::as_str) else {
        return Ok(None);
    };
    if !schema.starts_with(SCHEMA_PREFIX) {
        return Ok(None);
    }
    if schema != SCHEMA {
        bail!("unsupported Agent Plugins schema `{schema}`; supported schema: `{SCHEMA}`");
    }
    for field in object.keys().filter(|field| !FIELDS.contains(&field.as_str())) {
        tracing::warn!(path = %path.display(), field, "ignoring unknown Agent Plugins manifest field");
    }
    for field in ["version", "description", "author", "homepage", "repository", "license"] {
        if object.get(field).is_some_and(Value::is_null) {
            bail!("Agent Plugins `{field}` must use its declared type when present");
        }
    }
    if let Some(author) = object.get("author").and_then(Value::as_object) {
        for field in ["name", "email", "url"] {
            if author.get(field).is_some_and(Value::is_null) {
                bail!("Agent Plugins `author.{field}` must be a string when present");
            }
        }
    }
    let inline = match object.get("extensions") {
        Some(Value::Object(extensions)) => match extensions.get("com.openai") {
            Some(Value::Object(extension)) => Some(Value::Object(extension.clone())),
            Some(_) => {
                tracing::warn!("ignoring non-object Agent Plugins com.openai extension");
                None
            }
            None => None,
        },
        Some(_) => {
            tracing::warn!("ignoring non-object Agent Plugins extensions");
            None
        }
        None => None,
    };
    let raw: PortableManifest = serde_json::from_value(Value::Object(object))
        .with_context(|| format!("failed to parse Agent Plugins manifest: {}", path.display()))?;
    if !valid_name(&raw.name) {
        bail!(
            "invalid Agent Plugins name `{}`; use lowercase letters, numbers, dots, or hyphens",
            raw.name
        );
    }
    let description = raw.description.and_then(non_empty_trimmed);
    let mut manifest = PluginManifest {
        root: root.to_path_buf(),
        manifest_path: path.clone(),
        name: raw.name.clone(),
        version: raw.version.and_then(non_empty_trimmed),
        description: description.clone(),
        author: None,
        homepage: None,
        repository: None,
        license: None,
        keywords: raw.keywords,
        skills: vec![fixed_component(root, "skills")?],
        mcp_servers: Some(PluginMcpServers::Path(fixed_component(root, "mcp.json")?)),
        hooks: Vec::new(),
        apps: None,
        onboarding_skill: None,
        interface: Some(PluginInterface {
            display_name: Some(raw.name),
            short_description: description.clone(),
            long_description: description,
            developer_name: raw.author.and_then(|author| author.name).and_then(non_empty_trimmed),
            category: Some("Other".to_owned()),
            website_url: raw.homepage.and_then(non_empty_trimmed),
            ..PluginInterface::default()
        }),
    };
    let extension = if let Some(mut inline) = inline {
        // The inline namespace exposes onboardingSkill directly, unlike a legacy overlay.
        let onboarding = inline.get("onboardingSkill").cloned().unwrap_or(Value::Null);
        inline["extensions"] = json!({"com.openai": {"onboardingSkill": onboarding}});
        Some(parse_legacy_manifest(root, &path, &serde_json::to_vec(&inline)?)?)
    } else {
        load_overlay(root)?
    };
    if let Some(extension) = extension {
        manifest.apps = extension.apps;
        manifest.hooks = extension.hooks;
        manifest.onboarding_skill = extension.onboarding_skill;
        if extension.interface.is_some() {
            manifest.interface = extension.interface;
        }
    }
    Ok(Some(manifest))
}

fn non_empty_trimmed(value: String) -> Option<String> {
    let value = value.trim();
    (!value.is_empty()).then(|| value.to_owned())
}

fn valid_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 64
        && !name.contains("--")
        && !name.contains("..")
        && name
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || b".-".contains(&byte))
        && name.as_bytes().first().is_some_and(u8::is_ascii_alphanumeric)
        && name.as_bytes().last().is_some_and(u8::is_ascii_alphanumeric)
}

fn fixed_component(root: &Path, name: &str) -> Result<PluginPath> {
    let raw = format!("./{name}");
    match fs::symlink_metadata(root.join(name)) {
        Ok(_) => resolve_component_path(root, name, &raw),
        // These two fixed, root-level conventions need not exist at parse time.
        // No user-supplied path bypasses the existing strict component audit.
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            Ok(PluginPath { raw, canonical_path: root.join(name) })
        }
        Err(error) => Err(error).with_context(|| format!("failed to inspect portable {name}")),
    }
}

fn load_overlay(root: &Path) -> Result<Option<PluginManifest>> {
    let directory = root.join(".codex-plugin");
    match fs::symlink_metadata(&directory) {
        Ok(metadata) => {
            if metadata.file_type().is_symlink() {
                bail!("Plugin overlay directory must not be a symlink: {}", directory.display());
            }
            if !metadata.is_dir() {
                bail!("Plugin overlay parent must be a directory: {}", directory.display());
            }
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error).context("failed to inspect Codex overlay directory"),
    }
    let path = directory.join("plugin.json");
    match fs::symlink_metadata(&path) {
        Ok(metadata) => {
            if !metadata.is_file() || metadata.file_type().is_symlink() {
                bail!("Plugin overlay must be a regular file, not a symlink: {}", path.display());
            }
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error).context("failed to inspect Codex overlay"),
    }
    let path = resolve_component_path(root, "overlay", "./.codex-plugin/plugin.json")?;
    let bytes = fs::read(&path.canonical_path).context("failed to read Codex overlay")?;
    parse_legacy_manifest(root, &path.canonical_path, &bytes).map(Some)
}
