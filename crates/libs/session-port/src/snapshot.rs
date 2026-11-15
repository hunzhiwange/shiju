//! 已验证会话快照与规格持久化契约。

use serde::{Deserialize, Serialize};
use tool_types::ToolSpec;

/// Complete persisted replacement, including an explicitly empty tool collection.
/// Executors, connection credentials and pending calls are deliberately absent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersistedDynamicTools {
    pub version: u64,
    pub specs: Vec<ToolSpec>,
}
