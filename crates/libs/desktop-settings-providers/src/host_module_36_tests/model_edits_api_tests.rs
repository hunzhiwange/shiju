#![cfg(not(target_arch = "wasm32"))]
use crate::catalog_api::{self, Event};
use desktop_provider_apps::{
    AgentAppsContext, Scope,
    state::model_catalog::{CatalogContext, CatalogForm},
};
use gateway_model_catalog::GatewayModelCatalogClient;
use iced::futures::StreamExt;
use serde_json::{Value, json};
use std::{
    sync::mpsc::{self, Receiver, Sender},
    time::Duration,
};
use vw_api_app_identity::AppId;
use vw_api_model_catalog::ModelCatalogProject;
const ENDPOINT: &str = "https://fixture.example/v1";
const MODEL: &str = "Vendor/Manual[1m]";
const TIMEOUT: Duration = Duration::from_secs(5);
pub(super) struct Fixture {
    pub(super) catalog: Option<CatalogForm>,
    pub(super) generation: u64,
    scope: Scope,
    pub(super) current: Result<(GatewayModelCatalogClient, CatalogContext), String>,
    pub(super) feedback: Option<(bool, String)>,
    pub(super) requests: Receiver<String>,
    pub(super) responses: Sender<Option<String>>,
    pub(super) runtime: tokio::runtime::Runtime,
}
impl Fixture {
    pub(super) fn with(test: impl FnOnce(&mut Self)) {
        super::server::with_gateway(|endpoint, requests, responses| {
            let scope = Scope {
                context: AgentAppsContext {
                    gateway: format!("http://127.0.0.1:{}", endpoint.port),
                    tenant: Default::default(),
                    project: Some("/fixture/project".into()),
                },
                host: "fixture-mac".into(),
                app: AppId::Codex,
            };
            let context = CatalogContext {
                scope: scope.clone(),
                project: ModelCatalogProject {
                    project_id: "model-edit-fixture".into(),
                    directory: "/fixture/project".into(),
                },
                access_generation: 0,
            };
            let mut form = CatalogForm::new(context.clone(), "fixture-provider".into());
            form.set_endpoint(ENDPOINT.into());
            let client = vw_gateway_client::GatewayClient::new(endpoint).unwrap();
            let current = Ok((GatewayModelCatalogClient::new(client.transport().clone()), context));
            let runtime = tokio::runtime::Builder::new_multi_thread()
                .worker_threads(1)
                .enable_all()
                .build()
                .unwrap();
            test(&mut Self {
                catalog: Some(form),
                generation: 0,
                scope,
                current,
                feedback: None,
                requests,
                responses,
                runtime,
            });
        });
    }
    pub(super) fn event(&mut self, event: Event) -> iced::Task<Event> {
        if let Event::Feedback { success, text } = event {
            self.feedback = Some((success, text));
            return iced::Task::none();
        }
        catalog_api::update(
            &mut self.catalog,
            &mut self.generation,
            Some(&self.scope),
            None,
            self.current.clone(),
            "fixture-token",
            event,
        )
    }
    pub(super) fn run(&self, task: iced::Task<Event>) -> tokio::task::JoinHandle<Event> {
        let mut stream = iced_runtime::task::into_stream(task).expect("HTTP task");
        self.runtime.spawn(async move {
            while let Some(action) = stream.next().await {
                if let iced_runtime::Action::Output(event) = action {
                    if matches!(event, Event::Feedback { success: true, .. }) {
                        continue;
                    }
                    return event;
                }
            }
            panic!("missing catalog result");
        })
    }
    pub(super) fn finish(&mut self, run: tokio::task::JoinHandle<Event>) -> iced::Task<Event> {
        let event = self
            .runtime
            .block_on(async { tokio::time::timeout(TIMEOUT, run).await })
            .unwrap()
            .unwrap();
        self.event(event)
    }
    pub(super) fn feedback(&mut self, task: iced::Task<Event>) {
        let mut stream = iced_runtime::task::into_stream(task).expect("feedback task");
        let action = self
            .runtime
            .block_on(async { tokio::time::timeout(TIMEOUT, stream.next()).await })
            .unwrap()
            .expect("feedback output");
        let iced_runtime::Action::Output(event @ Event::Feedback { .. }) = action else {
            panic!("expected feedback");
        };
        drop(self.event(event));
    }
    pub(super) fn request(&self, route: &str) -> Option<Value> {
        let request = self.requests.recv_timeout(TIMEOUT).expect("SDK request");
        assert!(request.starts_with(route), "{request}");
        assert!(request.contains("projectId=model-edit-fixture"));
        assert!(request.to_ascii_lowercase().contains("x-agent-apps-token: fixture-token"));
        let (_, body) = request.split_once("\r\n\r\n").unwrap();
        (!body.is_empty()).then(|| serde_json::from_str(body).unwrap())
    }
    pub(super) fn reply(&self, value: Value) {
        self.responses.send(Some(value.to_string())).unwrap();
    }
    pub(super) fn form(&self) -> &CatalogForm {
        self.catalog.as_ref().expect("draft still open")
    }
}
pub(super) fn row(fixture: &Fixture, id: &str, alias: Option<&str>) -> Value {
    json!({
        "identity": {
            "scope": fixture.form().context.scope.context.tenant,
            "deployment": fixture.form().deployment(),
            "modelId": id
        },
        "displayAlias": alias,
        "metadata": {"contextWindow": 8192}
    })
}

pub(super) fn load(fixture: &mut Fixture, response: Value) {
    let task = fixture.event(Event::LoadCatalog);
    let load = fixture.run(task);
    assert!(fixture.request("GET /v1/model-catalog?").is_none());
    fixture.reply(response);
    let _ = fixture.finish(load);
}

pub(super) fn select(fixture: &mut Fixture, model_id: &str) {
    let _ = fixture
        .event(Event::EditModel { generation: fixture.generation, model_id: model_id.into() });
}

fn finish_save_and_refresh(
    fixture: &mut Fixture,
    pending: tokio::task::JoinHandle<Event>,
    response: Value,
) {
    let revision = response["revision"].clone();
    fixture.reply(json!({"revision":revision,"changed":true}));
    let follow_up = fixture.finish(pending);
    let refresh = fixture.run(follow_up);
    assert!(fixture.request("GET /v1/model-catalog?").is_none());
    fixture.reply(response);
    let _ = fixture.finish(refresh);
    assert!(fixture.requests.try_recv().is_err(), "no desktop settings write");
}

#[test]
fn catalog_desktop_edits_saves_unknown_model_alias_and_default_together() {
    Fixture::with(|fixture| {
        let _ = fixture.event(Event::ManualId(MODEL.into()));
        let _ = fixture.event(Event::Alias("中文别名".into()));
        let _ = fixture.event(Event::MakeDefault(true));
        let task = fixture.event(Event::Save);
        let pending = fixture.run(task);
        assert!(fixture.request("GET /v1/model-catalog?").is_none());
        fixture.reply(json!({"revision":"original","items":[]}));
        let write = fixture.request("POST /v1/model-catalog/edits?").unwrap();
        assert_eq!(write["deployment"]["endpoint"], ENDPOINT);
        assert_eq!(write["expectedRevision"], "original");
        assert_eq!(write["edits"][0]["modelId"], MODEL);
        assert_eq!(write["edits"][0]["displayAlias"], "中文别名");
        assert_eq!(write["defaultChange"], json!({"kind":"set","modelId":MODEL}));

        let response = json!({
            "revision":"saved",
            "items":[row(fixture, MODEL, Some("中文别名"))],
            "defaultModelId":MODEL
        });
        finish_save_and_refresh(fixture, pending, response);
        assert!(fixture.form().manual_id.is_empty());
        assert!(fixture.form().alias.is_empty());
        let catalog = fixture.form().catalog.as_ref().unwrap();
        assert_eq!(catalog.revision, "saved");
        assert_eq!(catalog.default_model_id.as_ref().unwrap().as_ref(), MODEL);
        assert_eq!(catalog.items[0].identity.model_id.as_ref(), MODEL);
        assert_eq!(catalog.items[0].display_alias.as_deref(), Some("中文别名"));
    });
}

#[test]
fn catalog_desktop_edits_clears_alias_using_displayed_revision_without_rewriting_metadata() {
    Fixture::with(|fixture| {
        let existing = row(fixture, MODEL, Some("旧别名"));
        load(fixture, json!({"revision":"displayed","items":[existing],"defaultModelId":MODEL}));
        select(fixture, MODEL);
        assert_eq!(fixture.form().alias, "旧别名");
        assert!(fixture.form().make_default());
        let _ = fixture.event(Event::Alias(String::new()));
        let task = fixture.event(Event::Save);
        let pending = fixture.run(task);
        let write = fixture.request("POST /v1/model-catalog/edits?").unwrap();
        assert_eq!(write["expectedRevision"], "displayed");
        assert_eq!(write["edits"][0]["displayAlias"], "");
        assert_eq!(write["edits"][0]["modelId"], MODEL);
        assert_eq!(write["edits"][0]["metadata"]["contextWindow"], Value::Null);
        assert_eq!(write["defaultChange"], Value::Null);
        let saved = row(fixture, MODEL, None);
        finish_save_and_refresh(
            fixture,
            pending,
            json!({"revision":"saved","items":[saved],"defaultModelId":MODEL}),
        );
        assert_eq!(
            fixture.form().catalog.as_ref().unwrap().items[0].metadata.context_window,
            Some(8192)
        );
    });
}

#[test]
fn catalog_desktop_edits_sets_and_clears_default_through_defaults_sdk() {
    Fixture::with(|fixture| {
        let existing = row(fixture, MODEL, Some("保留的别名"));
        load(fixture, json!({"revision":"original","items":[existing]}));
        for (revision, checked, change, result_revision) in [
            ("original", true, json!({"kind":"set","modelId":MODEL}), "set"),
            ("set", false, json!({"kind":"clear"}), "clear"),
        ] {
            select(fixture, MODEL);
            assert_eq!(fixture.form().make_default(), !checked);
            let _ = fixture.event(Event::MakeDefault(checked));
            let task = fixture.event(Event::Save);
            let pending = fixture.run(task);
            let write = fixture.request("POST /v1/model-catalog/defaults?").unwrap();
            assert_eq!(write["expectedRevision"], revision);
            assert_eq!(write["change"], change);
            let saved = row(fixture, MODEL, Some("保留的别名"));
            finish_save_and_refresh(
                fixture,
                pending,
                json!({"revision":result_revision,"items":[saved],"defaultModelId":checked.then_some(MODEL)}),
            );
        }
        let catalog = fixture.form().catalog.as_ref().unwrap();
        assert!(catalog.default_model_id.is_none());
        assert_eq!(catalog.items[0].display_alias.as_deref(), Some("保留的别名"));
    });
}

#[test]
fn catalog_desktop_edits_refresh_keeps_only_fields_the_user_edited() {
    Fixture::with(|fixture| {
        let existing = row(fixture, MODEL, Some("旧别名"));
        load(fixture, json!({"revision":"original","items":[existing]}));
        select(fixture, MODEL);
        let _ = fixture.event(Event::Alias("我的草稿".into()));
        let existing = row(fixture, MODEL, Some("远端别名"));
        load(fixture, json!({"revision":"newer","items":[existing],"defaultModelId":MODEL}));
        assert_eq!(fixture.form().alias, "我的草稿");
        assert!(fixture.form().make_default());
        let task = fixture.event(Event::Save);
        let pending = fixture.run(task);
        let write = fixture.request("POST /v1/model-catalog/edits?").unwrap();
        assert_eq!(write["expectedRevision"], "newer");
        assert_eq!(write["edits"][0]["displayAlias"], "我的草稿");
        assert_eq!(write["defaultChange"], Value::Null);
        let saved = row(fixture, MODEL, Some("我的草稿"));
        finish_save_and_refresh(
            fixture,
            pending,
            json!({"revision":"saved","items":[saved],"defaultModelId":MODEL}),
        );
    });
}

#[test]
fn catalog_desktop_edits_switching_endpoints_discards_previous_model_and_revision() {
    Fixture::with(|fixture| {
        let existing = row(fixture, MODEL, Some("旧部署别名"));
        load(fixture, json!({"revision":"original","items":[existing]}));
        select(fixture, MODEL);
        let _ = fixture.event(Event::Alias("未保存".into()));
        let _ = fixture.event(Event::Endpoint("https://another.example/v1".into()));
        assert!(fixture.form().edit_id.is_none());
        assert!(fixture.form().alias.is_empty());
        assert!(fixture.form().catalog.is_none());
        assert_eq!(fixture.event(Event::Save).units(), 0);
        assert!(fixture.requests.try_recv().is_err());
    });
}
