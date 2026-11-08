use super::discovery_url::build_models_url_candidates;
use reqwest::Url;
use vw_api_types::app_providers::{
    ProviderProtocol::{Anthropic, Gemini, OpenaiResponses},
    endpoints::AppProviderEndpointKind::{BaseUrl, FullUrl},
};

fn urls<const N: usize>(expected: [&str; N]) -> [Url; N] {
    expected.map(|value| Url::parse(value).unwrap())
}

#[test]
fn catalog_discovery_url_uses_explicit_models_url_first() {
    let candidates = build_models_url_candidates(
        "https://provider.test/apps/anthropic",
        BaseUrl,
        Anthropic,
        Some(" https://provider.test/exact/models/ "),
        &[],
    )
    .unwrap();
    assert_eq!(candidates, urls(["https://provider.test/exact/models/"]));
}

#[test]
fn catalog_discovery_url_treats_blank_override_as_absent() {
    let candidates = build_models_url_candidates(
        "https://provider.test/",
        BaseUrl,
        OpenaiResponses,
        Some(" "),
        &[],
    )
    .unwrap();
    assert_eq!(candidates, urls(["https://provider.test/v1/models"]));
}

#[test]
fn catalog_discovery_url_reuses_v1_once() {
    let candidates = build_models_url_candidates(
        "https://provider.test/deployment/v1/",
        BaseUrl,
        OpenaiResponses,
        None,
        &[],
    )
    .unwrap();
    assert_eq!(candidates, urls(["https://provider.test/deployment/v1/models"]));
}

#[test]
fn catalog_discovery_url_prefers_existing_non_v1_version() {
    for (host, version) in
        [("open.bigmodel.cn", "v4"), ("api.z.ai", "v4"), ("provider.test", "v10")]
    {
        let base = format!("https://{host}/api/coding/paas/{version}");
        let candidates =
            build_models_url_candidates(&base, BaseUrl, OpenaiResponses, None, &[]).unwrap();
        assert_eq!(candidates, urls([&format!("{base}/models"), &format!("{base}/v1/models")]));
    }
}

#[test]
fn catalog_discovery_url_preserves_unrecognized_deployment_suffix() {
    let candidates = build_models_url_candidates(
        "https://openrouter.ai/api",
        BaseUrl,
        OpenaiResponses,
        None,
        &[],
    )
    .unwrap();
    assert_eq!(candidates, urls(["https://openrouter.ai/api/v1/models"]));
}

#[test]
fn catalog_discovery_url_derives_full_v1_request_from_path() {
    let candidates = build_models_url_candidates(
        "https://provider.test:9443/team%2Fproject/v1/chat/completions",
        FullUrl,
        OpenaiResponses,
        None,
        &[],
    )
    .unwrap();
    assert_eq!(candidates, urls(["https://provider.test:9443/team%2Fproject/v1/models"]));
}

#[test]
fn catalog_discovery_url_derives_full_request_without_v1_from_parent() {
    let candidates = build_models_url_candidates(
        "http://127.0.0.1:8765/tenant/invoke/",
        FullUrl,
        OpenaiResponses,
        None,
        &[],
    )
    .unwrap();
    assert_eq!(candidates, urls(["http://127.0.0.1:8765/tenant/v1/models"]));
}

#[test]
fn catalog_discovery_url_rejects_full_url_without_request_path() {
    assert!(
        build_models_url_candidates("https://provider.test/", FullUrl, OpenaiResponses, None, &[])
            .is_err()
    );
}

#[test]
fn catalog_discovery_url_strips_longest_compatibility_suffix_first() {
    let candidates = build_models_url_candidates(
        "https://provider.test/team/api/anthropic",
        BaseUrl,
        Anthropic,
        None,
        &[],
    )
    .unwrap();
    assert_eq!(
        candidates,
        urls([
            "https://provider.test/team/api/anthropic/v1/models",
            "https://provider.test/team/v1/models",
            "https://provider.test/team/models",
        ])
    );
}

#[test]
fn catalog_discovery_url_keeps_source_site_fallback_order() {
    for base in [
        "https://api.deepseek.com/anthropic",
        "https://www.right.codes/claude",
        "https://api.stepfun.com/step_plan",
        "https://ark.cn-beijing.volces.com/api/coding",
        "https://provider.test/api/claudecode",
        "https://provider.test/claudecode",
        "https://provider.test/coding",
    ] {
        let root = Url::parse(base).unwrap().origin().ascii_serialization();
        let candidates =
            build_models_url_candidates(base, BaseUrl, OpenaiResponses, None, &[]).unwrap();
        assert_eq!(
            candidates,
            urls([
                &format!("{base}/v1/models"),
                &format!("{root}/v1/models"),
                &format!("{root}/models")
            ]),
        );
    }
}

#[test]
fn catalog_discovery_url_appends_compatible_mode_for_apps_anthropic() {
    for host in ["dashscope.aliyuncs.com", "token-plan.maas.qianwenaiapi.com"] {
        let root = format!("https://{host}");
        let candidates = build_models_url_candidates(
            &format!("{root}/apps/anthropic/"),
            BaseUrl,
            Anthropic,
            None,
            &[],
        )
        .unwrap();
        assert_eq!(
            candidates,
            urls([
                &format!("{root}/apps/anthropic/v1/models"),
                &format!("{root}/v1/models"),
                &format!("{root}/models"),
                &format!("{root}/compatible-mode/v1/models"),
            ])
        );
    }
}

#[test]
fn catalog_discovery_url_uses_google_native_version_for_base_urls() {
    for suffix in ["", "/v1", "/v1beta/"] {
        let candidates = build_models_url_candidates(
            &format!("https://provider.test/google{suffix}"),
            BaseUrl,
            Gemini,
            Some(" "),
            &[],
        )
        .unwrap();
        assert_eq!(candidates, urls(["https://provider.test/google/v1beta/models"]));
    }
}

#[test]
fn catalog_discovery_url_honors_google_override_and_full_url() {
    let base = "https://provider.test/gateway/v1/chat/completions";
    let override_url = "https://provider.test/native/models";
    let candidates =
        build_models_url_candidates(base, FullUrl, Gemini, Some(override_url), &[]).unwrap();
    assert_eq!(candidates, urls([override_url]));
    let candidates = build_models_url_candidates(base, FullUrl, Gemini, None, &[]).unwrap();
    assert_eq!(candidates, urls(["https://provider.test/gateway/v1/models"]));
}

#[test]
fn catalog_discovery_url_rejects_unlisted_cross_origin_override() {
    let base = "https://provider.test";
    let allowed = urls(["https://models.provider.test/allowed"]);
    for target in [
        "https://models.provider.test/other",
        "https://provider.test:8443/models",
        "https://provider.test.evil.test/models",
    ] {
        let error = build_models_url_candidates(base, BaseUrl, Anthropic, Some(target), &allowed)
            .unwrap_err();
        assert_eq!(error, "model discovery override is not authorized");
    }
    let error = build_models_url_candidates(
        "http://127.0.0.1:8765",
        BaseUrl,
        Anthropic,
        Some("https://127.0.0.1:8765/models"),
        &[],
    )
    .unwrap_err();
    assert_eq!(error, "model discovery override is not authorized");
}

#[test]
fn catalog_discovery_url_accepts_only_explicitly_authorized_cross_origin_endpoint() {
    let allowed = "https://models.provider.test/api/models";
    let candidates = build_models_url_candidates(
        "https://provider.test",
        BaseUrl,
        OpenaiResponses,
        Some(allowed),
        &urls([allowed]),
    )
    .unwrap();
    assert_eq!(candidates, urls([allowed]));
}

#[test]
fn catalog_discovery_url_rejects_invalid_request_endpoints_without_echoing_them() {
    for invalid in [
        "",
        "provider.test/v1",
        "https://user:fixture-secret@provider.test",
        "https://provider.test/?key=fixture-secret",
        "https://provider.test/models#fixture-secret",
        "https://provider.test\\evil.test/models",
        "http://provider.test/models",
        "file:///etc/hosts",
    ] {
        let error =
            build_models_url_candidates(invalid, BaseUrl, Anthropic, None, &[]).unwrap_err();
        assert_eq!(error, "invalid application provider endpoint");
    }
    let error = build_models_url_candidates(
        "https://provider.test",
        BaseUrl,
        Anthropic,
        Some("https://provider.test/?key=fixture-secret"),
        &[],
    )
    .unwrap_err();
    assert_eq!(error, "invalid application provider endpoint");
}
