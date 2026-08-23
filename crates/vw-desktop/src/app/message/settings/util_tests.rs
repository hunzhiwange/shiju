// 验证设置列表解析、提供商连接判定与同步保存包装。
// 测试夹具围绕目标分支构造可控状态；跨模块行为由对应集成测试负责。

use super::*;
use vw_shared::provider::types::{Info, ProviderSource};

fn provider(source: ProviderSource, key: Option<&str>) -> Info {
    Info {
        id: "provider-id".to_string(),
        name: "Provider".to_string(),
        source,
        env: Vec::new(),
        key: key.map(ToString::to_string),
        options: Default::default(),
        models: Default::default(),
    }
}

// 数据边界：覆盖规范化、默认值以及相关状态字段的既定规则。
// 这里固定可观察结果，避免依赖处理过程中的临时实现细节。
#[test]
fn parse_list_splits_mixed_separators_and_trims_values() {
    assert_eq!(
        parse_comma_or_newline_list(" alpha, beta\n gamma \r\nalpha "),
        vec!["alpha", "beta", "gamma", "alpha"]
    );
}

#[test]
fn parse_list_discards_empty_and_whitespace_only_values() {
    assert!(parse_comma_or_newline_list("  ,\n \r\n,").is_empty());
    assert_eq!(parse_comma_or_newline_list(",value,,\n"), vec!["value"]);
}

// 数据边界：覆盖规范化、默认值以及相关状态字段的既定规则。
// 这里固定可观察结果，避免依赖处理过程中的临时实现细节。
#[test]
fn provider_connection_requires_credentials_not_only_configuration() {
    for source in [ProviderSource::Api, ProviderSource::Env] {
        assert!(is_provider_connected(&provider(source, None)));
    }
    assert!(!is_provider_connected(&provider(ProviderSource::Config, None)));
    assert!(is_provider_connected(&provider(ProviderSource::Config, Some(" secret "))));
}

#[test]
fn custom_provider_requires_non_blank_key_to_be_connected() {
    assert!(!is_provider_connected(&provider(ProviderSource::Custom, None)));
    assert!(!is_provider_connected(&provider(ProviderSource::Custom, Some("  \t"))));
    assert!(is_provider_connected(&provider(ProviderSource::Custom, Some(" secret "))));
}

#[cfg(not(target_arch = "wasm32"))]
// 提交与删除：覆盖保存、安装或移除操作前后的状态衔接。
// 这里关注校验、任务载荷和结果反馈，不绑定底层实现。
#[test]
fn sync_save_executes_persist_and_returns_success() {
    let mut called = false;

    let result = sync_save(|| {
        called = true;
        Ok(())
    });

    assert_eq!(result, Ok(()));
    assert!(called);
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn sync_save_preserves_persist_error() {
    assert_eq!(sync_save(|| Err("persist failed".to_string())), Err("persist failed".to_string()));
}

#[cfg(target_arch = "wasm32")]
// 失败与边界：覆盖非法、缺失、过期或冲突场景的既定处理规则。
// 重点确认现有状态与错误反馈不会被意外的部分更新污染。
#[test]
fn sync_save_skips_synchronous_persist_on_wasm() {
    let mut called = false;

    let result = sync_save(|| {
        called = true;
        Err("must not run".to_string())
    });

    assert_eq!(result, Ok(()));
    assert!(!called);
}
