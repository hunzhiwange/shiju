use crate::app::agent::config::IdentityConfig;
use agent_identity::{AieosIdentity, normalize_aieos_identity};
use anyhow::{Context, Result};
use serde_json::Value;
use std::path::{Path, PathBuf};

/// 从配置加载 AIEOS 身份（支持文件路径或内联 JSON）。
///
/// 优先检查 `aieos_path` 配置，如果未设置则使用 `aieos_inline`。
/// 如果两者都未配置，返回 `Ok(None)`。
pub fn load_aieos_identity(
    config: &IdentityConfig,
    workspace_dir: &Path,
) -> Result<Option<AieosIdentity>> {
    if config.format != "aieos" {
        return Ok(None);
    }

    if let Some(ref path) = config.aieos_path {
        let full_path = if Path::new(path).is_absolute() {
            PathBuf::from(path)
        } else {
            workspace_dir.join(path)
        };

        let content = std::fs::read_to_string(&full_path)
            .with_context(|| format!("读取 AIEOS 文件失败: {}", full_path.display()))?;
        let identity = parse_aieos_identity(&content)
            .with_context(|| format!("解析 AIEOS JSON 失败: {}", full_path.display()))?;

        return Ok(Some(identity));
    }

    if let Some(ref inline) = config.aieos_inline {
        let identity = parse_aieos_identity(inline).context("解析内联 AIEOS JSON 失败")?;
        return Ok(Some(identity));
    }

    anyhow::bail!(
        "身份格式设置为 'aieos'，但未配置 aieos_path 或 aieos_inline。\
         请在配置文件中设置其一：\n\
         \n\
         [identity]\n\
         format = \"aieos\"\n\
         aieos_path = \"identity.json\"\n\
         \n\
         或使用内联方式：\n\
         \n\
         [identity]\n\
         format = \"aieos\"\n\
         aieos_inline = '{{\"identity\": {{...}}}}'"
    )
}

/// 解析 AIEOS 身份 JSON 字符串。
///
/// 将 JSON 字符串解析为 `Value`，验证其为对象类型，
/// 然后进行规范化处理。
pub(super) fn parse_aieos_identity(content: &str) -> Result<AieosIdentity> {
    let payload: Value = serde_json::from_str(content).context("无效的 AIEOS JSON")?;

    if !payload.is_object() {
        anyhow::bail!("AIEOS 载荷必须是 JSON 对象")
    }

    Ok(normalize_aieos_identity(&payload))
}

/// 检查是否配置了 AIEOS 身份。
///
/// 当格式为 "aieos" 且设置了 `aieos_path` 或 `aieos_inline` 时返回 true。
pub fn is_aieos_configured(config: &IdentityConfig) -> bool {
    config.format == "aieos" && (config.aieos_path.is_some() || config.aieos_inline.is_some())
}

#[cfg(test)]
#[path = "load_tests.rs"]
mod load_tests;
