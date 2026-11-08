#![cfg(not(target_arch = "wasm32"))]

use super::{
    catalog_api::Event,
    discovery::Event as Discovery,
    model_edits_api_tests::{Fixture, load, row},
};
use crate::app::{
    Message,
    message::{SettingsMessage, app_providers::list::ListMessage},
    state::app_providers::model_catalog::DiscoveryPhase,
};
use iced::{Task, futures::StreamExt};
use serde_json::{Value, json};
use std::{sync::mpsc, time::Duration};
use vw_gateway_client::vw_api_types::model_catalog::ModelDiscoveryMethod;

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

fn cancel(fixture: &mut Fixture) {
    assert_eq!(fixture.event(Event::Discovery(Discovery::Cancel)).units(), 0);
}

#[test]
fn catalog_desktop_discovery_cancel_keeps_previous_catalog() {
    Fixture::with(|fixture| {
        let existing = row(fixture, "Vendor/Saved", Some("已保存模型"));
        load(fixture, json!({"revision":"previous","items":[existing]}));
        let previous = fixture.form().catalog.clone();
        let run = start(fixture);
        phase(fixture, "queued");
        run.apply(fixture);
        assert!(matches!(fixture.form().discovery.phase, DiscoveryPhase::Queued));
        assert!(fixture.request(GET).is_none());

        cancel(fixture);
        assert!(matches!(fixture.form().discovery.phase, DiscoveryPhase::Stopping));
        // The upstream GET and even the DELETE can report a completed task
        // when discovery finished just as the user clicked cancel.
        let late = complete(fixture, "Vendor/Late");
        fixture.reply(late.clone());
        assert!(fixture.request(DELETE).is_none());
        fixture.reply(late);
        run.apply(fixture);
        assert!(matches!(fixture.form().discovery.phase, DiscoveryPhase::Cancelled));
        assert!(fixture.form().discovery.results.is_empty());
        assert_eq!(fixture.form().catalog, previous);
        run.ended();
        assert!(fixture.requests.try_recv().is_err(), "no poll or catalog write after cancel");
    });
}

#[test]
fn catalog_desktop_discovery_displays_real_phases_and_keeps_results_as_draft() {
    Fixture::with(|fixture| {
        let run = start(fixture);
        phase(fixture, "queued");
        run.apply(fixture);
        assert!(matches!(fixture.form().discovery.phase, DiscoveryPhase::Queued));
        fixture.request(GET);
        phase(fixture, "discovering");
        run.apply(fixture);
        assert!(matches!(fixture.form().discovery.phase, DiscoveryPhase::Discovering));
        fixture.request(GET);
        phase(fixture, "parsing");
        run.apply(fixture);
        assert!(matches!(fixture.form().discovery.phase, DiscoveryPhase::Parsing));
        fixture.request(GET);
        fixture.reply(complete(fixture, "Vendor/发现[1m]"));
        run.apply(fixture);
        run.ended();

        let discovery = &fixture.form().discovery;
        assert!(matches!(discovery.phase, DiscoveryPhase::Completed(_)));
        assert_eq!(discovery.results[0].model_id.as_ref(), "Vendor/发现[1m]");
        assert!(fixture.form().catalog.is_none());
        let generation = discovery.results_run;
        let _ = fixture.event(Event::Discovery(Discovery::Toggle {
            run: generation,
            index: 0,
            checked: true,
        }));
        assert_eq!(fixture.form().selected[0].model_id.as_ref(), "Vendor/发现[1m]");
        assert!(fixture.requests.try_recv().is_err(), "finding and selecting are read only");
    });
}

#[test]
fn catalog_desktop_discovery_closing_before_start_reply_cancels_original_task() {
    Fixture::with(|fixture| {
        let run = start(fixture);
        let accepted = status(fixture, json!({"phase":"queued"}));
        let stopped = status(fixture, json!({"phase":"cancelled"}));
        let _ = fixture.event(Event::Close);
        fixture.reply(accepted);
        fixture.request(DELETE);
        fixture.reply(stopped);
        run.apply(fixture);
        run.ended();
        assert!(fixture.app.app_providers.model_catalog.is_none());
        assert!(fixture.app.active_toast.is_none());
    });
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
        assert!(fixture.app.active_toast.is_none());
    });
}

#[test]
fn catalog_desktop_discovery_retry_ignores_a_previous_runs_completion() {
    Fixture::with(|fixture| {
        let run = start(fixture);
        fixture.reply(complete(fixture, "Vendor/Old"));
        let stale = run.next();
        run.ended();
        // Completion reached the desktop but was still queued when the user cancelled.
        cancel(fixture);
        assert!(matches!(fixture.form().discovery.phase, DiscoveryPhase::Cancelled));

        let retry = start(fixture);
        let _ = fixture.event(stale);
        assert!(matches!(fixture.form().discovery.phase, DiscoveryPhase::Starting));
        assert!(fixture.form().discovery.results.is_empty());
        fixture.reply(complete(fixture, "Vendor/Retry"));
        retry.apply(fixture);
        retry.ended();
        assert_eq!(fixture.form().discovery.results[0].model_id.as_ref(), "Vendor/Retry");
    });
}

#[test]
fn catalog_desktop_discovery_rejects_cross_deployment_result_and_retries_after_failure() {
    Fixture::with(|fixture| {
        let run = start(fixture);
        let mut wrong = complete(fixture, "Vendor/Wrong");
        wrong["state"]["models"][0]["identity"]["deployment"]["targetHostId"] = json!("wrong");
        fixture.reply(wrong);
        run.apply(fixture);
        run.ended();
        assert!(matches!(fixture.form().discovery.phase, DiscoveryPhase::Failed(_)));
        assert!(fixture.form().discovery.results.is_empty());

        let run = start(fixture);
        fixture.reply(status(fixture, json!({"phase":"failed","reason":"timed_out"})));
        run.apply(fixture);
        run.ended();
        assert!(matches!(fixture.form().discovery.phase, DiscoveryPhase::ServiceFailed(_)));
    });
}

#[test]
fn catalog_desktop_discovery_unconfirmed_cancel_releases_poll_and_preserves_saved_models() {
    Fixture::with(|fixture| {
        let existing = row(fixture, "Vendor/Saved", None);
        load(fixture, json!({"revision":"previous","items":[existing]}));
        let run = start(fixture);
        phase(fixture, "queued");
        run.apply(fixture);
        cancel(fixture);
        fixture.request(DELETE);
        fixture.responses.send(None).unwrap();
        run.apply(fixture);
        run.ended();
        assert!(matches!(fixture.form().discovery.phase, DiscoveryPhase::CancelUnconfirmed));
        assert_eq!(fixture.form().catalog.as_ref().unwrap().revision, "previous");
    });
}
