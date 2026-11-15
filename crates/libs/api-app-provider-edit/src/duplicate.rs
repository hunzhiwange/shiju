//! Duplicate within the authenticated tenant and application; never activate.
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AppProviderDuplicate {
    pub new_id: String,
    pub name: String,
}
