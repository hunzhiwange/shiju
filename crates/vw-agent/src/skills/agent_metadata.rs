//! Plugin skill declarations are metadata, never delegated-agent or tool grants.

use serde::Serialize;
use std::path::PathBuf;

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct SkillAgentMetadata {
    pub interface: Option<SkillAgentInterface>,
    pub dependencies: Vec<SkillAgentDependency>,
    pub policy: Option<SkillAgentPolicy>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct SkillAgentInterface {
    pub display_name: Option<String>,
    pub short_description: Option<String>,
    pub icon_small: Option<PathBuf>,
    pub icon_large: Option<PathBuf>,
    pub brand_color: Option<String>,
    pub default_prompt: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SkillAgentDependency {
    pub r#type: String,
    pub value: String,
    pub description: Option<String>,
    pub transport: Option<String>,
    pub command: Option<String>,
    pub url: Option<String>,
    pub oauth_callback_port: Option<u16>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct SkillAgentPolicy {
    pub allow_implicit_invocation: Option<bool>,
    pub products: Vec<String>,
}
