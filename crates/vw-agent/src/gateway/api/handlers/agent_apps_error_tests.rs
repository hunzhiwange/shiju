use super::agent_apps_error::AgentAppsApiError;
use axum::{body::to_bytes, response::IntoResponse};
use vw_api_types::agent_apps::AgentAppsErrorCode;

#[tokio::test]
async fn cc_base_icon_error_maps_all_codes() {
    for (code, status) in [
        (AgentAppsErrorCode::InvalidInput, 400),
        (AgentAppsErrorCode::NotFound, 404),
        (AgentAppsErrorCode::Conflict, 409),
        (AgentAppsErrorCode::StorageUnavailable, 503),
    ] {
        let response = AgentAppsApiError::from(code).into_response();
        assert_eq!(response.status().as_u16(), status);
        let body = to_bytes(response.into_body(), 4096).await.unwrap();
        let public: vw_api_types::agent_apps::AgentAppsErrorResponse =
            serde_json::from_slice(&body).unwrap();
        assert_eq!(public.code, code);
        assert!(!public.message.is_empty());
    }
}

#[derive(Clone, Default)]
struct Capture(std::sync::Arc<std::sync::Mutex<Vec<u8>>>);
impl tracing::Subscriber for Capture {
    fn max_level_hint(&self) -> Option<tracing::metadata::LevelFilter> {
        Some(tracing::metadata::LevelFilter::TRACE)
    }
    fn enabled(&self, _: &tracing::Metadata<'_>) -> bool {
        true
    }
    fn new_span(&self, _: &tracing::span::Attributes<'_>) -> tracing::span::Id {
        tracing::span::Id::from_u64(1)
    }
    fn record(&self, _: &tracing::span::Id, _: &tracing::span::Record<'_>) {}
    fn record_follows_from(&self, _: &tracing::span::Id, _: &tracing::span::Id) {}
    fn event(&self, event: &tracing::Event<'_>) {
        struct Visitor<'a>(&'a mut Vec<u8>);
        impl tracing::field::Visit for Visitor<'_> {
            fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
                use std::io::Write;
                write!(self.0, "{}={:?};", field.name(), value).unwrap();
            }
        }
        event.record(&mut Visitor(&mut self.0.lock().unwrap()));
    }
    fn enter(&self, _: &tracing::span::Id) {}
    fn exit(&self, _: &tracing::span::Id) {}
}

#[tokio::test]
async fn cc_base_icon_error_redacts_real_constraint_and_request() -> anyhow::Result<()> {
    use axum::{Router, body::Body, http::Request, routing::get};
    use tower::ServiceExt;
    let capture = Capture::default();
    let _guard = tracing::subscriber::set_default(capture.clone());
    let home = tempfile::Builder::new().prefix("SENTINEL_PATH").tempdir()?;
    let db = crate::global_database::GlobalDatabase::for_home(home.path())?;
    let connection = db.connect_sqlite()?;
    connection.execute_batch("CREATE TABLE secret(value TEXT CHECK(value != 'SENTINEL_DATA'));")?;
    let error = connection.execute("INSERT INTO secret VALUES ('SENTINEL_DATA')", []).unwrap_err();
    assert!(error.to_string().contains("SENTINEL_DATA"));
    let error = std::sync::Arc::new(std::sync::Mutex::new(Some(error)));
    let dispatch = tracing::Dispatch::new(capture.clone());
    let app = Router::new().route(
        "/{id}",
        get(move || {
            let error = error.lock().unwrap().take().unwrap();
            let dispatch = dispatch.clone();
            async move {
                tracing::dispatcher::with_default(&dispatch, || {
                    AgentAppsApiError::storage_unavailable(anyhow::Error::new(error))
                        .into_response()
                })
            }
        }),
    );
    let response = app
        .oneshot(
            Request::builder()
                .uri("/SENTINEL_PATH")
                .header("Authorization", "Bearer SENTINEL_AUTH")
                .body(Body::from("SENTINEL_PAYLOAD"))?,
        )
        .await?;
    assert_eq!(response.status().as_u16(), 503);
    let body = to_bytes(response.into_body(), 4096).await?;
    let logs = String::from_utf8(capture.0.lock().unwrap().clone())?;
    assert!(logs.contains("icon API request failed"), "{logs}");
    for sentinel in [
        "SENTINEL_DATA",
        "SENTINEL_PATH",
        "SENTINEL_AUTH",
        "SENTINEL_PAYLOAD",
        "INSERT INTO",
        "CHECK",
    ] {
        assert!(!logs.contains(sentinel), "{logs}");
        assert!(!String::from_utf8_lossy(&body).contains(sentinel));
    }
    Ok(())
}

#[tokio::test]
async fn cc_base_icon_error_business_boundaries() {
    use vw_agent_apps::icons::library::{add_icon, rename_icon};
    use vw_api_types::agent_apps::CustomIconEntry;
    let image = "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAAAAAA6fptVAAAACklEQVR4nGNgAAAAAgABSK+kcQAAAABJRU5ErkJggg==";
    let entries = vec![
        CustomIconEntry {
            id: "id".into(),
            name: "name".into(),
            data: "different".into(),
            created_at: 0
        };
        100
    ];
    let cases = [
        (add_icon(&[], "name", "SENTINEL_DATA", "id", 0).unwrap_err(), 400),
        (rename_icon(&[], "/SENTINEL_PATH", "name").unwrap_err(), 404),
        (add_icon(&entries, "name", image, "id", 0).unwrap_err(), 409),
    ];
    for (error, status) in cases {
        let response = AgentAppsApiError::from(error).into_response();
        assert_eq!(response.status().as_u16(), status);
        let body = to_bytes(response.into_body(), 4096).await.unwrap();
        assert!(!String::from_utf8_lossy(&body).contains("SENTINEL"));
    }
}
