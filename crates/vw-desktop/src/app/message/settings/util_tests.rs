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

#[test]
fn provider_with_builtin_connection_source_is_connected_without_key() {
    for source in [ProviderSource::Api, ProviderSource::Env, ProviderSource::Config] {
        assert!(is_provider_connected(&provider(source, None)));
    }
}

#[test]
fn custom_provider_requires_non_blank_key_to_be_connected() {
    assert!(!is_provider_connected(&provider(ProviderSource::Custom, None)));
    assert!(!is_provider_connected(&provider(ProviderSource::Custom, Some("  \t"))));
    assert!(is_provider_connected(&provider(ProviderSource::Custom, Some(" secret "))));
}

#[cfg(not(target_arch = "wasm32"))]
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
