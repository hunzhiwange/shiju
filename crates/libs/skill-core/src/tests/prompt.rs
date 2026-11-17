//! 技能提示生成模块的单元测试
//!
//! 本模块测试 `skills_to_prompt` 系列函数的正确性，包括：
//! - 空技能列表的处理
//! - 技能信息到 XML 格式的转换
//! - 紧凑模式下的输出格式
//! - 工具信息的包含
//! - XML 特殊字符的转义处理

use super::*;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// 测试空技能列表生成空提示
///
/// 验证当传入空切片时，`skills_to_prompt` 返回空字符串。
/// 这是一个边界条件测试，确保函数能正确处理无技能的场景。
#[test]
fn skills_to_prompt_empty() {
    let prompt = skills_to_prompt(&[], Path::new("/tmp"));
    assert!(prompt.is_empty());
}

/// 测试包含技能时的提示生成
///
/// 验证 `skills_to_prompt` 能正确将技能信息转换为 XML 格式：
/// - 输出包含 `<available_skills>` 根标签
/// - 技能名称正确嵌入 `<name>` 标签
/// - 提示指令正确嵌入 `<instruction>` 标签
#[test]
fn skills_to_prompt_with_skills() {
    // 构造测试用的技能对象
    let skills = vec![Skill {
        name: "test".to_string(),
        description: "A test".to_string(),
        version: "1.0.0".to_string(),
        author: None,
        tags: vec![],
        tools: vec![],
        prompts: vec!["Do the thing.".to_string()],
        location: None,
    }];
    let prompt = skills_to_prompt(&skills, Path::new("/tmp"));
    // 验证 XML 结构正确性
    assert!(prompt.contains("<available_skills>"));
    assert!(prompt.contains("<name>test</name>"));
    assert!(prompt.contains("<instruction>Do the thing.</instruction>"));
}

/// 测试紧凑模式下省略指令和工具信息
///
/// 当使用 `SkillsPromptInjectionMode::Compact` 模式时：
/// - 技能位置以相对路径显示
/// - 要求在执行技能任务前从 location 读取完整指令
/// - 不包含详细指令(`<instructions>`)和工具(`<tools>`)信息
///
/// 这种模式适用于减少提示词体积的场景。
#[test]
fn skills_to_prompt_compact_mode_omits_instructions_and_tools() {
    // 构造包含工具和提示的完整技能对象
    let skills = vec![Skill {
        name: "test".to_string(),
        description: "A test".to_string(),
        version: "1.0.0".to_string(),
        author: None,
        tags: vec![],
        tools: vec![SkillTool {
            name: "run".to_string(),
            description: "Run task".to_string(),
            kind: "shell".to_string(),
            command: "echo hi".to_string(),
            args: HashMap::new(),
        }],
        prompts: vec!["Do the thing.".to_string()],
        location: Some(PathBuf::from("/tmp/workspace/skills/test/SKILL.md")),
    }];
    // 使用紧凑模式生成提示
    let prompt = skills_to_prompt_with_mode(
        &skills,
        Path::new("/tmp/workspace"),
        SkillsPromptInjectionMode::Compact,
    );

    // 验证紧凑模式的输出特征
    assert!(prompt.contains("<available_skills>"));
    assert!(prompt.contains("<name>test</name>"));
    assert!(prompt.contains("<location>skills/test/SKILL.md</location>"));
    assert!(prompt.contains(
        "Before taking task actions for a skill that applies or is invoked, read its complete instruction file from `location`."
    ));
    // 验证省略的内容
    assert!(!prompt.contains("<instructions>"));
    assert!(!prompt.contains("<instruction>Do the thing.</instruction>"));
    assert!(!prompt.contains("<tools>"));
}

/// 测试提示中包含工具信息
///
/// 验证技能的工具定义被正确包含在生成的提示中：
/// - 工具名称在 `<name>` 标签中
/// - 工具描述在 `<description>` 标签中
/// - 工具类型在 `<kind>` 标签中
#[test]
fn skills_to_prompt_includes_tools() {
    // 构造包含工具的技能
    let skills = vec![Skill {
        name: "weather".to_string(),
        description: "Get weather".to_string(),
        version: "1.0.0".to_string(),
        author: None,
        tags: vec![],
        tools: vec![SkillTool {
            name: "get_weather".to_string(),
            description: "Fetch forecast".to_string(),
            kind: "shell".to_string(),
            command: "curl wttr.in".to_string(),
            args: HashMap::new(),
        }],
        prompts: vec![],
        location: None,
    }];
    let prompt = skills_to_prompt(&skills, Path::new("/tmp"));
    // 验证工具信息完整性
    assert!(prompt.contains("weather"));
    assert!(prompt.contains("<name>get_weather</name>"));
    assert!(prompt.contains("<description>Fetch forecast</description>"));
    assert!(prompt.contains("<kind>shell</kind>"));
}

/// 测试 XML 特殊字符的正确转义
///
/// 验证技能元数据和指令中的 XML 特殊字符被正确转义：
/// - `<` 转义为 `&lt;`
/// - `>` 转义为 `&gt;`
/// - `&` 转义为 `&amp;`
/// - `"` 转义为 `&quot;`
///
/// 这确保生成的 XML 是格式良好的，不会因特殊字符而破坏结构。
#[test]
fn skills_to_prompt_escapes_xml_content() {
    // 构造包含各种 XML 特殊字符的测试数据
    let skills = vec![Skill {
        name: "xml<skill>".to_string(),
        description: "A & B".to_string(),
        version: "1.0.0".to_string(),
        author: None,
        tags: vec![],
        tools: vec![],
        prompts: vec!["Use <tool> & check \"quotes\".".to_string()],
        location: None,
    }];

    let prompt = skills_to_prompt(&skills, Path::new("/tmp"));
    // 验证所有特殊字符被正确转义
    assert!(prompt.contains("<name>xml&lt;skill&gt;</name>"));
    assert!(prompt.contains("<description>A &amp; B</description>"));
    assert!(
        prompt.contains(
            "<instruction>Use &lt;tool&gt; &amp; check &quot;quotes&quot;.</instruction>"
        )
    );
}
