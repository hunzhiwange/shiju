//! Real ProviderRelayBackend wire test, isolated from unrelated lib unit tests.
use vw_agent::model_relay::{ProviderRelayBackend, RelayBinding, RelayIdentity, RelayServer};
pub use vw_agent::{config, env, project};
#[path = "../src/model_relay/messages_thinking_http_tests.rs"]
mod wire;

#[tokio::test]
async fn model_protocol_015_http_tool_round_and_sse_preserve_reasoning() {
    let home = tempfile::tempdir().unwrap();
    let root = home.path().canonicalize().unwrap();
    let mut command = tokio::process::Command::new(std::env::current_exe().unwrap());
    command
        .args(["--exact", "relay_child", "--ignored", "--nocapture"])
        .env_clear()
        .env("PATH", std::env::var_os("PATH").unwrap_or_default())
        .env("HOME", &root)
        .env("VIBEWINDOW_TEST_HOME", &root)
        .env("XDG_DATA_HOME", root.join("data"))
        .env("XDG_CONFIG_HOME", root.join("config"))
        .env("XDG_CACHE_HOME", root.join("cache"))
        .current_dir(&root)
        .kill_on_drop(true);
    let output = tokio::time::timeout(std::time::Duration::from_secs(30), command.output())
        .await
        .unwrap()
        .unwrap();
    assert!(
        output.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[tokio::test]
#[ignore = "run by the parent test in a private HOME"]
async fn relay_child() {
    let root = std::env::current_dir().unwrap();
    let paths = vw_agent::global::paths();
    for path in [&paths.home, &paths.data, &paths.config, &paths.cache, &paths.state] {
        assert!(path.starts_with(&root));
    }
    wire::thinking_replays_through_bound_http().await;
}
