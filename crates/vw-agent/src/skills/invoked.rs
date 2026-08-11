//! 已调用技能的可持久化会话状态与按需重放。

use serde::{Deserialize, Serialize};
use serde_yaml::Value;
use std::path::{Path, PathBuf};

/// 已调用技能的稳定记录；不保存技能正文或其它展开后的上下文。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct InvokedSkillRecord {
    pub(crate) skill_name: String,
    pub(crate) source: PathBuf,
    pub(crate) args: Option<String>,
}

impl InvokedSkillRecord {
    /// 从已通过调用层验证的技能文件创建可跨会话保存的记录。
    pub(crate) fn new(
        skill_name: impl Into<String>,
        source: &Path,
        args: Option<String>,
    ) -> Result<Self, String> {
        let source = source
            .canonicalize()
            .map_err(|error| format!("已调用技能来源不存在：{}（{error}）", source.display()))?;
        Ok(Self { skill_name: skill_name.into(), source, args })
    }

    pub(crate) fn stable_id(&self) -> String {
        self.source.to_string_lossy().to_string()
    }

    /// 重新读取当前文件，而非复用调用时的旧正文。
    pub(crate) fn replay(&self) -> Result<InvokedSkillReplay, String> {
        let source = &self.source;
        if !source.is_file() {
            return Err(format!("已调用技能来源不存在：{}", source.display()));
        }
        let Some(directory) = source.parent() else {
            return Err(format!("已调用技能来源无效：{}", source.display()));
        };
        if super::is_local_skill_disabled(directory) {
            return Err(format!("已调用技能已被禁用：{}", self.skill_name));
        }

        let source_text = std::fs::read_to_string(source)
            .map_err(|error| format!("无法重新读取已调用技能 {}：{error}", source.display()))?;
        let metadata = parse_metadata(&source_text)?;
        if metadata.disable_model_invocation {
            return Err(format!("已调用技能已禁止模型调用：{}", self.skill_name));
        }

        let content = source_text.replace("$ARGUMENTS", self.args.as_deref().unwrap_or_default());
        let base_dir = format!("file://{}", directory.to_string_lossy());
        Ok(InvokedSkillReplay {
            stable_id: self.stable_id(),
            content: format!(
                "<system-reminder>\nSkill \"{}\" is now active. Its base directory is {}. Relative paths in the skill resolve from that directory.\n\n{}\n</system-reminder>",
                self.skill_name, base_dir, content
            ),
            allowed_tools: metadata.allowed_tools,
            model: metadata.model,
            effort: metadata.effort,
        })
    }
}

/// 从已调用技能恢复的一次临时模型上下文修改。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct InvokedSkillReplay {
    pub(crate) stable_id: String,
    pub(crate) content: String,
    pub(crate) allowed_tools: Vec<String>,
    pub(crate) model: Option<String>,
    pub(crate) effort: Option<String>,
}

#[derive(Debug, Default)]
struct SkillMetadata {
    allowed_tools: Vec<String>,
    model: Option<String>,
    effort: Option<String>,
    disable_model_invocation: bool,
}

fn parse_metadata(source: &str) -> Result<SkillMetadata, String> {
    let Some(frontmatter) = source
        .strip_prefix("---\n")
        .and_then(|source| source.split_once("\n---"))
        .map(|(frontmatter, _)| frontmatter)
    else {
        return Ok(SkillMetadata::default());
    };
    let value: Value = serde_yaml::from_str(frontmatter)
        .map_err(|error| format!("已调用技能 frontmatter 无效：{error}"))?;
    let Some(fields) = value.as_mapping() else {
        return Err("已调用技能 frontmatter 必须为对象".to_string());
    };

    let field = |name: &str| fields.get(Value::String(name.to_string()));
    Ok(SkillMetadata {
        allowed_tools: field("allowed-tools").map(string_list).unwrap_or_default(),
        model: field("model").and_then(value_string),
        effort: field("effort").and_then(value_string),
        disable_model_invocation: field("disable-model-invocation")
            .and_then(Value::as_bool)
            .unwrap_or(false),
    })
}

fn value_string(value: &Value) -> Option<String> {
    value.as_str().map(str::trim).filter(|value| !value.is_empty()).map(ToOwned::to_owned)
}

fn string_list(value: &Value) -> Vec<String> {
    match value {
        Value::String(value) => value
            .split([',', ' '])
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToOwned::to_owned)
            .collect(),
        Value::Sequence(values) => values.iter().filter_map(value_string).collect(),
        _ => Vec::new(),
    }
}
