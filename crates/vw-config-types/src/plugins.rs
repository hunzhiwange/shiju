//! Plugin discovery configuration.
//! 定义插件发现来源，并在反序列化时清理空值和重复路径。
//! 本模块只描述候选插件的筛选范围，不负责加载或执行插件。

use schemars::JsonSchema;
use serde::{Deserialize, Deserializer, Serialize};
use std::collections::HashSet;

/// 本地插件发现配置。
#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
pub struct PluginsConfig {
    /// 配置中显式启用的插件名。
    ///
    /// 空列表表示所有已发现且未标记为禁用的插件均可进入候选集。
    #[serde(default, deserialize_with = "deduplicate_trimmed_strings")]
    pub enabled: Vec<String>,

    /// 参与发现的附加插件来源目录。
    #[serde(default, deserialize_with = "deduplicate_trimmed_strings")]
    pub sources: Vec<String>,
}

fn deduplicate_trimmed_strings<'de, D>(deserializer: D) -> Result<Vec<String>, D::Error>
where
    D: Deserializer<'de>,
{
    let values = Vec::<String>::deserialize(deserializer)?;
    Ok(deduplicate_trimmed(values))
}

fn deduplicate_trimmed(values: Vec<String>) -> Vec<String> {
    let mut seen = HashSet::new();
    let mut out = Vec::new();

    for value in values {
        let trimmed = value.trim().to_string();
        // 保留首次出现的顺序，使配置合并后的插件发现顺序保持稳定。
        if seen.insert(trimmed.clone()) {
            out.push(trimmed);
        }
    }

    out
}

#[cfg(test)]
#[path = "plugins_tests.rs"]
mod plugins_tests;
