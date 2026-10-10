//! 技能加载配置模块。
//!
//! 本模块定义技能提示注入方式及开放技能目录的开关，供运行时决定如何发现和拼装
//! 技能内容。

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// 技能目录提供方。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, Default)]
#[serde(rename_all = "snake_case")]
pub enum SkillsDirectoryProvider {
    /// 使用 VibeWindow 原生技能目录。
    #[default]
    Vibewindow,
    /// 使用 Codex/Codex 风格技能目录。
    Codex,
    /// 使用 Claude Code 技能目录。
    Claude,
    /// 使用 Cursor 技能目录。
    Cursor,
}

impl SkillsDirectoryProvider {
    pub const ALL: [Self; 4] = [Self::Vibewindow, Self::Codex, Self::Claude, Self::Cursor];

    pub fn label(self) -> &'static str {
        match self {
            Self::Vibewindow => "VibeWindow",
            Self::Codex => "Codex",
            Self::Claude => "Claude",
            Self::Cursor => "Cursor",
        }
    }
}

impl std::fmt::Display for SkillsDirectoryProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.label())
    }
}

/// 技能提示注入模式。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, Default)]
#[serde(rename_all = "snake_case")]
pub enum SkillsPromptInjectionMode {
    /// 以紧凑形式注入技能提示。
    #[default]
    Compact,
    /// 以完整形式注入技能提示。
    Full,
}

/// 解析技能提示注入模式字符串。
pub fn parse_skills_prompt_injection_mode(raw: &str) -> Option<SkillsPromptInjectionMode> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "full" => Some(SkillsPromptInjectionMode::Full),
        "compact" => Some(SkillsPromptInjectionMode::Compact),
        _ => None,
    }
}

/// 解析技能目录提供方字符串。
pub fn parse_skills_directory_provider(raw: &str) -> Option<SkillsDirectoryProvider> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "vibewindow" | "vibe_window" | "vibe-window" => Some(SkillsDirectoryProvider::Vibewindow),
        "codex" => Some(SkillsDirectoryProvider::Codex),
        "claude" | "claude_code" | "claude-code" => Some(SkillsDirectoryProvider::Claude),
        "cursor" => Some(SkillsDirectoryProvider::Cursor),
        _ => None,
    }
}

/// 技能静态安全检测配置。
///
/// 所有检测默认关闭，并可按行为类别独立启用。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, Default)]
#[serde(default)]
pub struct SkillSecurityConfig {
    /// 是否要求技能目录包含清单文件。
    pub require_manifest: bool,

    /// 是否拒绝技能目录中的符号链接。
    pub reject_symlinks: bool,

    /// 是否拒绝技能目录中的脚本文件。
    pub reject_script_files: bool,

    /// 是否拒绝技能目录中的超大文本文件。
    pub reject_oversized_text_files: bool,

    /// 是否检测技能文件中的高风险内容。
    pub detect_high_risk_content: bool,

    /// 是否检测 Shell 命令串联。
    pub detect_shell_chaining: bool,

    /// 是否校验技能清单结构。
    pub validate_manifest: bool,

    /// 是否校验技能 Markdown 链接。
    pub validate_markdown_links: bool,
}

/// 技能系统配置。
///
/// 用于控制开放技能目录是否启用，以及提示注入使用紧凑模式还是完整模式。
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SkillsConfig {
    /// 技能目录提供方，缺省使用 VibeWindow 原生目录。
    #[serde(default)]
    pub directory_provider: SkillsDirectoryProvider,

    /// 是否让委托代理默认加载项目及父级技能。
    #[serde(default = "default_auto_load_project_skills")]
    pub auto_load_project_skills: bool,

    /// 技能提示注入模式。
    #[serde(default)]
    pub prompt_injection_mode: SkillsPromptInjectionMode,

    /// 技能静态安全检测策略。
    #[serde(default)]
    pub security: SkillSecurityConfig,
}

fn default_auto_load_project_skills() -> bool {
    true
}

impl Default for SkillsConfig {
    fn default() -> Self {
        Self {
            directory_provider: SkillsDirectoryProvider::default(),
            auto_load_project_skills: default_auto_load_project_skills(),
            prompt_injection_mode: SkillsPromptInjectionMode::default(),
            security: SkillSecurityConfig::default(),
        }
    }
}
#[cfg(test)]
#[path = "skills_tests.rs"]
mod skills_tests;
