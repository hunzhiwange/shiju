use crate::Error;
use crate::core::LanguageModel;

use super::resolver::{OpenAICompatibleResolveOptions, resolve_openai_compatible_model_with_env};

#[cfg(feature = "alibaba-cn")]
const VERSIONED_MODEL_REF: &str = "alibaba-cn/qwen3.7-plus-2026-05-26";

fn resolve_with_test_env(
    model_ref: &str,
    options: OpenAICompatibleResolveOptions,
) -> crate::Result<super::OpenAICompatible<crate::core::DynamicModel>> {
    resolve_openai_compatible_model_with_env(model_ref, options, |name| {
        (name == "DASHSCOPE_API_KEY").then(|| "registry-api-key".to_owned())
    })
}

#[test]
#[cfg(feature = "alibaba-cn")]
fn resolves_known_provider_with_versioned_dynamic_model() {
    let model = resolve_with_test_env(VERSIONED_MODEL_REF, Default::default())
        .expect("registered provider should resolve from its environment metadata");

    assert_eq!(model.name(), "qwen3.7-plus-2026-05-26");
    assert_eq!(model.settings.provider_name, "alibaba-cn");
    assert_eq!(
        model.settings.base_url.trim_end_matches('/'),
        "https://dashscope.aliyuncs.com/compatible-mode/v1"
    );
    assert_eq!(model.settings.api_key, "registry-api-key");
    assert_eq!(model.settings.path, None);
}

#[test]
#[cfg(feature = "alibaba-cn")]
fn does_not_require_model_catalog_membership() {
    let model = resolve_with_test_env(VERSIONED_MODEL_REF, Default::default())
        .expect("a dynamic date snapshot should not require a static catalog entry");

    assert_eq!(model.name(), "qwen3.7-plus-2026-05-26");
}

#[test]
fn unknown_provider_requires_explicit_base_url() {
    let error = resolve_with_test_env(
        "custom-provider/org/future-model",
        OpenAICompatibleResolveOptions {
            api_key: Some("explicit-api-key".to_owned()),
            ..Default::default()
        },
    )
    .expect_err("an unknown provider without an explicit base URL must be rejected");

    assert!(matches!(
        error,
        Error::InvalidInput(message)
            if message.contains("custom-provider") && message.contains("base_url")
    ));
}

#[test]
fn unknown_provider_uses_openai_compatible_with_explicit_config() {
    let model = resolve_with_test_env(
        "custom-provider/org/future-model",
        OpenAICompatibleResolveOptions {
            api_key: Some("explicit-api-key".to_owned()),
            base_url: Some("https://custom.example/v1".to_owned()),
            path: Some("chat/completions".to_owned()),
        },
    )
    .expect("complete explicit configuration should resolve an unknown provider");

    assert_eq!(model.name(), "org/future-model");
    assert_eq!(model.settings.provider_name, "custom-provider");
    assert_eq!(
        model.settings.base_url.trim_end_matches('/'),
        "https://custom.example/v1"
    );
    assert_eq!(model.settings.api_key, "explicit-api-key");
    assert_eq!(model.settings.path.as_deref(), Some("chat/completions"));
}

#[test]
#[cfg(feature = "alibaba-cn")]
fn explicit_options_override_registry_values() {
    let model = resolve_with_test_env(
        VERSIONED_MODEL_REF,
        OpenAICompatibleResolveOptions {
            api_key: Some("override-api-key".to_owned()),
            base_url: Some("https://override.example/api".to_owned()),
            path: Some("custom/chat".to_owned()),
        },
    )
    .expect("explicit options should override registered values");

    assert_eq!(model.name(), "qwen3.7-plus-2026-05-26");
    assert_eq!(model.settings.provider_name, "alibaba-cn");
    assert_eq!(
        model.settings.base_url.trim_end_matches('/'),
        "https://override.example/api"
    );
    assert_eq!(model.settings.api_key, "override-api-key");
    assert_eq!(model.settings.path.as_deref(), Some("custom/chat"));
}
