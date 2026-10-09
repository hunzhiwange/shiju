//! 配置文件读写过程中的 payload 归一化工具。
//!
//! 该模块只处理 JSON/TOML 容器形状和历史别名兼容，不负责业务校验。所有迁移都保持显式日志，方便用户
//! 发现旧配置键仍在生效或被忽略。

use anyhow::{Context, Result, bail};
use std::path::Path;
#[cfg(not(target_arch = "wasm32"))]
use tokio::fs;
use vw_config_types::config::Config;

use crate::app::agent::config::schema::CONFIG_AGENT_KEY;

/// 归一化 TOML 顶层历史表名。
///
/// 参数 `raw_toml` 会被原地修改：`[Gateway]` 会映射为 `[gateway]`，`[acp2]` 会映射为 `[acp]`。
/// 如果新旧键同时存在，旧键会被删除并记录警告，避免旧配置覆盖用户的新配置。
pub(crate) fn normalize_top_level_table_aliases(raw_toml: &mut toml::Value) {
    let Some(root) = raw_toml.as_table_mut() else {
        return;
    };

    if root.contains_key("Gateway") {
        if root.contains_key("gateway") {
            let _ = root.remove("Gateway");
            tracing::warn!("Legacy table [Gateway] ignored because [gateway] is already present.");
        } else {
            let value = root.remove("Gateway").expect("Gateway exists after contains_key");
            root.insert("gateway".to_string(), value);
            tracing::warn!("Legacy table [Gateway] mapped to [gateway].");
        }
    }

    if root.contains_key("acp2") {
        if root.contains_key("acp") {
            let _ = root.remove("acp2");
            tracing::warn!("Legacy table [acp2] ignored because [acp] is already present.");
        } else {
            let value = root.remove("acp2").expect("acp2 exists after contains_key");
            root.insert("acp".to_string(), value);
            tracing::warn!("Legacy table [acp2] mapped to [acp].");
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
/// 从 JSON 配置文件读取根对象。
///
/// 文件不存在或内容为空时返回空对象，便于初始化流程继续执行。读取失败或 JSON 解析失败会返回带上下文的错误。
pub(crate) async fn read_config_root_json(path: &Path) -> Result<serde_json::Value> {
    if !path.exists() {
        return Ok(serde_json::json!({}));
    }
    let contents = fs::read_to_string(path).await.context("Failed to read config file")?;
    if contents.trim().is_empty() {
        return Ok(serde_json::json!({}));
    }
    serde_json::from_str(&contents).context("Failed to parse config file")
}

/// 将项目覆盖递归合并到数据库基础 payload。
///
/// 对象逐层合并，`null` 删除基础字段，其他值直接替换。调用方负责保证两个根值均为对象。
pub(crate) fn merge_config_payload(target: &mut serde_json::Value, patch: serde_json::Value) {
    match (target, patch) {
        (serde_json::Value::Object(target_object), serde_json::Value::Object(patch_object)) => {
            for (key, patch_value) in patch_object {
                if patch_value.is_null() {
                    target_object.remove(&key);
                } else if let Some(current) = target_object.get_mut(&key) {
                    merge_config_payload(current, patch_value);
                } else {
                    target_object.insert(key, patch_value);
                }
            }
        }
        (target_value, patch_value) => *target_value = patch_value,
    }
}

#[cfg(target_arch = "wasm32")]
/// WASM 环境下的 JSON 配置读取占位实现。
///
/// 浏览器目标不访问本地文件系统，因此始终返回空对象。
pub(crate) async fn read_config_root_json(_path: &Path) -> Result<serde_json::Value> {
    Ok(serde_json::json!({}))
}

/// 从根 JSON 中提取实际配置 payload。
///
/// 空对象表示还没有持久化配置，返回 `None`；非空对象会克隆为独立 payload 供反序列化使用。
pub(crate) fn extract_config_payload(root: &serde_json::Value) -> Option<serde_json::Value> {
    let obj = root.as_object()?;
    if obj.is_empty() { None } else { Some(root.clone()) }
}

/// 将配置 payload 写回根 JSON 对象。
///
/// 该函数会移除旧的嵌套 agent key 和旧模型别名，再把新的配置字段合并到根对象。这样可以避免保存时
/// 同时保留新旧字段而造成下一次加载的优先级歧义。
pub(crate) fn upsert_config_payload(
    root: &mut serde_json::Value,
    config_payload: serde_json::Value,
) {
    if !root.is_object() {
        *root = serde_json::json!({});
    }
    let root_obj = root.as_object_mut().expect("root initialized as JSON object");

    root_obj.remove(CONFIG_AGENT_KEY);
    root_obj.remove("model");
    root_obj.remove("model_provider");

    if let Some(config_obj) = config_payload.as_object() {
        for (key, value) in config_obj {
            if key == CONFIG_AGENT_KEY {
                continue;
            }
            root_obj.insert(key.clone(), value.clone());
        }
    }
}

/// 归一化 JSON 配置中的历史别名冲突。
///
/// 当新旧字段同时出现时保留新字段，缺少新字段时把旧字段迁移到新字段。该函数只处理 payload 形状，
/// 不验证字段值是否合法。
pub(crate) fn normalize_legacy_alias_conflicts(config_payload: &mut serde_json::Value) {
    let Some(obj) = config_payload.as_object_mut() else {
        return;
    };

    if obj.contains_key("default_model") && obj.contains_key("model") {
        obj.remove("model");
    }

    if obj.contains_key("default_provider") && obj.contains_key("model_provider") {
        obj.remove("model_provider");
    }

    if obj.contains_key("acp") && obj.contains_key("acp2") {
        obj.remove("acp2");
    } else if !obj.contains_key("acp") {
        if let Some(value) = obj.remove("acp2") {
            obj.insert("acp".to_string(), value);
        }
    }
}

/// 解析用户配置原文并归一化为当前配置结构，不应用环境变量覆盖。
///
/// 当前 JSON 与旧 TOML 都受支持。JSON 根必须是对象；未知键会记录键路径，但不会记录配置值。
/// 该函数只负责格式解析和历史别名归一化，密钥处理与业务校验由调用方显式执行。
pub(crate) fn parse_config_contents(contents: &str) -> Result<Config> {
    if contents.trim().is_empty() {
        bail!("config source is empty");
    }

    match serde_json::from_str::<serde_json::Value>(contents) {
        Ok(mut payload) => {
            if !payload.is_object() {
                bail!("config JSON root must be an object");
            }
            normalize_legacy_alias_conflicts(&mut payload);
            let normalized =
                serde_json::to_string(&payload).context("Failed to normalize JSON config")?;
            let mut deserializer = serde_json::Deserializer::from_str(&normalized);
            deserialize_config(&mut deserializer, "JSON")
        }
        Err(json_error) => {
            let mut raw_toml: toml::Value = toml::from_str(contents).with_context(|| {
                format!("Failed to parse config as JSON or TOML; JSON error: {json_error}")
            })?;
            normalize_top_level_table_aliases(&mut raw_toml);
            let normalized =
                toml::to_string(&raw_toml).context("Failed to normalize TOML config")?;
            let deserializer = toml::Deserializer::parse(&normalized)
                .context("Failed to prepare normalized TOML config")?;
            deserialize_config(deserializer, "TOML")
        }
    }
}

fn deserialize_config<'de, D>(deserializer: D, format: &str) -> Result<Config>
where
    D: serde::Deserializer<'de>,
{
    let mut ignored_paths = Vec::new();
    let config = serde_ignored::deserialize(deserializer, |path| {
        ignored_paths.push(path.to_string());
    })
    .map_err(|error| anyhow::anyhow!("Failed to deserialize {format} config: {error}"))?;

    for path in ignored_paths {
        tracing::warn!(key = %path, format, "Unknown config key ignored during migration");
    }
    Ok(config)
}
