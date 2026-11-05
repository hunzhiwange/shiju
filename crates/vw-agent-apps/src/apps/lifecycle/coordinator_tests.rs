use super::coordinator::ToolLifecycleCoordinator;
use std::sync::{Arc, mpsc};
use std::time::Duration;
use vw_api_types::agent_apps::AppId;

// A real child holds the write interval until the fixture explicitly releases it.
#[cfg(unix)]
#[tokio::test]
async fn cc_apps_lifecycle_lock_survives_disconnect() {
    let temp = tempfile::tempdir().unwrap();
    let started = temp.path().join("started");
    let release = temp.path().join("release");
    let finished = temp.path().join("finished");
    let child_started = started.clone();
    let child_release = release.clone();
    let child_finished = finished.clone();
    let first = tokio::spawn(async move {
        ToolLifecycleCoordinator
            .run(vec![AppId::Claude], move || {
                let status = std::process::Command::new("/bin/sh")
                    .arg("-c")
                    .arg(r#"touch "$1"; i=0; while [ ! -f "$2" ]; do sleep 0.02; i=$((i+1)); [ "$i" -lt 500 ] || exit 70; done; touch "$3""#)
                    .arg("fixture")
                    .arg(child_started)
                    .arg(child_release)
                    .arg(child_finished)
                    .status()
                    .map_err(|e| e.to_string())?;
                if status.success() { Ok(()) } else { Err("child failed".into()) }
            })
            .await
    });
    // Keep a bounded emergency release even if an assertion fails.
    let emergency_release = release.clone();
    let (stop, stopped) = mpsc::channel();
    let watchdog = std::thread::spawn(move || {
        let _ = stopped.recv_timeout(Duration::from_secs(10));
        let _ = std::fs::write(emergency_release, b"");
    });
    tokio::time::timeout(Duration::from_secs(5), async {
        while !started.exists() {
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    })
    .await
    .unwrap();
    first.abort();
    assert!(first.await.unwrap_err().is_cancelled());
    let called = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let duplicate_called = called.clone();
    let duplicate = ToolLifecycleCoordinator
        .run(vec![AppId::Claude], move || {
            duplicate_called.store(true, std::sync::atomic::Ordering::SeqCst);
            Ok(())
        })
        .await;
    assert_eq!(duplicate.unwrap_err(), "TOOL_ACTION_IN_PROGRESS");
    assert!(!called.load(std::sync::atomic::Ordering::SeqCst));
    // A separate caller (including another tenant) uses the same host lock.
    let second_started = temp.path().join("second");
    let marker = second_started.clone();
    let completion = finished.clone();
    let second = tokio::spawn(async move {
        ToolLifecycleCoordinator
            .run(vec![AppId::Codex], move || {
                assert!(completion.exists(), "overlapping host writes");
                std::fs::write(marker, b"second").unwrap();
                Ok(42)
            })
            .await
    });
    // Poll a queued submission once, then disconnect before execution starts.
    let queued_marker = temp.path().join("queued");
    let marker = queued_marker.clone();
    let mut queued = Box::pin(ToolLifecycleCoordinator.run(vec![AppId::Gemini], move || {
        std::fs::write(marker, b"unexpected").unwrap();
        Ok(())
    }));
    std::future::poll_fn(|cx| {
        use std::future::Future;
        assert!(queued.as_mut().poll(cx).is_pending());
        std::task::Poll::Ready(())
    })
    .await;
    assert_eq!(
        ToolLifecycleCoordinator.run(vec![AppId::Gemini], || Ok(())).await.unwrap_err(),
        "TOOL_ACTION_IN_PROGRESS"
    );
    drop(queued);
    tokio::time::sleep(Duration::from_millis(50)).await;
    assert!(!second_started.exists());
    std::fs::write(&release, b"").unwrap();
    assert_eq!(second.await.unwrap().unwrap(), 42);
    assert!(finished.exists() && second_started.exists());
    assert!(!queued_marker.exists());
    ToolLifecycleCoordinator.run(vec![AppId::Gemini], || Ok(())).await.unwrap();
    stop.send(()).unwrap();
    watchdog.join().unwrap();
    // Errors and panics release both reservations; neither becomes success.
    let failure = ToolLifecycleCoordinator
        .run(vec![AppId::Claude], || {
            let status = std::process::Command::new("/bin/sh")
                .args(["-c", "exit 17"])
                .status()
                .map_err(|e| e.to_string())?;
            assert_eq!(status.code(), Some(17));
            Err::<(), _>("fixture failure".into())
        })
        .await;
    assert_eq!(failure.unwrap_err(), "fixture failure");
    assert!(
        ToolLifecycleCoordinator
            .run(vec![AppId::Claude], || -> Result<(), String> { panic!("fixture panic") })
            .await
            .unwrap_err()
            .contains("join")
    );
    assert_eq!(ToolLifecycleCoordinator.run(vec![AppId::Claude], || Ok(7)).await.unwrap(), 7);
    assert!(ToolLifecycleCoordinator.run(Vec::new(), || Ok(())).await.is_err());
    assert!(
        ToolLifecycleCoordinator.run(vec![AppId::Claude, AppId::Claude], || Ok(())).await.is_err()
    );
    assert!(
        ToolLifecycleCoordinator
            .run(vec![AppId::Claude, AppId::ClaudeDesktop], || Ok(()))
            .await
            .is_err()
    );
    ToolLifecycleCoordinator.run(vec![AppId::Claude], || Ok(())).await.unwrap();
}
