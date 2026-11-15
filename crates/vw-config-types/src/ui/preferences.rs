use chrono_tz::{TZ_VARIANTS, Tz};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub const DEFAULT_TIMEZONE: &str = "Asia/Shanghai";

/// Desktop interface language, independent of the editor language.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, Default, PartialEq, Eq)]
pub enum DesktopLocale {
    #[default]
    #[serde(rename = "zh-CN")]
    SimplifiedChinese,
    #[serde(rename = "zh-TW")]
    TraditionalChinese,
    #[serde(rename = "en")]
    English,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
#[derive(Default)]
pub enum PreviewAutoSaveMode {
    Off,
    AfterDelay,
    #[default]
    OnFocusChange,
    OnWindowChange,
}

impl PreviewAutoSaveMode {
    pub const ALL: [Self; 4] =
        [Self::Off, Self::AfterDelay, Self::OnFocusChange, Self::OnWindowChange];

    pub fn label(self) -> &'static str {
        match self {
            Self::Off => "关闭",
            Self::AfterDelay => "延迟保存",
            Self::OnFocusChange => "编辑器失焦时保存",
            Self::OnWindowChange => "窗口失焦时保存",
        }
    }

    pub fn description(self) -> &'static str {
        match self {
            Self::Off => "不自动保存，需手动保存当前文件。",
            Self::AfterDelay => "编辑内容变更后，等待短暂延迟再自动保存。",
            Self::OnFocusChange => "当编辑器失去焦点时，自动保存当前文件。",
            Self::OnWindowChange => "当应用窗口失去焦点时，自动保存当前文件。",
        }
    }
}

impl std::fmt::Display for PreviewAutoSaveMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.label())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Default, PartialEq)]
pub struct ModelRoute {
    #[serde(default)]
    pub pattern: String,
    #[serde(default)]
    pub provider: String,
    #[serde(default)]
    pub model: String,
    #[serde(default)]
    pub priority: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Default, PartialEq, Eq)]
#[serde(default)]
pub struct TuiKeybindingOverrideConfig {
    pub context: String,
    pub action: String,
    pub keys: String,
}

pub fn default_timezone() -> String {
    DEFAULT_TIMEZONE.to_string()
}

pub fn timezone_options() -> Vec<&'static str> {
    TZ_VARIANTS.iter().map(|timezone| timezone.name()).collect()
}

pub(super) fn deserialize_timezone<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = String::deserialize(deserializer)?;
    value
        .parse::<Tz>()
        .map_err(|_| serde::de::Error::custom(format!("invalid IANA timezone: {value}")))?;
    Ok(value)
}

#[cfg(test)]
#[path = "preferences_tests.rs"]
mod tests;
