use super::identity::{MetadataEndpointKey, find_metadata_model};
use std::collections::HashMap;
use vw_api_types::{
    agent_apps::AppId,
    id::TenantScope,
    model_catalog::{ModelCatalogIdentity, ScopedModelCatalogIdentity},
};

fn identity(endpoint: &str) -> ScopedModelCatalogIdentity {
    ScopedModelCatalogIdentity {
        scope: TenantScope::default(),
        deployment: ModelCatalogIdentity {
            target_host_id: "host-a".into(),
            app_id: AppId::OpenCode,
            provider_id: "provider-a".into(),
            endpoint: endpoint.into(),
        },
        model_id: "glm-5".into(),
    }
}

fn endpoint(raw: &str) -> MetadataEndpointKey {
    MetadataEndpointKey::parse(raw).unwrap()
}

#[test]
fn catalog_identity_separates_plan_paths() {
    let standard = "https://api.z.ai/api/paas/v4";
    let coding = "https://api.z.ai/api/coding/paas/v4";
    assert_ne!(identity(standard), identity(coding));
    let metadata = HashMap::from([
        (endpoint(standard), [("glm-5", 200_000)]),
        (endpoint(coding), [("glm-5", 202_752)]),
    ]);
    let standard_rows = &metadata[&endpoint(standard)];
    let coding_rows = &metadata[&endpoint(coding)];
    assert_eq!(
        find_metadata_model("glm-5", standard_rows.iter().map(|(id, value)| (*id, value))),
        Some(&200_000),
    );
    assert_eq!(
        find_metadata_model("glm-5", coding_rows.iter().map(|(id, value)| (*id, value))),
        Some(&202_752),
    );
}

#[test]
fn catalog_identity_separates_tenant_and_execution_host() {
    let exact = identity("https://api.z.ai/api/paas/v4");
    let stored = HashMap::from([(exact.clone(), 200_000)]);
    let mut other = exact.clone();
    other.scope = TenantScope::new("platform", "another-company").unwrap();
    assert_eq!(stored.get(&other), None);
    other.scope = exact.scope.clone();
    other.deployment.target_host_id = "host-b".into();
    assert_eq!(stored.get(&other), None);
    assert_eq!(stored.get(&exact), Some(&200_000));
}

#[test]
fn catalog_identity_separates_app_and_provider_deployments() {
    let exact = identity("https://api.z.ai/api/paas/v4");
    let stored = HashMap::from([(exact.clone(), 200_000)]);
    let mut other = exact.clone();
    other.deployment.app_id = AppId::Codex;
    assert_eq!(stored.get(&other), None);
    other.deployment.app_id = exact.deployment.app_id;
    other.deployment.provider_id = "provider-b".into();
    assert_eq!(stored.get(&other), None);
}

#[test]
fn catalog_identity_retains_exact_execution_id_and_endpoint() {
    let exact = identity("https://opencode.ai/zen/go/v1");
    let mut other = exact.clone();
    other.model_id = "Vendor/GLM-5:free".into();
    assert_ne!(exact, other);
    assert_eq!(other.model_id.as_ref(), "Vendor/GLM-5:free");
    other.model_id = exact.model_id.clone();
    other.deployment.endpoint = "https://opencode.ai/inference/go/openai/v1".into();
    assert_ne!(exact, other);
    assert_eq!(endpoint(&exact.deployment.endpoint), endpoint(&other.deployment.endpoint));
}

#[test]
fn catalog_identity_normalizes_trailing_versions_and_requests() {
    let base = endpoint("https://API.Z.AI/api/coding/paas");
    for raw in [
        "api.z.ai/api/coding/paas/",
        "https://api.z.ai/api/coding/paas/v1/",
        "https://api.z.ai/api/coding/paas/v4/chat/completions",
        "https://api.z.ai/api/coding/paas/v1beta/messages",
        "https://api.z.ai/api/coding/paas/v1alpha2/responses/",
    ] {
        assert_eq!(endpoint(raw), base, "{raw}");
    }
    assert_ne!(endpoint("https://api.z.ai/api/v4/coding/paas"), base);
    assert_ne!(endpoint("https://api.z.ai/api/coding/paas/completions"), base);
}

#[test]
fn catalog_identity_maps_zen_aliases_only_at_the_canonical_origin() {
    for (old, new) in [("/zen", "/inference/openai"), ("/zen/go", "/inference/go/openai")] {
        assert_eq!(
            endpoint(&format!("https://opencode.ai{old}/v1")),
            endpoint(&format!("https://opencode.ai{new}/v1")),
        );
        for origin in ["https://opencode.ai:8443", "https://relay.example.test"] {
            assert_ne!(endpoint(&format!("{origin}{old}")), endpoint(&format!("{origin}{new}")));
        }
    }
    assert_ne!(endpoint("https://opencode.ai/zen"), endpoint("https://opencode.ai/zen/go"));
}

#[test]
fn catalog_identity_keeps_origins_and_distinct_paths_separate() {
    let base = endpoint("https://api.z.ai/api/paas");
    assert_eq!(endpoint("https://api.z.ai:443/api/paas/v4"), base);
    for raw in [
        "https://open.bigmodel.cn/api/paas",
        "https://api.z.ai:8443/api/paas",
        "https://api.z.ai/api/PAAS",
        "https://api.z.ai/api//paas",
    ] {
        assert_ne!(endpoint(raw), base, "{raw}");
    }
    assert_ne!(endpoint("http://localhost:8080/v1"), endpoint("https://localhost:8080/v1"));
}

#[test]
fn catalog_identity_rejects_invalid_metadata_endpoints() {
    for raw in [
        "",
        "https://",
        "file:///api/v1",
        "http://api.z.ai/v1",
        "https://user:secret@api.z.ai/v1",
        "https://api.z.ai/v1?key=secret",
        "https://api.z.ai/v1#fragment",
    ] {
        assert_eq!(MetadataEndpointKey::parse(raw), None, "{raw}");
    }
}

#[test]
fn catalog_identity_prefers_exact_then_case_insensitive_model_id() {
    let rows = [("glm-5", 1), ("z-ai/glm-5", 2), ("Z-AI/GLM-5", 3)];
    assert_eq!(find_metadata_model("Z-AI/GLM-5", rows.iter().map(|(id, v)| (*id, v))), Some(&3));
    assert_eq!(find_metadata_model("Z-AI/glm-5", rows.iter().map(|(id, v)| (*id, v))), Some(&2));
}

#[test]
fn catalog_identity_matches_source_id_variants_within_selected_rows() {
    let rows = [("upstream/vendor/model-name", 1), ("unrelated", 2)];
    let find = |id| find_metadata_model(id, rows.iter().map(|(key, v)| (*key, v)));
    assert_eq!(find("relay/MODEL@NAME[1M]:free"), Some(&1));
    assert_eq!(find("vendor/Model-Name [1m]"), Some(&1));
    assert_eq!(find("model-name-extra"), None);
    assert_eq!(find("vendor/"), None);
}
