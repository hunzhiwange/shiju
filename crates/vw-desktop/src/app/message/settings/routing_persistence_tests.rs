use super::super::messages::QueryClassificationMessage;
use super::super::{model_routes, query_classification};
use super::*;
use crate::app::state::ModelRoute;
use iced_runtime::futures::futures::StreamExt;
use serde_json::json;
use std::time::Duration;

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn routing_refresh_and_shutdown_preserve_newer_edits_after_failed_save() {
    const TIMEOUT: Duration = Duration::from_secs(4);
    fn run(
        runtime: &tokio::runtime::Runtime,
        task: Task<Message>,
    ) -> tokio::task::JoinHandle<Message> {
        let mut stream = iced_runtime::task::into_stream(task).unwrap();
        runtime.spawn(async move {
            while let Some(action) = stream.next().await {
                if let iced_runtime::Action::Output(message) = action {
                    return message;
                }
            }
            panic!("missing result");
        })
    }
    fn finish(
        runtime: &tokio::runtime::Runtime,
        task: tokio::task::JoinHandle<Message>,
        app: &mut App,
    ) -> Task<Message> {
        let message =
            runtime.block_on(async { tokio::time::timeout(TIMEOUT, task).await }).unwrap().unwrap();
        let Message::Settings(message) = message else {
            panic!("settings result");
        };
        model_routes::update(app, message)
    }
    crate::app::config::with_editor_settings_gateway(|mut app, requests, responses| {
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(1)
            .enable_all()
            .build()
            .unwrap();
        let load = run(&runtime, refresh(&mut app));
        assert!(requests.recv_timeout(TIMEOUT).unwrap().starts_with("GET /v1/global/config "));
        // A newer draft arriving while refresh waits must survive the stale GET result.
        app.model_routes_settings.routes = vec![ModelRoute {
            pattern: "code".into(),
            provider: "local".into(),
            model: "new".into(),
            priority_input: "1".into(),
        }];
        let _ = model_routes::update(
            &mut app,
            SettingsMessage::ModelRoutes(ModelRoutesMessage::PriorityChanged(0, "2".into())),
        );
        responses.send(Some(json!({"model_routes": [], "query_classification": {"enabled": false, "rules": []}}).to_string())).unwrap();
        let _ = finish(&runtime, load, &mut app);
        assert_eq!(app.model_routes_settings.routes[0].model, "new");

        // Shutdown flushes pending snapshots without waiting for the debounce timer.
        let (flush, mut idle) = app.model_routes_settings.persistence.flush();
        let first = run(&runtime, flush);
        assert!(!*idle.as_mut().unwrap().borrow_and_update());
        let first_request = requests.recv_timeout(TIMEOUT).unwrap();
        assert!(first_request.starts_with("PATCH /v1/global/config "));
        let _ = query_classification::update(
            &mut app,
            SettingsMessage::QueryClassification(QueryClassificationMessage::CategoryChanged(
                0,
                "last".into(),
            )),
        );
        let revision = app.model_routes_settings.persistence.revision;
        let _ = save_due(&mut app, revision);
        responses.send(None).unwrap();
        let failed = finish(&runtime, first, &mut app);
        // The failure task batches a toast timeout with the next save; collect its save output.
        let mut actions = iced_runtime::task::into_stream(failed).unwrap();
        let second = runtime.spawn(async move {
            while let Some(action) = actions.next().await {
                if let iced_runtime::Action::Output(message @ Message::Settings(_)) = action {
                    return message;
                }
            }
            panic!("missing save completion");
        });
        let request = requests.recv_timeout(TIMEOUT).unwrap();
        let patch: Value = serde_json::from_str(request.split_once("\r\n\r\n").unwrap().1).unwrap();
        assert_eq!(patch["model_routes"][0]["model"], "new");
        assert_eq!(patch["app_ui"]["system_settings"]["model_routes"][0]["priority"], 2);
        assert_eq!(patch["query_classification"]["rules"][0]["hint"], "last");
        assert!(!*idle.as_mut().unwrap().borrow_and_update());
        responses.send(Some("{}".into())).unwrap();
        let _ = finish(&runtime, second, &mut app);
        assert!(*idle.as_mut().unwrap().borrow_and_update());
        assert!(app.model_routes_settings.save_error.is_none());
        assert!(app.query_classification_settings.save_error.is_none());
        assert!(!app.model_routes_settings.persistence.dirty);
        assert!(requests.try_recv().is_err());
    });
}
