//! Executor-retained editor context. Requests contain only explicit user edits, never a baseline.
use serde::{Deserialize, Serialize};
use serde_json::Value;
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EditorOpen {
    pub target_host_id: String,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EditorEdit {
    pub path: Vec<String>,
    pub value: EditorValue,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum EditorValue {
    Set { value: Value },
    Remove,
}
#[derive(Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EditorChoice {
    #[default]
    Refuse,
    KeepMine,
    KeepTheirs,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EditorSave {
    pub target_host_id: String,
    pub context: String,
    pub edits: Vec<EditorEdit>,
    #[serde(default)]
    pub choice: EditorChoice,
    pub resolution: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EditorView {
    pub context: String,
    pub settings: Value,
    pub current: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EditorResult {
    pub saved: bool,
    pub conflicts: Vec<Vec<String>>,
    pub resolution: Option<String>,
}
macro_rules! concealed { ($($t:ty),*) => { $(impl std::fmt::Debug for $t {
 fn fmt(&self,f:&mut std::fmt::Formatter<'_>)->std::fmt::Result { f.write_str("EditorInput([redacted])") }
})* }; }
concealed!(EditorOpen, EditorEdit, EditorValue, EditorChoice, EditorSave);
