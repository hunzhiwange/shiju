use vw_api_types::apiql::ApiQlListMode;
use vw_api_types::id::TenantScope;

use super::functions::{
    api_ql, api_ql_list_count, api_ql_list_only, api_ql_list_page, api_ql_with_mode,
};

fn tenant() -> TenantScope {
    TenantScope::new("apiql-functions-platform", "apiql-functions-company")
        .expect("ApiQL function tenant")
}

#[tokio::test]
async fn apiql_function_wrappers_reject_invalid_query_keys_before_runtime_lookup() {
    let cases = [
        api_ql(tenant(), "product:product", "[name]=apple").await,
        api_ql_list_count(tenant(), "product:product", "[name]=apple").await,
        api_ql_list_only(tenant(), "product:product", "[name]=apple").await,
        api_ql_list_page(tenant(), "product:product", "[name]=apple").await,
    ];

    for error in cases.into_iter().map(|result| result.expect_err("invalid query")) {
        assert!(error.to_string().contains("Invalid ApiQL query key"));
    }
}

#[tokio::test]
async fn apiql_with_mode_uses_requested_list_mode_during_query_normalization() {
    let error = api_ql_with_mode(
        tenant(),
        "product:product",
        "expectPage=maybe",
        ApiQlListMode::IndexCount,
    )
    .await
    .expect_err("invalid boolean should fail while normalizing selected mode");

    assert!(error.to_string().contains("Invalid ApiQL boolean"));
}

#[tokio::test]
async fn apiql_functions_report_unknown_entities_from_empty_registry() {
    let unique_tenant =
        TenantScope::new("apiql-functions-empty-registry", "apiql-functions-empty-company")
            .expect("unique tenant");

    let error = api_ql(unique_tenant, "missing:entity", "").await.expect_err("unknown entity");

    assert!(error.to_string().contains("ApiQL entity not registered: missing:entity"));
}
