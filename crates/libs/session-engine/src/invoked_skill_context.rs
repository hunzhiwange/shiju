//! Typed context updates produced when invoked skills are restored.
use serde_json::Value;

#[derive(Debug, Clone, PartialEq)]
pub struct InvokedSkillContextUpdate {
    pub key: String,
    pub value: Value,
}
