//! 集成模块的单元测试
//!
//! 本模块包含针对集成系统各个组件的测试用例，验证以下功能：
//! - 集成分类枚举的完整性和标签正确性
//! - 集成命令处理器的行为（信息查询、列表展示、搜索等）
//! - 分类过滤器和状态过滤器的解析逻辑
//!
//! 所有测试均使用默认配置进行，确保集成系统的基础行为符合预期。

use super::*;

/// 测试集成分类枚举包含所有变体且不重复
///
/// 验证 `IntegrationCategory::all()` 方法返回的分类数量正确（9个），
/// 并且每个分类的标签都能正确映射到预期的显示文本。
#[test]
fn integration_category_all_includes_every_variant_once() {
    let all = IntegrationCategory::all();
    assert_eq!(all.len(), 9);

    let labels: Vec<&str> = all.iter().map(|cat| cat.label()).collect();
    assert!(labels.contains(&"Chat Providers"));
    assert!(labels.contains(&"AI Models"));
    assert!(labels.contains(&"Productivity"));
    assert!(labels.contains(&"Music & Audio"));
    assert!(labels.contains(&"Smart Home"));
    assert!(labels.contains(&"Tools & Automation"));
    assert!(labels.contains(&"Media & Creative"));
    assert!(labels.contains(&"Social"));
    assert!(labels.contains(&"Platforms"));
}

/// 测试信息查询命令对已知集成名称的大小写不敏感性
///
/// 从注册表中获取第一个集成的名称，转换为小写后查询，
/// 验证系统能够正确识别（不区分大小写）。
#[test]
fn handle_command_info_is_case_insensitive_for_known_integrations() {
    let config = Config::default();
    let first_name = registry::all_integrations()
        .first()
        .expect("registry should define at least one integration")
        .name
        .to_lowercase();

    let result = handle_command(IntegrationCommands::Info { name: first_name }, &config);

    assert!(result.is_ok());
}

/// 测试信息查询命令对未知集成返回错误
///
/// 使用不存在的集成名称调用 Info 命令，验证系统返回包含
/// "Unknown integration" 字符串的错误信息。
#[test]
fn handle_command_info_returns_error_for_unknown_integration() {
    let config = Config::default();
    let result = handle_command(
        IntegrationCommands::Info { name: "definitely-not-a-real-integration".into() },
        &config,
    );

    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(err.contains("Unknown integration"));
}

/// 测试无过滤条件的列表命令执行成功
///
/// 调用 List 命令时不指定分类和状态过滤器，
/// 验证能够成功返回所有集成的列表。
#[test]
fn list_all_integrations_succeeds() {
    let config = Config::default();
    let result =
        handle_command(IntegrationCommands::List { category: None, status: None }, &config);
    assert!(result.is_ok());
}

/// 测试带分类过滤的列表命令执行成功
///
/// 使用 "chat" 作为分类过滤器调用 List 命令，
/// 验证分类过滤功能正常工作。
#[test]
fn list_with_category_filter_succeeds() {
    let config = Config::default();
    let result = handle_command(
        IntegrationCommands::List { category: Some("chat".into()), status: None },
        &config,
    );
    assert!(result.is_ok());
}

/// 测试带状态过滤的列表命令执行成功
///
/// 使用 "available" 作为状态过滤器调用 List 命令，
/// 验证状态过滤功能正常工作。
#[test]
fn list_with_status_filter_succeeds() {
    let config = Config::default();
    let result = handle_command(
        IntegrationCommands::List { category: None, status: Some("available".into()) },
        &config,
    );
    assert!(result.is_ok());
}

/// 测试使用无效分类过滤器的列表命令失败
///
/// 使用不存在的分类名称调用 List 命令，
/// 验证系统返回包含 "Unknown category" 字符串的错误。
#[test]
fn list_with_invalid_category_fails() {
    let config = Config::default();
    let result = handle_command(
        IntegrationCommands::List { category: Some("nonexistent".into()), status: None },
        &config,
    );
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Unknown category"));
}

/// 测试使用无效状态过滤器的列表命令失败
///
/// 使用无效的状态值 "bogus" 调用 List 命令，
/// 验证系统返回包含 "Unknown status" 字符串的错误。
#[test]
fn list_with_invalid_status_fails() {
    let config = Config::default();
    let result = handle_command(
        IntegrationCommands::List { category: None, status: Some("bogus".into()) },
        &config,
    );
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Unknown status"));
}

/// 测试搜索命令能够找到匹配的集成
///
/// 使用 "telegram" 作为查询关键词调用 Search 命令，
/// 验证搜索功能能够正确匹配并返回结果。
#[test]
fn search_finds_matching_integrations() {
    let config = Config::default();
    let result = handle_command(IntegrationCommands::Search { query: "telegram".into() }, &config);
    assert!(result.is_ok());
}

/// 测试搜索命令在无匹配时仍能成功执行
///
/// 使用不可能匹配任何集成的查询词 "zzz-no-match-zzz" 调用 Search 命令，
/// 验证即使无匹配结果也不会返回错误（空结果是合法的）。
#[test]
fn search_no_match_succeeds() {
    let config = Config::default();
    let result =
        handle_command(IntegrationCommands::Search { query: "zzz-no-match-zzz".into() }, &config);
    assert!(result.is_ok());
}

/// 测试分类过滤器解析函数覆盖所有别名
///
/// 验证 `parse_category_filter` 函数能够正确识别所有有效的分类别名，
/// 包括 "chat"、"ai"、"models"、"productivity"、"music"、"smart-home"、
/// "tools"、"media"、"social"、"platform"，同时拒绝无效值 "bogus"。
#[test]
fn parse_category_filter_covers_all_aliases() {
    assert!(parse_category_filter("chat").is_some());
    assert!(parse_category_filter("ai").is_some());
    assert!(parse_category_filter("models").is_some());
    assert!(parse_category_filter("productivity").is_some());
    assert!(parse_category_filter("music").is_some());
    assert!(parse_category_filter("smart-home").is_some());
    assert!(parse_category_filter("tools").is_some());
    assert!(parse_category_filter("media").is_some());
    assert!(parse_category_filter("social").is_some());
    assert!(parse_category_filter("platform").is_some());
    assert!(parse_category_filter("bogus").is_none());
}

/// 测试状态过滤器解析函数覆盖所有别名
///
/// 验证 `parse_status_filter` 函数能够正确识别所有有效的状态别名，
/// 包括 "active"、"available"、"coming-soon"、"soon"，
/// 同时拒绝无效值 "bogus"。
#[test]
fn parse_status_filter_covers_all_aliases() {
    assert!(parse_status_filter("active").is_some());
    assert!(parse_status_filter("available").is_some());
    assert!(parse_status_filter("coming-soon").is_some());
    assert!(parse_status_filter("soon").is_some());
    assert!(parse_status_filter("bogus").is_none());
}
