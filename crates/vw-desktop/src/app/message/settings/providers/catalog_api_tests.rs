use crate::app::state::{
    AgentAppsContext,
    app_providers::{
        list::Scope,
        model_catalog::{CatalogContext, CatalogForm},
    },
};
use vw_gateway_client::vw_api_types::{
    agent_apps::AppId,
    id::TenantScope,
    model_catalog::{ModelCatalogPreset, ModelCatalogProject},
};

fn form() -> CatalogForm {
    let directory = "/fixture/project".to_owned();
    CatalogForm::new(
        CatalogContext {
            scope: Scope {
                context: AgentAppsContext {
                    gateway: "http://127.0.0.1:8080".into(),
                    tenant: TenantScope::default(),
                    project: Some(directory.clone()),
                },
                host: "current-host".into(),
                app: AppId::Codex,
            },
            project: ModelCatalogProject { project_id: "fixture".into(), directory },
            access_generation: 3,
        },
        "saved-provider".into(),
    )
}

fn preset(app_id: AppId, endpoint: &str) -> ModelCatalogPreset {
    ModelCatalogPreset {
        app_id,
        key: "fixture".into(),
        name: "同名部署".into(),
        name_key: None,
        website_url: None,
        family: Some("fixture-family".into()),
        endpoint: Some(endpoint.into()),
        model_ids: vec!["Ns/Model[1m]".into(), "Other".into()],
    }
}

#[test]
fn catalog_desktop_search_ignores_old_search_response() {
    let mut form = form();
    let old = form.context.clone();
    form.context.scope.host = "another-host".into();
    let current = form.context.clone();
    let items = vec![preset(AppId::Codex, "https://current.example/v1")];
    assert!(form.finish_search(&current, items.clone()));
    form.set_endpoint("https://current.example/v1".into());
    form.toggle(0, 0, true);
    assert_eq!(form.selected.len(), 1);
    let selection = form.selected.clone();
    assert!(!form.finish_search(&old, vec![preset(AppId::Codex, "https://old.example/v1")]));
    assert_eq!(form.items, items);
    assert_eq!(form.selected, selection);
}

#[test]
fn catalog_desktop_search_keeps_exact_model_and_deployment() {
    let mut form = form();
    let context = form.context.clone();
    form.finish_search(
        &context,
        vec![
            preset(AppId::Codex, "https://example.test/cn"),
            preset(AppId::Codex, "https://example.test/global"),
        ],
    );
    form.set_endpoint("https://example.test/global".into());
    form.toggle(0, 0, true);
    assert!(form.selected.is_empty());
    form.toggle(1, 0, true);
    form.toggle(1, 1, true);
    assert_eq!(form.selected.len(), 2);
    let model = &form.selected[0];
    assert_eq!(model.model_id.as_ref(), "Ns/Model[1m]");
    assert_eq!(model.deployment.endpoint, "https://example.test/global");
    assert_eq!(model.deployment.provider_id, "saved-provider");
    assert_eq!(model.deployment.target_host_id, "current-host");
}

#[test]
fn catalog_desktop_search_rejects_cross_application_rows() {
    let mut form = form();
    let context = form.context.clone();
    assert!(form.finish_search(&context, vec![preset(AppId::Claude, "https://wrong.example/v1")]));
    assert!(form.items.is_empty());
}

#[test]
fn catalog_desktop_search_distinguishes_empty_catalog_from_no_match() {
    let mut form = form();
    assert_eq!(form.empty_label(), "目录为空");
    form.query = "中文搜索".into();
    assert_eq!(form.empty_label(), "未找到匹配的预设");
}
