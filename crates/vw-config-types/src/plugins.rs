//! Plugin discovery configuration.

use schemars::JsonSchema;
use serde::{Deserialize, Deserializer, Serialize};
use std::collections::HashSet;

/// Local plugin discovery configuration.
#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
pub struct PluginsConfig {
    /// Plugin names explicitly enabled by configuration. Empty means every discovered plugin is
    /// eligible unless a disabled marker is present.
    #[serde(default, deserialize_with = "deduplicate_trimmed_strings")]
    pub enabled: Vec<String>,

    /// Additional plugin source directories.
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
        if seen.insert(trimmed.clone()) {
            out.push(trimmed);
        }
    }

    out
}

#[cfg(test)]
#[path = "plugins_tests.rs"]
mod plugins_tests;
