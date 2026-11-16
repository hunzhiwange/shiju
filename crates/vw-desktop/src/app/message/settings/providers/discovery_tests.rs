#![cfg(not(target_arch = "wasm32"))]

use super::{
    catalog_api::Event,
    discovery::Event as Discovery,
    model_edits_api_tests::{Fixture, row},
};
use crate::app::{
    Message,
    message::{SettingsMessage, app_providers::list::ListMessage},
    state::app_providers::model_catalog::DiscoveryPhase,
};
use iced::{Task, futures::StreamExt};
use serde_json::{Value, json};
use std::{sync::mpsc, time::Duration};
use vw_api_model_catalog::ModelDiscoveryMethod;

const ID: &str = "fixture-task";
const GET: &str = "GET /v1/model-catalog/discoveries/fixture-task?";
const DELETE: &str = "DELETE /v1/model-catalog/discoveries/fixture-task?";

struct Run(mpsc::Receiver<Event>);

impl Run {
    fn new(fixture: &Fixture, task: Task<Message>) -> Self {
        let mut stream = iced_runtime::task::into_stream(task).expect("discovery task");
        let (tx, rx) = mpsc::channel();
        fixture.runtime.spawn(async move {
            while let Some(action) = stream.next().await {
                if let iced_runtime::Action::Output(Message::Settings(
                    SettingsMessage::AppProviders(ListMessage::ModelCatalog(event)),
                )) = action
                {
                    let _ = tx.send(event);
                }
            }
        });
        Self(rx)
    }

    fn next(&self) -> Event {
        self.0.recv_timeout(Duration::from_secs(5)).expect("discovery progress or result")
    }

    fn apply(&self, fixture: &mut Fixture) {
        assert_eq!(fixture.event(self.next()).units(), 0);
    }

    fn ended(self) {
        assert!(
            matches!(
                self.0.recv_timeout(Duration::from_secs(2)),
                Err(mpsc::RecvTimeoutError::Disconnected)
            ),
            "terminal discovery must release polling"
        );
    }
}

fn start(fixture: &mut Fixture) -> Run {
    let task = fixture.event(Event::Discovery(Discovery::Start(ModelDiscoveryMethod::Http)));
    assert!(task.units() > 0, "a saved deployment must be able to start discovery");
    let run = Run::new(fixture, task);
    let post = fixture.request("POST /v1/model-catalog/discoveries?").unwrap();
    assert_eq!(post, json!({"deployment":fixture.form().deployment(),"method":"http"}));
    run
}

fn status(fixture: &Fixture, state: Value) -> Value {
    json!({"taskId":ID,"deployment":fixture.form().deployment(),"state":state})
}

fn phase(fixture: &Fixture, phase: &str) {
    fixture.reply(status(fixture, json!({"phase":phase})));
}

fn complete(fixture: &Fixture, model: &str) -> Value {
    status(fixture, json!({"phase":"completed","models":[row(fixture, model, None)]}))
}



#[test]
fn catalog_desktop_discovery_host_change_releases_original_poll() {
    Fixture::with(|fixture| {
        let run = start(fixture);
        phase(fixture, "discovering");
        run.apply(fixture);
        fixture.request(GET);
        let late = complete(fixture, "Vendor/OldHost");
        let cancelled = status(fixture, json!({"phase":"cancelled"}));
        let _ = crate::app::message::settings::update(
            &mut fixture.app,
            SettingsMessage::AppsCatalog(
                crate::app::message::settings::AppsCatalogMessage::HostInput("new-mac".into()),
            ),
        );
        fixture.reply(late);
        fixture.request(DELETE);
        fixture.reply(cancelled);
        run.apply(fixture);
        run.ended();
        assert!(fixture.app.app_providers.model_catalog.is_none());
        assert!(fixture.app.toast_state.active_toast.is_none());
    });
}
