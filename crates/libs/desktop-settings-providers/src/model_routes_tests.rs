// 验证模型路由行的规范化、必填校验与错误定位。
// 测试夹具围绕目标分支构造可控状态；跨模块行为由对应集成测试负责。

use super::*;
use crate::state::ModelRoute;


fn route(pattern: &str, provider: &str, model: &str, priority: &str) -> ModelRoute {
    ModelRoute {
        pattern: pattern.to_string(),
        provider: provider.to_string(),
        model: model.to_string(),
        priority_input: priority.to_string(),
    }
}

// 数据边界：覆盖规范化、默认值以及相关状态字段的既定规则。
// 这里固定可观察结果，避免依赖处理过程中的临时实现细节。
#[test]
fn validate_routes_skips_blank_rows_and_trims_complete_routes() {
    let mut input = Vec::new();
    input =
        vec![route("  ", " ", "\t", ""), route("  coding  ", "  openai  ", "  gpt-5  ", "  7  ")];

    let routes = validate_routes(&input).expect("blank and complete routes should be valid");

    assert_eq!(routes.len(), 1);
    assert_eq!(routes[0].hint, "coding");
    assert_eq!(routes[0].provider, "openai");
    assert_eq!(routes[0].model, "gpt-5");
    assert_eq!(routes[0].max_tokens, None);
    assert_eq!(routes[0].api_key, None);
}

#[test]
fn validate_routes_accepts_complete_route_without_priority() {
    let mut input = Vec::new();
    input =
        vec![route("  analysis  ", "  anthropic  ", "  claude-sonnet  ", "  ")];

    let routes = validate_routes(&input).expect("priority is optional");

    assert_eq!(routes.len(), 1);
    assert_eq!(routes[0].hint, "analysis");
    assert_eq!(routes[0].provider, "anthropic");
    assert_eq!(routes[0].model, "claude-sonnet");
}

// 数据边界：覆盖规范化、默认值以及相关状态字段的既定规则。
// 这里固定可观察结果，避免依赖处理过程中的临时实现细节。
#[test]
fn validate_routes_reports_provider_model_and_priority_errors_with_row_number() {
    let mut input = Vec::new();
    input =
        vec![ModelRoute::default(), route("chat", " ", "gpt-5", "1")];
    assert_eq!(validate_routes(&input).unwrap_err(), "第 2 条路由缺少 provider");

    input[1].provider = "openai".to_string();
    input[1].model.clear();
    assert_eq!(validate_routes(&input).unwrap_err(), "第 2 条路由缺少 model");

    input[1].model = "gpt-5".to_string();
    input[1].priority_input = "-1".to_string();
    assert_eq!(validate_routes(&input).unwrap_err(), "第 2 条路由的 priority 不是有效数字");
}
