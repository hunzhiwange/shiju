//! # 技能提示生成模块
//!
//! 本模块负责将技能（Skill）数据结构转换为可在 LLM 提示中使用的 XML 格式字符串。
//!
//! ## 主要功能
//!
//! - **XML 格式化**：将技能元数据格式化为结构化的 XML 文本
//! - **XML 转义**：自动处理特殊字符（如 `&`、`<`、`>` 等）的转义
//! - **双模式输出**：支持完整模式（Full）和精简模式（Compact）两种输出格式
//!
//! ## 输出格式
//!
//! 生成的 XML 结构遵循以下格式：
//!
//! ```xml
//! <available_skills>
//!   <skill>
//!     <name>技能名称</name>
//!     <description>技能描述</description>
//!     <location>技能文件路径</location>
//!     <!-- Full 模式下额外包含 -->
//!     <instructions>
//!       <instruction>指令内容</instruction>
//!     </instructions>
//!     <tools>
//!       <tool>
//!         <name>工具名称</name>
//!         <description>工具描述</description>
//!         <kind>工具类型</kind>
//!       </tool>
//!     </tools>
//!   </skill>
//! </available_skills>
//! ```

use crate::app::agent::skills::listing::SkillListingEntry;
use crate::app::agent::skills::types::Skill;
use std::path::{Path, PathBuf};

/// 将文本追加到输出字符串，同时进行 XML 特殊字符转义
///
/// 此函数遍历输入文本的每个字符，将 XML 特殊字符替换为对应的实体引用，
/// 以确保生成的 XML 文档格式正确且安全。
///
/// # 参数
///
/// - `out`：输出字符串的可变引用，转义后的文本将追加到此字符串
/// - `text`：需要进行转义的原始文本
///
/// # 转义规则
///
/// | 原字符 | 转义后      |
/// |--------|-------------|
/// | `&`    | `&amp;`     |
/// | `<`    | `&lt;`      |
/// | `>`    | `&gt;`      |
/// | `"`    | `&quot;`    |
/// | `'`    | `&apos;`    |
fn append_xml_escaped(out: &mut String, text: &str) {
    for ch in text.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            _ => out.push(ch),
        }
    }
}

/// 写入一个带有缩进的 XML 文本元素
///
/// 此函数生成格式化的 XML 元素，包含指定的缩进、标签名和文本内容。
/// 文本内容会自动进行 XML 特殊字符转义。
///
/// # 参数
///
/// - `out`：输出字符串的可变引用
/// - `indent`：缩进空格数，用于格式化输出
/// - `tag`：XML 标签名称
/// - `value`：元素的文本内容（将被自动转义）
///
/// # 输出格式
///
/// ```xml
/// [indent个空格]<tag>转义后的value</tag>\n
/// ```
///
/// # 示例
///
/// ```ignore
/// let mut output = String::new();
/// write_xml_text_element(&mut output, 4, "name", "Test & Demo");
/// // 输出: "    <name>Test &amp; Demo</name>\n"
/// ```
fn write_xml_text_element(out: &mut String, indent: usize, tag: &str, value: &str) {
    // 添加缩进空格
    for _ in 0..indent {
        out.push(' ');
    }
    // 写入开始标签
    out.push('<');
    out.push_str(tag);
    out.push('>');
    // 写入转义后的文本内容
    append_xml_escaped(out, value);
    // 写入结束标签
    out.push_str("</");
    out.push_str(tag);
    out.push_str(">\n");
}

/// 渲染可信 developer 目录；完整正文仍由 skill 工具按需提供为 user 上下文。
pub(crate) fn format_skill_listing_reminder(entries: &[SkillListingEntry]) -> String {
    let mut reminder = String::from(
        "<skills_instructions>\n\
         Use the skill tool to load a skill's complete instructions before executing it.\n\
         If the user explicitly names a skill, you must use it. Select skills when their description matches the task.\n\
         Follow the loaded workflow; resolve relative resources from the skill file's directory.\n\
         The catalog contains metadata only, not preloaded instructions.\n\
         <available_skills>\n",
    );
    for entry in entries {
        reminder.push_str("  <skill>\n");
        write_xml_text_element(&mut reminder, 4, "name", &entry.name);
        write_xml_text_element(&mut reminder, 4, "description", &entry.description);
        write_xml_text_element(&mut reminder, 4, "location", &entry.location);
        reminder.push_str("  </skill>\n");
    }
    reminder.push_str("</available_skills>\n</skills_instructions>");
    reminder
}

/// 解析技能的完整文件路径
///
/// 如果技能已指定 `location` 字段，则直接使用该路径；
/// 否则，根据约定在 `workspace_dir/skills/{skill_name}/SKILL.md` 位置构建默认路径。
///
/// # 参数
///
/// - `skill`：技能对象引用
/// - `workspace_dir`：工作空间根目录路径
///
/// # 返回值
///
/// 返回技能文件的完整路径
///
/// # 示例
///
/// ```ignore
/// let skill = Skill { name: "my_skill".to_string(), location: None, ... };
/// let path = resolve_skill_location(&skill, &PathBuf::from("/workspace"));
/// // 返回: "/workspace/skills/my_skill/SKILL.md"
/// ```
fn resolve_skill_location(skill: &Skill, workspace_dir: &Path) -> PathBuf {
    skill
        .location
        .clone()
        // 若未指定 location，则使用默认路径：{workspace}/skills/{skill_name}/SKILL.md
        .unwrap_or_else(|| workspace_dir.join("skills").join(&skill.name).join("SKILL.md"))
}

/// 渲染技能位置路径的字符串表示
///
/// 根据配置偏好，将技能文件路径转换为字符串。在精简模式下优先使用相对路径，
/// 以减少上下文长度并提高可读性。
///
/// # 参数
///
/// - `skill`：技能对象引用
/// - `workspace_dir`：工作空间根目录路径
/// - `prefer_relative`：是否优先使用相对路径
///
/// # 返回值
///
/// 返回路径的字符串表示。如果 `prefer_relative` 为 true 且路径位于工作空间内，
/// 则返回相对于工作空间的相对路径；否则返回绝对路径。
///
/// # 示例
///
/// ```ignore
/// // 假设 workspace_dir = "/workspace", location = "/workspace/skills/test/SKILL.md"
/// render_skill_location(&skill, &workspace_dir, true);  // 返回: "skills/test/SKILL.md"
/// render_skill_location(&skill, &workspace_dir, false); // 返回: "/workspace/skills/test/SKILL.md"
/// ```
fn render_skill_location(skill: &Skill, workspace_dir: &Path, prefer_relative: bool) -> String {
    let location = resolve_skill_location(skill, workspace_dir);
    if prefer_relative {
        // 尝试将路径转换为相对于工作空间的相对路径
        if let Ok(relative) = location.strip_prefix(workspace_dir) {
            return relative.display().to_string();
        }
    }
    // 无法生成相对路径或不要求相对路径时，返回绝对路径
    location.display().to_string()
}

/// 将技能列表转换为提示字符串（完整模式）
///
/// 这是 [`skills_to_prompt_with_mode`] 的便捷封装，默认使用完整模式输出。
///
/// # 参数
///
/// - `skills`：技能对象切片
/// - `workspace_dir`：工作空间根目录路径
///
/// # 返回值
///
/// 返回格式化的 XML 字符串，包含所有技能的完整信息。
/// 如果技能列表为空，则返回空字符串。
///
/// # 示例
///
/// ```ignore
/// let skills = vec![Skill { name: "coding".to_string(), ... }];
/// let prompt = skills_to_prompt(&skills, &PathBuf::from("/workspace"));
/// ```
pub fn skills_to_prompt(skills: &[Skill], workspace_dir: &Path) -> String {
    skills_to_prompt_with_mode(
        skills,
        workspace_dir,
        crate::app::agent::config::SkillsPromptInjectionMode::Full,
    )
}

/// 将技能列表转换为提示字符串（支持模式选择）
///
/// 此函数是技能提示生成的核心实现，根据指定模式生成不同详细程度的 XML 输出。
///
/// # 参数
///
/// - `skills`：技能对象切片引用
/// - `workspace_dir`：工作空间根目录路径，用于解析技能文件位置
/// - `mode`：提示注入模式
///   - `Full`：完整模式，包含技能的所有详细信息（指令和工具）
///   - `Compact`：精简模式，仅包含摘要信息，技能指令按需加载
///
/// # 返回值
///
/// 返回格式化的 XML 字符串。如果技能列表为空，则返回空字符串。
///
/// # 输出模式说明
///
/// ## Full 模式（完整模式）
///
/// 适用于需要立即使用技能指令的场景。输出包含：
/// - 技能名称、描述、位置
/// - 完整的指令列表
/// - 工具元数据（名称、描述、类型）
///
/// ## Compact 模式（精简模式）
///
/// 适用于上下文空间受限或技能按需加载的场景。输出包含：
/// - 技能名称、描述、位置（使用相对路径）
/// - 不包含指令和工具信息（需通过读取技能文件获取）
///
/// # 示例
///
/// ```ignore
/// use crate::app::agent::config::SkillsPromptInjectionMode;
///
/// let skills = vec![
///     Skill {
///         name: "code_review".to_string(),
///         description: "代码审查助手".to_string(),
///         prompts: vec!["检查代码风格".to_string()],
///         tools: vec![ToolMeta { ... }],
///         location: None,
///     },
/// ];
///
/// // 完整模式输出
/// let full_prompt = skills_to_prompt_with_mode(
///     &skills,
///     &PathBuf::from("/workspace"),
///     SkillsPromptInjectionMode::Full,
/// );
///
/// // 精简模式输出
/// let compact_prompt = skills_to_prompt_with_mode(
///     &skills,
///     &PathBuf::from("/workspace"),
///     SkillsPromptInjectionMode::Compact,
/// );
/// ```
pub fn skills_to_prompt_with_mode(
    skills: &[Skill],
    workspace_dir: &Path,
    mode: crate::app::agent::config::SkillsPromptInjectionMode,
) -> String {
    use std::fmt::Write;

    // 空技能列表直接返回空字符串
    if skills.is_empty() {
        return String::new();
    }

    // 根据模式初始化提示字符串的头部说明
    let mut prompt = match mode {
        // 完整模式：技能指令和工具元数据已预加载
        crate::app::agent::config::SkillsPromptInjectionMode::Full => String::from(
            "## Available Skills\n\n\
             Skill instructions and tool metadata are preloaded below.\n\
             Follow these instructions directly; do not read skill files at runtime unless the user asks.\n\
             Unless the user explicitly asks for a specific skill to be executed in a different way, that skill's instructions are mandatory. Follow its documented workflow exactly and completely, including every required sequence, tool, check, and output.\n\
             A user-requested variation applies only to the identified skill and the specifically requested parts; every other instruction remains mandatory. Do not infer an override from general preferences, goals, or suggested shortcuts.\n\
             Without such an explicit request, do not simplify, omit, substitute, short-circuit, or bypass any required step, even if another approach seems faster or equivalent.\n\
             If a required step cannot be executed, stop the skill workflow and report the exact blocker; do not silently improvise around it.\n\n\
             <available_skills>\n",
        ),
        // 精简模式：仅预加载摘要，指令按需从文件加载
        crate::app::agent::config::SkillsPromptInjectionMode::Compact => String::from(
            "## Available Skills\n\n\
             Skill summaries are preloaded below to keep context compact.\n\
             Before taking task actions for a skill that applies or is invoked, read its complete instruction file from `location`.\n\
             Unless the user explicitly asks for a specific skill to be executed in a different way, that skill's loaded instructions are mandatory. Follow the documented workflow exactly and completely, including every required sequence, tool, check, and output.\n\
             A user-requested variation applies only to the identified skill and the specifically requested parts; every other instruction remains mandatory. Do not infer an override from general preferences, goals, or suggested shortcuts.\n\
             Without such an explicit request, do not simplify, omit, substitute, short-circuit, or bypass any required step, even if another approach seems faster or equivalent.\n\
             If a required step cannot be executed, stop the skill workflow and report the exact blocker; do not silently improvise around it.\n\n\
             <available_skills>\n",
        ),
    };

    // 遍历所有技能，逐个格式化为 XML 元素
    for skill in skills {
        let _ = writeln!(prompt, "  <skill>");

        // 写入技能基本信息
        write_xml_text_element(&mut prompt, 4, "name", &skill.invocation_name());
        write_xml_text_element(&mut prompt, 4, "description", &skill.description);

        // 解析并写入技能位置
        // 精简模式下优先使用相对路径以减少上下文长度
        let location = render_skill_location(
            skill,
            workspace_dir,
            matches!(mode, crate::app::agent::config::SkillsPromptInjectionMode::Compact),
        );
        write_xml_text_element(&mut prompt, 4, "location", &location);

        // 仅在完整模式下输出详细的指令和工具信息
        if matches!(mode, crate::app::agent::config::SkillsPromptInjectionMode::Full) {
            // 输出指令列表（如果存在）
            if !skill.prompts.is_empty() {
                let _ = writeln!(prompt, "    <instructions>");
                for instruction in &skill.prompts {
                    write_xml_text_element(&mut prompt, 6, "instruction", instruction);
                }
                let _ = writeln!(prompt, "    </instructions>");
            }

            // 输出工具列表（如果存在）
            if !skill.tools.is_empty() {
                let _ = writeln!(prompt, "    <tools>");
                for tool in &skill.tools {
                    let _ = writeln!(prompt, "      <tool>");
                    write_xml_text_element(&mut prompt, 8, "name", &tool.name);
                    write_xml_text_element(&mut prompt, 8, "description", &tool.description);
                    write_xml_text_element(&mut prompt, 8, "kind", &tool.kind);
                    let _ = writeln!(prompt, "      </tool>");
                }
                let _ = writeln!(prompt, "    </tools>");
            }
        }

        let _ = writeln!(prompt, "  </skill>");
    }

    // 闭合最外层标签
    prompt.push_str("</available_skills>");
    prompt
}
#[cfg(test)]
#[path = "prompt_tests.rs"]
mod prompt_tests;
