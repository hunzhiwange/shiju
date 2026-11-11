use super::app_routing::configure_with_policy;
use crate::{
    app_providers::AppProviderStore, app_routing::RoutingListener, global_database::GlobalDatabase,
};
use axum::{
    Extension, Json, Router,
    body::{Body, to_bytes},
    http::{HeaderMap, Request, StatusCode},
    middleware,
    routing::post,
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{fs, os::unix::fs::PermissionsExt, sync::Arc};
use tokio::{net::TcpListener, sync::mpsc};
use tower::ServiceExt;
use vw_agent_apps::providers::{
    codex_catalog::CodexFiles, live::commit::*, mode_state::StackState,
};
use vw_api_types::{
    agent_apps::AppId,
    app_providers::codex_catalog::CodexProfile,
    id::{COMPANY_ID_HEADER, PLATFORM_ID_HEADER, ProjectScope, TenantScope},
};
const TOKEN: &str = "aggregation-task16-codex-manager-credential";
fn profile(endpoint: &str, id: &str, model: &str) -> CodexProfile {
    CodexProfile {
        name: id.into(),
        config: format!(
            "model='{model}'\nmodel_provider='vendor'\n[model_providers.vendor]\nname='Fixture'\nbase_url='{endpoint}/v1'\nwire_api='responses'\n"
        ),
        api_key: Some(format!("fixture-secret-for-{id}")),
        preserve_login: false,
    }
}
async fn request(router: &Router, method: &str, path: &str, body: Value) -> (StatusCode, Value) {
    let response = router
        .clone()
        .oneshot(
            Request::builder()
                .method(method)
                .uri(path)
                .header(PLATFORM_ID_HEADER, "platform")
                .header(COMPANY_ID_HEADER, "company")
                .header("authorization", format!("Bearer {TOKEN}"))
                .header("content-type", "application/json")
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    let status = response.status();
    let bytes = to_bytes(response.into_body(), 1024 * 1024).await.unwrap();
    (status, serde_json::from_slice(&bytes).unwrap_or(Value::Null))
}
#[tokio::test]
async fn aggregation_t16_codex_catalog_pointer_record_and_native_responses_settle_together() {
    let root = tempfile::tempdir().unwrap();
    fs::set_permissions(root.path(), fs::Permissions::from_mode(0o700)).unwrap();
    let private = root.path().join("private");
    fs::create_dir(&private).unwrap();
    fs::set_permissions(&private, fs::Permissions::from_mode(0o700)).unwrap();
    let upstream = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let endpoint = format!("http://{}", upstream.local_addr().unwrap());
    let (send, mut requests) = mpsc::unbounded_channel();
    let server = Router::new().route(
        "/v1/responses",
        post(move |headers: HeaderMap, Json(body): Json<Value>| {
            let send = send.clone();
            async move {
                send.send((headers, body)).unwrap();
                Json(json!({"id":"fixture","object":"response","status":"completed","output":[]}))
            }
        }),
    );
    let upstream_task = tokio::spawn(async move { axum::serve(upstream, server).await.unwrap() });
    let db = GlobalDatabase::for_home(root.path()).unwrap();
    let store = AppProviderStore::new(db.clone()).unwrap();
    let tenant = TenantScope::new("platform", "company").unwrap();
    let project = ProjectScope::new(tenant.clone(), "project".into()).unwrap();
    for (id, model) in [("alpha", "model-a"), ("beta", "model-b")] {
        store.codex_save(&tenant, id, &profile(&endpoint, id, model), None, None).unwrap();
    }
    let tx = ProviderTransactions::new(&private, ExecutionHostId::new("host").unwrap()).unwrap();
    let mut files = CodexFiles::open(root.path()).unwrap();
    fs::write(root.path().join(".codex/config.toml"),"# user comment\nmodel='model-a'\nmodel_provider='custom'\n[model_providers.custom]\nname='Old local'\nbase_url='http://127.0.0.1:7/v1'\nwire_api='responses'\n[projects.fixture]\ntrust_level='trusted'\n").unwrap();
    let auth = br#"{"tokens":{"access_token":"fixture-login-kept"}}"#;
    fs::write(root.path().join(".codex/auth.json"), auth).unwrap();
    tx.commit_project_checked(
        &project,
        &mut files.guard,
        &tx.state(AppId::Codex).unwrap(),
        true,
        &[],
        PendingTarget {
            pointer: Some("alpha".into()),
            stack: Some(StackState {
                enabled: true,
                members: vec!["alpha".into(), "beta".into()],
                keys: [("a".into(), "alpha".into()), ("b".into(), "beta".into())].into(),
            }),
            mode: Some(ModeState {
                mode: Mode::Proxy,
                attached: true,
                proxy_route: Some("alpha".into()),
                contract: None,
            }),
            ..Default::default()
        },
        || Ok(()),
    )
    .unwrap();
    drop(files);
    let policy = root.path().join("access.json");
    fs::write(&policy,json!({"principals":[{"id":"manager","token_sha256":hex::encode(Sha256::digest(TOKEN)),"grants":[{"platform_id":"platform","company_id":"company","host_id":"host","provider_config_read":true,"provider_config_write":true,"aggregation_read":[{"project_id":"project","app_id":"codex"}],"aggregation_write":[{"project_id":"project","app_id":"codex"}],"routing_read":true,"routing_manage":true,"routing_apps":[{"project_id":"project","app_id":"codex"}]}]}],"hosts":[{"id":"host","online":true,"binding":"local","home":root.path(),"credential_root":private,"search_paths":[],"applications":root.path().join("Applications")}]}).to_string()).unwrap();
    fs::set_permissions(&policy, fs::Permissions::from_mode(0o600)).unwrap();
    let listener = Arc::new(RoutingListener::default());
    listener.start(0).unwrap();
    let router = super::aggregation_read::router_with_policy(Some(policy.clone()))
        .merge(super::app_providers::codex_catalog::router_with_policy(Some(policy.clone())));
    let router = configure_with_policy(router, &listener, policy, db.clone())
        .await
        .layer(Extension(db))
        .layer(middleware::from_fn(crate::gateway::middleware::tenant_scope_middleware));
    let (status, state) = request(
        &router,
        "GET",
        "/agent-apps/codex/aggregation/state?targetHostId=host&projectId=project",
        Value::Null,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let revision = state["revision"].as_str().unwrap();
    let path =
        format!("/app-providers/codex/beta/config?projectId=project&expectedRevision={revision}");
    let models:Vec<_>=["model-new","model-extra"].into_iter().map(|slug|json!({"slug":slug,"display_name":slug,"description":"fixture","base_instructions":"fixture","supports_reasoning_summaries":false,"supports_parallel_tool_calls":false})).collect();
    let body = json!({"targetHostId":"host","profile":profile(&endpoint,"beta","model-new"),"catalog":{"models":models},"activation":"catalog_only"});
    assert_eq!(request(&router, "PUT", &path, body.clone()).await.0, StatusCode::OK);
    let config = fs::read_to_string(root.path().join(".codex/config.toml")).unwrap();
    let doc: toml::Table = config.parse().unwrap();
    assert_eq!(doc["model"].as_str(), Some("model-a"));
    let selected = doc["model_provider"].as_str().unwrap();
    let native = &doc["model_providers"][selected];
    let base = native["base_url"].as_str().unwrap();
    let token = native["experimental_bearer_token"].as_str().unwrap();
    assert_eq!(doc["model_catalog_json"].as_str(), Some("shiju-model-catalog.json"));
    assert!(config.contains("user comment"));
    assert_eq!(fs::read(root.path().join(".codex/auth.json")).unwrap(), auth);
    let catalog: Value = serde_json::from_slice(
        &fs::read(root.path().join(".codex/shiju-model-catalog.json")).unwrap(),
    )
    .unwrap();
    assert!(catalog["models"].as_array().unwrap().iter().any(|m| m["slug"] == "ccs-b/model-new"));
    assert!(catalog["models"].as_array().unwrap().iter().any(|m| m["slug"] == "ccs-b/model-extra"));
    assert!(!catalog["models"].as_array().unwrap().iter().any(|m| m["slug"] == "ccs-b/model-b"));
    let client = reqwest::Client::builder().no_proxy().build().unwrap();
    let response = client
        .post(format!("{base}/responses"))
        .bearer_auth(token)
        .json(&json!({"model":"ccs-b/model-new","input":[]}))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let (headers, body) = requests.try_recv().unwrap();
    assert_eq!(body["model"], "model-new");
    assert_eq!(headers["authorization"], "Bearer fixture-secret-for-beta");
    assert_eq!(request(&router,"PUT",&path,json!({"targetHostId":"host","profile":profile(&endpoint,"beta","late"),"catalog":null,"activation":"catalog_only"})).await.0,StatusCode::CONFLICT);
    assert_eq!(fs::read_to_string(root.path().join(".codex/config.toml")).unwrap(), config);
    assert!(tx.pending(AppId::Codex).unwrap().is_none());
    listener.stop().await.unwrap();
    upstream_task.abort();
    let _ = upstream_task.await;
}
