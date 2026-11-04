//! 智能体运行引擎与新会话默认选择；权限值沿用现有 permission JSON 规则。
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// 绑定的运行引擎。ACP backend 使用目录中的稳定 key。
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum AgentRuntime {
    #[default]
    Native,
    Acp {
        backend: String,
    },
}

impl<'de> Deserialize<'de> for AgentRuntime {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        // A struct-shaped native variant also rejects fields that only ACP accepts.
        #[derive(Deserialize)]
        #[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
        enum RuntimeShape {
            Native {},
            Acp {
                #[serde(deserialize_with = "nonempty_string")]
                backend: String,
            },
        }
        Ok(match RuntimeShape::deserialize(deserializer)? {
            RuntimeShape::Native {} => Self::Native,
            RuntimeShape::Acp { backend } => Self::Acp { backend },
        })
    }
}

fn nonempty_string<'de, D: serde::Deserializer<'de>>(deserializer: D) -> Result<String, D::Error> {
    let value = String::deserialize(deserializer)?;
    if value.trim().is_empty() {
        return Err(serde::de::Error::custom("reference must not be empty"));
    }
    Ok(value)
}

/// 完整模型引用，不携带凭据。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AgentModelReference {
    #[serde(deserialize_with = "nonempty_string")]
    pub provider: String,
    #[serde(deserialize_with = "nonempty_string")]
    pub model: String,
}

/// 继承环境默认，或显式固定一个值；固定空列表表示不选择任何条目。
#[derive(Clone, PartialEq, Eq, Default, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "mode", content = "value", rename_all = "snake_case", deny_unknown_fields)]
pub enum AgentDefault<T> {
    #[default]
    Inherit,
    Fixed(T),
}

// 默认值可能包含权限路径等私有内容，不将载荷写入日志。
impl<T> std::fmt::Debug for AgentDefault<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Inherit => "Inherit",
            Self::Fixed(_) => "Fixed([REDACTED])",
        })
    }
}

/// 智能体的新会话默认值；权限仍须在运行时受安全上限约束。
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize, JsonSchema)]
#[serde(default, deny_unknown_fields)]
pub struct AgentDefaults {
    pub model: AgentDefault<AgentModelReference>,
    pub permission: AgentDefault<Value>,
    /// 技能的稳定 ID。
    pub skills: AgentDefault<Vec<String>>,
    /// MCP 服务的稳定 ID，不包含连接配置或凭据。
    pub mcp: AgentDefault<Vec<String>>,
}
