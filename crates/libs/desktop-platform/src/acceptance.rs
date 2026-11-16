//! Explicit window acceptance backend processes; callers own the UI thread and daemon.
use std::{
    path::{Path, PathBuf},
    process::{Child, Command},
    time::{Duration, Instant},
};
pub struct BackendFixture {
    child: Child,
    pub root: PathBuf,
}
impl Drop for BackendFixture {
    fn drop(&mut self) {
        let _ = std::fs::write(self.root.join("stop"), "stop");
        let deadline = Instant::now() + Duration::from_secs(5);
        while self.child.try_wait().ok().flatten().is_none() && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(20));
        }
        if self.child.try_wait().ok().flatten().is_none() {
            let _ = self.child.kill();
        }
        let _ = self.child.wait();
    }
}

impl BackendFixture {
    pub fn ready(&self, timeout_message: &str) -> serde_json::Value {
        let deadline = Instant::now() + Duration::from_secs(30);
        while !self.root.join("ready.json").exists() {
            assert!(Instant::now() < deadline, "{timeout_message}");
            std::thread::sleep(Duration::from_millis(20));
        }
        serde_json::from_slice(&std::fs::read(self.root.join("ready.json")).unwrap()).unwrap()
    }
}

pub fn recovery(backend: &str, root: &Path) -> BackendFixture {
    let child = Command::new(backend)
        .args([
            "gateway::recovery_window_tests::recovery_window_gateway_fixture",
            "--exact",
            "--nocapture",
            "--test-threads=1",
        ])
        .env("VW_MODEL_RECOVERY_TEST_ROOT", &root)
        .env("VIBEWINDOW_TEST_ROOT", &root)
        .env("VW_RECOVERY_WINDOW_HTTP", "1")
        .env("VIBEWINDOW_CONFIG_DIR", root.join("config"))
        .env_remove("VIBEWINDOW_WORKSPACE")
        .env_remove("VIBEWINDOW_SERVER_PASSWORD")
        .env("VW_RECOVERY_WINDOW_API_KEY", "fixture-only")
        .spawn()
        .unwrap();
    BackendFixture { child, root: root.to_owned() }
}

pub fn plugin(backend: &str, root: &Path) -> BackendFixture {
    let child = Command::new(backend)
        .args([
            "plugins::compat::plugin_window_fixture::plugin_window_gateway_fixture",
            "--exact",
            "--ignored",
            "--nocapture",
        ])
        .env("SHIJU_PLUGIN_WINDOW_ROOT", &root)
        .spawn()
        .unwrap();
    std::fs::write(root.join("backend.pid"), child.id().to_string()).unwrap();
    BackendFixture { child, root: root.to_owned() }
}
