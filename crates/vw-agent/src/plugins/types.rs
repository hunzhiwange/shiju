//! 插件清单与已审计路径的数据类型。

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// 经过静态审计后的清单路径，同时保留原始声明供展示。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PluginPath {
    pub raw: String,
    pub canonical_path: PathBuf,
}

/// 已解析并完成组件路径审计的插件清单。
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

/// 兼容简写名称与结构化形式的插件作者信息。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum PluginAuthor {
    Name(String),
    Details(PluginAuthorDetails),
}

/// 结构化插件作者元数据。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PluginAuthorDetails {
    pub name: String,
    #[serde(default)]
    pub email: Option<String>,
    #[serde(default)]
    pub url: Option<String>,
}

/// 插件清单中面向安装与展示界面的元数据。
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
