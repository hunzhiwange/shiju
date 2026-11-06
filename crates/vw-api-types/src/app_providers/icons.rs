//! Selecting a library entry copies its inline PNG, so later deletion cannot dangle a reference.
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AppProviderIcon {
    pub data: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "camelCase", deny_unknown_fields)]
pub enum AppProviderIconChange {
    Select {
        #[serde(rename = "iconId")]
        icon_id: String,
    },
    Clear,
}
