//! 插件清单与已审计路径的数据类型。

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// 本地插件保持原名称；市场插件以名称与市场共同确定身份。
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PluginIdentity {
    pub(super) name: String,
    pub(super) marketplace: Option<String>,
}

/// 保存在原包之外的安装来源与版本记录。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PluginInstallMetadata {
    pub identity: PluginIdentity,
    pub source: String,
    pub version: Option<String>,
}

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
    pub version: Option<String>,
    pub description: Option<String>,
    pub author: Option<PluginAuthor>,
    pub homepage: Option<String>,
    pub repository: Option<String>,
    pub license: Option<String>,
    pub keywords: Vec<String>,
    pub skills: Vec<PluginPath>,
    pub mcp_servers: Option<PluginMcpServers>,
    pub hooks: Vec<PluginHookSource>,
    pub apps: Option<PluginPath>,
    pub onboarding_skill: Option<PluginPath>,
    pub interface: Option<PluginInterface>,
}

/// MCP 声明保留原始内联对象，执行适配器负责协议校验。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PluginMcpServers {
    Path(PluginPath),
    Inline(serde_json::Map<String, serde_json::Value>),
}

/// Hooks 文件与内联声明的来源不会混为一个文件路径。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PluginHookSource {
    Path(PluginPath),
    Inline(serde_json::Map<String, serde_json::Value>),
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
#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
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
    pub logo_dark: Option<PluginPath>,
    pub screenshots: Vec<PluginPath>,
}
