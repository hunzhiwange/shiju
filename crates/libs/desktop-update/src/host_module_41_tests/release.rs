use super::*;
// Update sources and checked release snapshots; no production network access.

use desktop_release::{DesktopRelease, UpdateSource};

#[test]
fn desktop_integration_47_preserves_expected_behavior() {
    let mut app = Fixture::new();
    app.desktop_update = DesktopUpdate::from_config(&serde_json::json!({}));
    let _ = app.update(ViewMessage::CheckAppUpdate);
    let request = app.desktop_update.request;
    let _ = app.update(ViewMessage::CheckAppUpdateFinished(
        request,
        UpdateSource::Shiju,
        Ok(DesktopRelease {
            source: UpdateSource::Github,
            version: "99.0.0".into(),
            assets: vec![],
        }),
    ));
    assert!(app.desktop_update.release.is_none());
    assert!(!app.desktop_update.installable());
}

#[cfg(not(target_arch = "wasm32"))]
#[tokio::test(flavor = "multi_thread")]
async fn desktop_integration_47_download_uses_current_snapshot() {
    use iced_runtime::futures::futures::StreamExt;
    for source in [UpdateSource::Shiju, UpdateSource::Github] {
        let mut app = Fixture::new();
        app.desktop_update =
            DesktopUpdate::from_config(&crate::app::desktop_update::source_patch(source));
        // Empty assets fail before cache creation or HTTP: refetching latest would lose this error.
        let snapshot = DesktopRelease { source, version: "99.0.0".into(), assets: vec![] };
        app.desktop_update.release = Some(snapshot.clone());
        let task = app.update(ViewMessage::RunAppUpdate);
        assert!(app.desktop_update.busy);
        assert_eq!(app.desktop_update.release.as_ref(), Some(&snapshot));
        let mut stream = iced_runtime::task::into_stream(task).expect("download task");
        let action = tokio::time::timeout(std::time::Duration::from_secs(5), stream.next())
            .await
            .expect("snapshot validation timeout")
            .expect("download result");
        match action {
            iced_runtime::Action::Output(ViewMessage::AppUpdateDownloadPrepared(_, Err(error))) => {
                assert!(error.contains("No desktop installers available"), "{error}");
            }
            _ => panic!("expected checked snapshot validation failure"),
        }
    }
}
