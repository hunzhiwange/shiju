//! 技能提示生成测试模块
//!
//! 本模块包含对 `skills_to_prompt` 及相关函数的单元测试，
//! 验证技能列表转换为提示文本的各种场景，包括：
//! - 空技能列表处理
//! - 完整技能信息生成
//! - 紧凑模式下的信息省略
//! - 工具信息包含
//! - XML 内容转义

use super::super::*;
use std::collections::HashMap;
use std::path::PathBuf;

/// 测试空技能列表生成的提示是否为空
///
/// 验证当传入空技能列表时，`skills_to_prompt` 函数应返回空字符串，
/// 确保边界条件正确处理。
#[test]
fn skills_to_prompt_empty() {
    let prompt = skills_to_prompt(&[], Path::new("/tmp"));
    assert!(prompt.is_empty());
}

/// 测试包含技能时的提示生成
///
/// 验证当传入技能列表时，生成的提示应包含：
/// - `<available_skills>` 标签包裹
/// - 技能名称（`<name>` 标签）
/// - 技能指令（`<instruction>` 标签）
///
/// 此测试确保完整的技能信息能够正确转换为 XML 格式的提示文本。
#[test]
fn skills_to_prompt_with_skills() {
    // 创建一个测试技能，包含名称、描述、版本和指令
    let skills = vec![Skill {
        name: "test".to_string(),
        description: "A test".to_string(),
        version: "1.0.0".to_string(),
        author: None,
        tags: vec![],
        tools: vec![],
        prompts: vec!["Do the thing.".to_string()],
        location: None,
        ..Skill::default()
    }];
    let prompt = skills_to_prompt(&skills, Path::new("/tmp"));

    // 验证生成的提示包含必要的 XML 标签
    assert!(prompt.contains("<available_skills>"));
    assert!(prompt.contains("<name>test</name>"));
    assert!(prompt.contains("<instruction>Do the thing.</instruction>"));
}

/// 测试紧凑模式下提示生成的信息省略
///
/// 验证在 `Compact` 模式下，`skills_to_prompt_with_mode` 函数应：
/// - 包含技能名称和位置信息
/// - 省略指令（`<instructions>` 和 `<instruction>`）
/// - 省略工具列表（`<tools>`）
/// - 显示"按需加载"提示
///
/// 紧凑模式用于减少提示文本长度，只保留核心元数据，
/// 实际指令和工具信息在需要时从技能文件中加载。
#[test]
fn skills_to_prompt_compact_mode_omits_instructions_and_tools() {
    // 创建一个包含工具和指令的测试技能
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
        ..Skill::default()
    }];

    // 使用紧凑模式生成提示
    let prompt = skills_to_prompt_with_mode(
        &skills,
        Path::new("/tmp/workspace"),
        crate::app::agent::config::SkillsPromptInjectionMode::Compact,
    );

    // 验证包含的元素
    assert!(prompt.contains("<available_skills>"));
    assert!(prompt.contains("<name>test</name>"));
    // 位置应为相对于工作空间的相对路径
    assert!(prompt.contains("<location>skills/test/SKILL.md</location>"));
    // 紧凑模式应显示按需加载提示
    assert!(prompt.contains("loaded on demand"));

    // 验证省略的元素
    assert!(!prompt.contains("<instructions>"));
    assert!(!prompt.contains("<instruction>Do the thing.</instruction>"));
    assert!(!prompt.contains("<tools>"));
}

/// 测试技能工具信息是否正确包含在提示中
///
/// 验证当技能包含工具定义时，生成的提示应包含：
/// - 技能名称
/// - 工具名称（`<name>` 标签）
/// - 工具描述（`<description>` 标签）
/// - 工具类型（`<kind>` 标签）
///
/// 此测试确保工具元数据能够正确嵌入到生成的提示文本中，
/// 使代理了解可用的工具及其用途。
#[test]
fn skills_to_prompt_includes_tools() {
    // 创建一个包含工具的天气查询技能
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
        ..Skill::default()
    }];
    let prompt = skills_to_prompt(&skills, Path::new("/tmp"));

    // 验证技能名称和工具信息都正确包含
    assert!(prompt.contains("weather"));
    assert!(prompt.contains("<name>get_weather</name>"));
    assert!(prompt.contains("<description>Fetch forecast</description>"));
    assert!(prompt.contains("<kind>shell</kind>"));
}

/// 测试 XML 特殊字符的正确转义
///
/// 验证当技能名称、描述或指令中包含 XML 特殊字符时，
/// `skills_to_prompt` 函数应正确转义这些字符：
/// - `<` 转义为 `&lt;`
/// - `>` 转义为 `&gt;`
/// - `&` 转义为 `&amp;`
/// - `"` 转义为 `&quot;`
///
/// 此测试确保生成的 XML 格式提示文本格式正确，避免 XML 注入或格式错误。
/// 转义功能对于安全性至关重要，防止恶意内容破坏 XML 结构。
#[test]
fn skills_to_prompt_escapes_xml_content() {
    // 创建一个包含特殊 XML 字符的技能
    let skills = vec![Skill {
        name: "xml<skill>".to_string(),   // 包含 < 和 >
        description: "A & B".to_string(), // 包含 &
        version: "1.0.0".to_string(),
        author: None,
        tags: vec![],
        tools: vec![],
        prompts: vec!["Use <tool> & check \"quotes\".".to_string()], // 包含多种特殊字符
        location: None,
        ..Skill::default()
    }];

    let prompt = skills_to_prompt(&skills, Path::new("/tmp"));

    // 验证特殊字符被正确转义
    assert!(prompt.contains("<name>xml&lt;skill&gt;</name>"));
    assert!(prompt.contains("<description>A &amp; B</description>"));
    assert!(
        prompt.contains(
            "<instruction>Use &lt;tool&gt; &amp; check &quot;quotes&quot;.</instruction>"
        )
    );
}
