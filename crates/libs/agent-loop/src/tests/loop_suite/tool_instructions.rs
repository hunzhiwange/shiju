//! 工具指令构建与格式化测试模块
//!
//! 本模块测试代理循环中的工具指令生成和格式化功能：
//! - 验证工具指令文本是否包含所有必需的工具
//! - 验证工具转换为 OpenAI 格式是否生成有效的 schema
//! - 确保工具描述符符合协议规范

// 由 vw-agent 的测试入口绑定真实 default_tools；不在领域库中注册。
use super::*;

/// 测试构建工具指令是否包含所有必需的工具
///
/// 此测试验证 `build_tool_instructions` 函数生成的指令文本：
/// - 包含工具使用协议的标题
/// - 包含工具调用的格式标记（如 ）
/// - 包含核心工具：bash、file_read、edit、file_write
///
/// # 测试步骤
/// 1. 使用默认安全策略创建工具集合
/// 2. 调用 `build_tool_instructions` 生成指令文本
/// 3. 断言指令文本包含所有必需的组件
#[test]
fn build_tool_instructions_includes_all_tools() {
    use crate::app::agent::security::SecurityPolicy;

    // 使用默认配置创建安全策略实例
    let security = Arc::new(SecurityPolicy::from_config(
        &crate::app::agent::config::AutonomyConfig::default(),
        std::path::Path::new("/tmp"),
    ));

    // 获取默认工具集合
    let tools = crate::app::agent::tools::default_tools(security);

    // 构建工具指令文本
    let instructions = build_tool_instructions(&tools);

    // 验证指令包含工具使用协议标题
    assert!(instructions.contains("## Tool Use Protocol"));

    // 验证指令包含工具调用格式标记
    assert!(instructions.contains(" "));

    // 验证指令包含核心工具名称
    assert!(instructions.contains("bash"));
    assert!(instructions.contains("file_read"));
    assert!(instructions.contains("file_edit"));
    assert!(instructions.contains("file_write"));
}

/// 测试工具转换为 OpenAI 格式是否生成有效的 schema
///
/// 此测试验证 `tools_to_openai_format` 函数生成的 JSON schema：
/// - 每个工具条目的 type 字段为 "function"
/// - 每个工具包含有效的 name 和 description 字符串
/// - 格式化后的工具集合包含已知的核心工具
///
/// # 测试步骤
/// 1. 使用默认安全策略创建工具集合
/// 2. 调用 `tools_to_openai_format` 转换为 OpenAI 格式
/// 3. 验证每个工具条目的结构有效性
/// 4. 验证已知工具（bash、file_read）存在于结果中
#[test]
fn tools_to_openai_format_produces_valid_schema() {
    use crate::app::agent::security::SecurityPolicy;

    // 使用默认配置创建安全策略实例
    let security = Arc::new(SecurityPolicy::from_config(
        &crate::app::agent::config::AutonomyConfig::default(),
        std::path::Path::new("/tmp"),
    ));

    // 获取默认工具集合
    let tools = crate::app::agent::tools::default_tools(security);

    // 将工具转换为 OpenAI 格式
    let formatted = tools_to_openai_format(&tools);

    // 验证转换结果非空
    assert!(!formatted.is_empty());

    // 验证每个工具条目的结构符合 OpenAI schema 规范
    for tool_json in &formatted {
        assert_eq!(tool_json["type"], "function");
        assert!(tool_json["function"]["name"].is_string());
        assert!(tool_json["function"]["description"].is_string());
        assert!(!tool_json["function"]["name"].as_str().unwrap().is_empty());
    }

    // 提取所有工具名称用于验证
    let names: Vec<&str> =
        formatted.iter().filter_map(|t| t["function"]["name"].as_str()).collect();

    // 验证已知的核心工具存在于转换结果中
    assert!(names.contains(&"bash"));
    assert!(names.contains(&"file_read"));
    assert!(names.contains(&"file_edit"));
}
