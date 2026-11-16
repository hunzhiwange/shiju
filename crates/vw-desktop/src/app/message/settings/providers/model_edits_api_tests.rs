#![cfg(not(target_arch = "wasm32"))]

use super::catalog_api::{self, Event};
use crate::app::{
    App, Message,
    message::{
        SettingsMessage,
        app_providers::list::{self, ListMessage},
    },
    state::{
        ToastKind,
        app_providers::model_catalog::{CatalogContext, CatalogForm},
    },
};
use iced::futures::StreamExt;
use serde_json::{Value, json};
use std::{
    sync::mpsc::{Receiver, Sender},
    time::Duration,
};
use vw_api_model_catalog::ModelCatalogProject;
use vw_api_types::agent_apps::AppId;

const ENDPOINT: &str = "https://fixture.example/v1";
const MODEL: &str = "Vendor/Manual[1m]";
const TIMEOUT: Duration = Duration::from_secs(5);

pub(super) struct Fixture {
    pub(super) app: App,
    pub(super) requests: Receiver<String>,
    pub(super) responses: Sender<Option<String>>,
    pub(super) runtime: tokio::runtime::Runtime,
}

impl Fixture {
    pub(super) fn with(test: impl FnOnce(&mut Self)) {
        crate::app::config::with_editor_settings_gateway(|mut app, requests, responses| {
            app.project_id = Some("model-edit-fixture".into());
            app.navigation_tabs.project_path = Some("/fixture/project".into());
            app.apps_catalog.host = "fixture-mac".into();
            app.apps_catalog.token = "fixture-token".into();
            let (_, scope) = list::current(&app, AppId::Codex).unwrap();
            let mut form = CatalogForm::new(
                CatalogContext {
                    scope: scope.clone(),
                    project: ModelCatalogProject {
                        project_id: "model-edit-fixture".into(),
                        directory: "/fixture/project".into(),
                    },
                    access_generation: app.apps_catalog.generation,
                },
                "fixture-provider".into(),
            );
            form.set_endpoint(ENDPOINT.into());
            app.app_providers.scope = Some(scope);
            app.app_providers.model_catalog = Some(form);
            let runtime = tokio::runtime::Builder::new_multi_thread()
                .worker_threads(1)
                .enable_all()
                .build()
                .unwrap();
            test(&mut Self { app, requests, responses, runtime });
        });
    }

    pub(super) fn event(&mut self, event: Event) -> iced::Task<Message> {
        catalog_api::update(&mut self.app, event)
    }

    pub(super) fn run(&self, task: iced::Task<Message>) -> tokio::task::JoinHandle<Event> {
        let mut stream = iced_runtime::task::into_stream(task).expect("HTTP task");
        self.runtime.spawn(async move {
            while let Some(action) = stream.next().await {
                if let iced_runtime::Action::Output(Message::Settings(
                    SettingsMessage::AppProviders(ListMessage::ModelCatalog(event)),
                )) = action
                {
                    return event;
                }
            }
            panic!("missing catalog result");
        })
    }

    pub(super) fn finish(&mut self, run: tokio::task::JoinHandle<Event>) -> iced::Task<Message> {
        let event = self
            .runtime
            .block_on(async { tokio::time::timeout(TIMEOUT, run).await })
            .unwrap()
            .unwrap();
        self.event(event)
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
        self.app.app_providers.model_catalog.as_ref().expect("draft still open")
    }
}

#[test]
fn catalog_desktop_edits_retains_draft_after_save_error() {
    Fixture::with(|fixture| {
        let _ = fixture.event(Event::ManualId(MODEL.into()));
        let task = fixture.event(Event::Save);
        let save = fixture.run(task);
        assert!(fixture.request("GET /v1/model-catalog?").is_none());
        fixture.reply(json!({"revision":"original", "items":[]}));
        let write = fixture.request("POST /v1/model-catalog/edits?").unwrap();
        assert_eq!(write["edits"][0]["modelId"], MODEL);
        fixture.responses.send(None).unwrap();
        let feedback = fixture.finish(save);
        let notice = fixture.run(feedback);
        let _ = fixture.finish(notice);
        assert_eq!(fixture.form().manual_id, MODEL);
        assert!(!fixture.form().saving);
        let toast = fixture.app.active_toast.as_ref().expect("SDK save error must show a toast");
        assert!(matches!(toast.kind, ToastKind::Error));
        assert!(fixture.requests.try_recv().is_err(), "no desktop settings write");
    });
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
