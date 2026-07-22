//! Plugin manifest data types.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// A manifest path after static path auditing.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PluginPath {
    pub raw: String,
    pub canonical_path: PathBuf,
}

/// A parsed plugin manifest.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PluginManifest {
    pub root: PathBuf,
    pub manifest_path: PathBuf,
    pub name: String,
    pub version: String,
    pub description: String,
    pub author: Option<PluginAuthor>,
    pub homepage: Option<String>,
    pub repository: Option<String>,
    pub license: Option<String>,
    pub keywords: Vec<String>,
    pub skills: Option<PluginPath>,
    pub mcp_servers: Option<PluginPath>,
    pub hooks: Vec<PluginPath>,
    pub interface: Option<PluginInterface>,
}

/// Plugin author metadata.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum PluginAuthor {
    Name(String),
    Details(PluginAuthorDetails),
}

/// Structured plugin author metadata.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PluginAuthorDetails {
    pub name: String,
    #[serde(default)]
    pub email: Option<String>,
    #[serde(default)]
    pub url: Option<String>,
}

/// Install-surface metadata from the plugin manifest.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PluginInterface {
    pub display_name: Option<String>,
    pub short_description: Option<String>,
    pub long_description: Option<String>,
    pub developer_name: Option<String>,
    pub category: Option<String>,
    pub capabilities: Vec<String>,
    pub website_url: Option<String>,
    pub privacy_policy_url: Option<String>,
    pub terms_of_service_url: Option<String>,
    pub default_prompt: Vec<String>,
    pub brand_color: Option<String>,
    pub composer_icon: Option<PluginPath>,
    pub logo: Option<PluginPath>,
    pub screenshots: Vec<PluginPath>,
}
