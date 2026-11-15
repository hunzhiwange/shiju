use serde::{Deserialize, Serialize};

/// 权限规则命中后的处理动作。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Action {
    Allow,
    Deny,
    Ask,
}

/// 单条权限匹配规则。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Rule {
    pub permission: String,
    pub pattern: String,
    pub action: Action,
}

/// 权限规则集合。
pub type Ruleset = Vec<Rule>;

#[cfg(test)]
#[path = "permission_tests.rs"]
mod permission_tests;
