use super::{
    active_runs::{register, snapshot},
    add_job,
};
use crate::app::agent::config::Config;
use chrono::Utc;
use tempfile::TempDir;

fn config(tmp: &TempDir) -> Config {
    Config { workspace_dir: tmp.path().to_path_buf(), ..Config::default() }
}

#[test]
fn cron_history_active_runs_are_workspace_scoped() {
    let a = TempDir::new().unwrap();
    let b = TempDir::new().unwrap();
    let cfg = config(&a);
    let job = add_job(&cfg, "*/5 * * * *", "echo private").unwrap();
    let started = Utc::now();
    let guard = register(&cfg, &job, started);
    let runs = snapshot(&cfg);
    assert_eq!(runs.len(), 1);
    assert_eq!(runs[0].started_at, started);
    assert_eq!(runs[0].job_id, job.id);
    assert_eq!(runs[0].name, job.name);
    assert!(snapshot(&config(&b)).is_empty());
    let alias = Config { workspace_dir: a.path().join("."), ..Config::default() };
    assert_eq!(snapshot(&alias), runs);
    #[cfg(unix)]
    {
        let link = b.path().join("workspace-link");
        std::os::unix::fs::symlink(a.path(), &link).unwrap();
        let alias = Config { workspace_dir: link, ..Config::default() };
        assert_eq!(snapshot(&alias), runs);
    }
    drop(guard);
    assert!(snapshot(&cfg).is_empty());
}

#[test]
fn cron_history_concurrent_runs_do_not_overwrite() {
    let tmp = TempDir::new().unwrap();
    let cfg = config(&tmp);
    let job = add_job(&cfg, "*/5 * * * *", "echo ok").unwrap();
    let started = Utc::now();
    let first = register(&cfg, &job, started);
    let second = register(&cfg, &job, started);
    let runs = snapshot(&cfg);
    assert_eq!(runs.len(), 2);
    assert_ne!(runs[0].execution_id, runs[1].execution_id);
    drop(first);
    assert_eq!(snapshot(&cfg).len(), 1);
    drop(second);
    assert!(snapshot(&cfg).is_empty());
}

// A local HTTP connection blocks the real shell command until the test releases it.
#[cfg(not(target_arch = "wasm32"))]
pub(crate) async fn shell_barrier() -> (tokio::net::TcpListener, String) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let command = format!(
        "curl --noproxy '*' --silent --max-time 10 http://{}/",
        listener.local_addr().unwrap()
    );
    (listener, command)
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) async fn release_shell(mut stream: tokio::net::TcpStream) {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let mut request = [0; 4096];
    stream.read(&mut request).await.unwrap();
    stream
        .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\nok")
        .await
        .unwrap();
}

#[tokio::test]
async fn cron_history_cancelled_run_cleans_snapshot() {
    let tmp = TempDir::new().unwrap();
    let cfg = std::sync::Arc::new(config(&tmp));
    let job = add_job(&cfg, "*/5 * * * *", "echo ok").unwrap();
    let (ready, started) = tokio::sync::oneshot::channel();
    let task_cfg = cfg.clone();
    let task = tokio::spawn(async move {
        let _guard = register(&task_cfg, &job, Utc::now());
        ready.send(()).unwrap();
        std::future::pending::<()>().await;
    });
    started.await.unwrap();
    assert_eq!(snapshot(&cfg).len(), 1);
    task.abort();
    assert!(task.await.unwrap_err().is_cancelled());
    assert!(snapshot(&cfg).is_empty());
}

#[test]
fn cron_history_unwind_cleans_snapshot() {
    let tmp = TempDir::new().unwrap();
    let cfg = config(&tmp);
    let job = add_job(&cfg, "*/5 * * * *", "echo ok").unwrap();
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _guard = register(&cfg, &job, Utc::now());
        assert_eq!(snapshot(&cfg).len(), 1);
        panic!("test unwind");
    }));
    assert_eq!(result.unwrap_err().downcast_ref::<&str>(), Some(&"test unwind"));
    assert!(snapshot(&cfg).is_empty());
}
