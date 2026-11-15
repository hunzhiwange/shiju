use serde::{Deserialize, Serialize};

/// 项目当前使用的版本控制系统类型。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Vcs {
    Git,
}

/// 项目图标与主题色配置。
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct Icon {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub override_icon: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
}

/// 项目级快捷命令配置。
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct Commands {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start: Option<String>,
}

/// 项目的创建与更新时间信息。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TimeInfo {
    pub created: u64,
    pub updated: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub initialized: Option<u64>,
}

/// 项目在共享层中的完整描述。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Info {
    pub id: String,
    pub worktree: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vcs: Option<Vcs>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon: Option<Icon>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub commands: Option<Commands>,
    pub time: TimeInfo,
    pub sandboxes: Vec<String>,
}

/// 更新项目元数据时使用的输入结构。
#[derive(Debug, Clone)]
pub struct UpdateInput {
    pub project_id: String,
    pub name: Option<Option<String>>,
    pub icon: Option<IconUpdate>,
    pub commands: Option<CommandsUpdate>,
}

/// 图标配置的增量更新载荷。
#[derive(Debug, Clone)]
pub struct IconUpdate {
    pub url: Option<Option<String>>,
    pub override_icon: Option<Option<String>>,
    pub color: Option<Option<String>>,
}

/// 命令配置的增量更新载荷。
#[derive(Debug, Clone)]
pub struct CommandsUpdate {
    pub start: Option<Option<String>>,
}

#[cfg(test)]
#[path = "project_tests.rs"]
mod project_tests;
