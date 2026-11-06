//! ACP 会话选项 meta 构造。

use super::*;

/// 将会话选项转换为 ACP `meta` 扩展字段。
///
/// 当前只在存在有效选项时返回 `Some`，避免向代理发送空扩展对象。空字符串工具
/// 名或模型名会被过滤。
pub(super) fn build_session_options_meta(options: Option<&AcpSessionOptions>) -> Option<acp::Meta> {
    let options = options?;
    let mut claude_code_options = Map::new();
    if let Some(model) = options.model.as_ref().filter(|value| !value.trim().is_empty()) {
        claude_code_options.insert("model".to_string(), Value::String(model.clone()));
    }
    if let Some(allowed_tools) = &options.allowed_tools {
        let allowed_tools = allowed_tools
            .iter()
            .filter_map(|value| {
                let trimmed = value.trim();
                (!trimmed.is_empty()).then(|| Value::String(trimmed.to_string()))
            })
            .collect::<Vec<_>>();
        if !allowed_tools.is_empty() {
            claude_code_options.insert("allowedTools".to_string(), Value::Array(allowed_tools));
        }
    }
    if let Some(max_turns) = options.max_turns {
        claude_code_options.insert("maxTurns".to_string(), json!(max_turns));
    }
    if claude_code_options.is_empty() {
        return None;
    }

    let mut meta = Map::new();
    meta.insert(
        "claudeCode".to_string(),
        json!({
            "options": Value::Object(claude_code_options),
        }),
    );
    Some(meta)
}

/// Only a request-owned Claude launch may pass a private SDK settings file.
/// cli_local has no launch reference and retains the original SDK options.
pub(super) fn managed_claude_settings(config: &AcpAgentConfig) -> Option<&str> {
    config.env.get("VW_ACP_LAUNCH_REFERENCE")?;
    config.env.get("VW_ACP_CLAUDE_SETTINGS").map(String::as_str)
}

pub(super) fn build_launch_session_meta(
    options: Option<&AcpSessionOptions>,
    config: &AcpAgentConfig,
) -> Option<acp::Meta> {
    let mut meta = build_session_options_meta(options);
    if let Some(path) = managed_claude_settings(config) {
        let meta = meta.get_or_insert_with(Map::new);
        let extension = meta.entry("claudeCode").or_insert_with(|| json!({"options": {}}));
        let options = extension["options"].as_object_mut().expect("typed Claude SDK options");
        options.insert("settings".into(), json!(path));
        // The selected client model also wins over persisted/native SDK model options.
        if let Some(model) = config.env.get("ANTHROPIC_MODEL") {
            options.insert("model".into(), json!(model));
        }
    }
    meta
}

impl AcpClient {
    pub(super) fn mounted_session_meta(&self) -> Option<acp::Meta> {
        let mut meta = build_launch_session_meta(self.session_options.as_ref(), &self.config);
        if self.agent_name == "kiro" {
            // Send the current grant on every open, including an empty preset list
            // on load; never inherit a historical client grant deliberately.
            meta.get_or_insert_with(Map::new).insert(
                "kiro".into(),
                crate::access_policy::kiro::session_policy(self.full_access, self.permission_mode),
            );
        }
        if let Some(selection) = &self.skill_mounts {
            if matches!(self.agent_name.as_str(), "claude" | "claude code") {
                let options = meta
                    .get_or_insert_with(Map::new)
                    .entry("claudeCode")
                    .or_insert_with(|| json!({"options": {}}))["options"]
                    .as_object_mut()
                    .expect("typed Claude options");
                let plugins: Vec<_> = selection
                    .skills
                    .first()
                    .into_iter()
                    .filter_map(|skill| {
                        Path::new(&skill.path)
                            .parent()?
                            .parent()
                            .map(|root| json!({"type":"local", "path":root}))
                    })
                    .collect();
                options.insert("plugins".into(), json!(plugins));
            }
            meta.get_or_insert_with(Map::new).insert(
                crate::SESSION_SKILL_MOUNTS_META_KEY.into(),
                serde_json::to_value(selection).expect("typed skill mount selection"),
            );
        }
        meta
    }
}
