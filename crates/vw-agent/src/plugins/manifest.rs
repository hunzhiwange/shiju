//! Parser for `.vibewindow-plugin/plugin.json`.

use anyhow::{Context, Result, bail};
use serde::Deserialize;
use std::fs;
use std::path::Path;

use super::audit::{canonical_plugin_root, plugin_manifest_path, resolve_component_path};
use super::types::{PluginAuthor, PluginInterface, PluginManifest, PluginPath};

/// Load and statically audit a local VibeWindow plugin manifest.
pub fn load_plugin_manifest(plugin_root: &Path) -> Result<PluginManifest> {
    let canonical_root = canonical_plugin_root(plugin_root)?;
    let manifest_path = plugin_manifest_path(&canonical_root)?;
    let manifest_bytes = fs::read(&manifest_path)
        .with_context(|| format!("failed to read plugin manifest: {}", manifest_path.display()))?;
    let raw: RawPluginManifest = serde_json::from_slice(&manifest_bytes)
        .with_context(|| format!("failed to parse plugin manifest: {}", manifest_path.display()))?;

    normalize_manifest(canonical_root.as_path(), manifest_path.as_path(), raw)
}

fn normalize_manifest(
    canonical_root: &Path,
    manifest_path: &Path,
    raw: RawPluginManifest,
) -> Result<PluginManifest> {
    let mut missing = Vec::new();
    let name = required_string("name", raw.name, &mut missing);
    let version = required_string("version", raw.version, &mut missing);
    let description = required_string("description", raw.description, &mut missing);

    if !missing.is_empty() {
        bail!("Plugin manifest missing required field(s): {}", missing.join(", "));
    }

    let interface = raw
        .interface
        .map(|interface| normalize_interface(canonical_root, interface))
        .transpose()?;

    Ok(PluginManifest {
        root: canonical_root.to_path_buf(),
        manifest_path: manifest_path.to_path_buf(),
        name: name.expect("required field checked"),
        version: version.expect("required field checked"),
        description: description.expect("required field checked"),
        author: raw.author,
        homepage: raw.homepage,
        repository: raw.repository,
        license: raw.license,
        keywords: raw.keywords.unwrap_or_default(),
        skills: raw
            .skills
            .map(|path| resolve_component_path(canonical_root, "skills", &path))
            .transpose()?,
        mcp_servers: raw
            .mcp_servers
            .map(|path| resolve_component_path(canonical_root, "mcpServers", &path))
            .transpose()?,
        hooks: normalize_paths(canonical_root, "hooks", raw.hooks)?,
        interface,
    })
}

fn required_string(
    field: &'static str,
    value: Option<String>,
    missing: &mut Vec<&'static str>,
) -> Option<String> {
    let value = value.map(|value| value.trim().to_string()).filter(|value| !value.is_empty());
    if value.is_none() {
        missing.push(field);
    }
    value
}

fn normalize_paths(
    canonical_root: &Path,
    field: &str,
    paths: Option<RawPathList>,
) -> Result<Vec<PluginPath>> {
    match paths {
        Some(RawPathList::One(path)) => {
            Ok(vec![resolve_component_path(canonical_root, field, &path)?])
        }
        Some(RawPathList::Many(paths)) => paths
            .into_iter()
            .map(|path| resolve_component_path(canonical_root, field, &path))
            .collect(),
        None => Ok(Vec::new()),
    }
}

fn normalize_interface(canonical_root: &Path, raw: RawPluginInterface) -> Result<PluginInterface> {
    Ok(PluginInterface {
        display_name: raw.display_name,
        short_description: raw.short_description,
        long_description: raw.long_description,
        developer_name: raw.developer_name,
        category: raw.category,
        capabilities: raw.capabilities.unwrap_or_default(),
        website_url: raw.website_url,
        privacy_policy_url: raw.privacy_policy_url,
        terms_of_service_url: raw.terms_of_service_url,
        default_prompt: raw.default_prompt.unwrap_or_default(),
        brand_color: raw.brand_color,
        composer_icon: raw
            .composer_icon
            .map(|path| resolve_component_path(canonical_root, "interface.composerIcon", &path))
            .transpose()?,
        logo: raw
            .logo
            .map(|path| resolve_component_path(canonical_root, "interface.logo", &path))
            .transpose()?,
        screenshots: raw
            .screenshots
            .unwrap_or_default()
            .into_iter()
            .map(|path| resolve_component_path(canonical_root, "interface.screenshots", &path))
            .collect::<Result<Vec<_>>>()?,
    })
}

#[derive(Debug, Deserialize)]
struct RawPluginManifest {
    name: Option<String>,
    version: Option<String>,
    description: Option<String>,
    #[serde(default)]
    author: Option<PluginAuthor>,
    #[serde(default)]
    homepage: Option<String>,
    #[serde(default)]
    repository: Option<String>,
    #[serde(default)]
    license: Option<String>,
    #[serde(default)]
    keywords: Option<Vec<String>>,
    #[serde(default)]
    skills: Option<String>,
    #[serde(default, rename = "mcpServers")]
    mcp_servers: Option<String>,
    #[serde(default)]
    hooks: Option<RawPathList>,
    #[serde(default)]
    interface: Option<RawPluginInterface>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum RawPathList {
    One(String),
    Many(Vec<String>),
}

#[derive(Debug, Deserialize)]
struct RawPluginInterface {
    #[serde(default, rename = "displayName")]
    display_name: Option<String>,
    #[serde(default, rename = "shortDescription")]
    short_description: Option<String>,
    #[serde(default, rename = "longDescription")]
    long_description: Option<String>,
    #[serde(default, rename = "developerName")]
    developer_name: Option<String>,
    #[serde(default)]
    category: Option<String>,
    #[serde(default)]
    capabilities: Option<Vec<String>>,
    #[serde(default, rename = "websiteURL")]
    website_url: Option<String>,
    #[serde(default, rename = "privacyPolicyURL")]
    privacy_policy_url: Option<String>,
    #[serde(default, rename = "termsOfServiceURL")]
    terms_of_service_url: Option<String>,
    #[serde(default, rename = "defaultPrompt")]
    default_prompt: Option<Vec<String>>,
    #[serde(default, rename = "brandColor")]
    brand_color: Option<String>,
    #[serde(default, rename = "composerIcon")]
    composer_icon: Option<String>,
    #[serde(default)]
    logo: Option<String>,
    #[serde(default)]
    screenshots: Option<Vec<String>>,
}
