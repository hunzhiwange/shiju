//! Native agent profile rendering data.
pub fn entries() -> Vec<crate::state::DelegateAgentSettingsEntry> {
    [
                ("acp-codex", serde_json::json!({"label":"内置 Codex 验收", "description":"中文长描述，验证配置编辑与暗黑主题。\n第二行描述。\n".repeat(8), "runtime":{"kind":"acp","backend":"codex"}, "recommended_prompts":["检查代码", "解释变更"], "system_prompt":"规则：先阅读代码，明确说明验证结果。"})),
                ("profile-fixture", serde_json::json!({"label":"自定义验收", "runtime":{"kind":"native"}})),
            ].into_iter().map(|(key, value)| crate::state::DelegateAgentSettingsEntry::from_config(key, Some({ let mut full = serde_json::to_value(config_agent::DelegateAgentConfig::default()).unwrap(); full.as_object_mut().unwrap().extend(value.as_object().unwrap().clone()); serde_json::from_value(full).unwrap() }))).collect()
}
