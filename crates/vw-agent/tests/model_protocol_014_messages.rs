//! Exercise the complete production relay without compiling every lib unit test.
//! The same wire assertions also run in model_relay::messages_request_tests.
use vw_agent::model_relay::{ProviderRelayBackend, RelayBinding, RelayIdentity, RelayServer};
pub use vw_agent::{config, env, project, provider};

#[path = "../src/model_relay/messages_request_http_tests.rs"]
mod messages_request_http_tests;

#[tokio::test]
async fn model_protocol_014_http_request_reaches_bound_chat_and_finishes_tool_round() {
    let home = tempfile::tempdir().unwrap();
    let root = home.path().canonicalize().unwrap();
    // Follow the existing integration targets: initialize production globals only
    // in a child with a private HOME, so no live catalog or credentials are read.
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
        .expect("production relay child must finish within 30 seconds")
        .unwrap();
    assert!(
        output.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[tokio::test]
#[ignore = "run by the parent test with a private HOME"]
async fn relay_child() {
    let home = std::env::current_dir().unwrap();
    let base = directories::BaseDirs::new().unwrap();
    assert!(base.home_dir().starts_with(&home) && base.data_dir().starts_with(&home));
    let paths = vw_agent::global::paths();
    for path in [&paths.home, &paths.data, &paths.config, &paths.cache, &paths.state] {
        assert!(path.starts_with(&home), "production storage must stay in the fixture HOME");
    }
    messages_request_http_tests::request_reaches_bound_chat_and_finishes_tool_round().await;
}

// Include the actual stateless production modules, with the same Model re-export.
// No converted payloads or capability decisions are reimplemented here.
#[path = "../src/model_relay/chat.rs"]
mod chat;
#[allow(dead_code)]
#[path = "../src/model_relay/messages.rs"]
mod messages;
#[path = "../src/model_relay/messages_request_core_tests.rs"]
mod request_core_tests;
