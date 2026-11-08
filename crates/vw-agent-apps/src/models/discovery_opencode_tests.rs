use std::{
    fs, future,
    os::unix::fs::{PermissionsExt, symlink},
    path::Path,
    time::{Duration, Instant},
};

use super::discovery_opencode::{OpenCodeModelRef, discover};
use crate::apps::discovery::read::LocalReadEnvironment;

fn host_with_cli(script: &str) -> (tempfile::TempDir, LocalReadEnvironment) {
    let temp = tempfile::tempdir().unwrap();
    let home = temp.path().canonicalize().unwrap();
    let bin = home.join("trusted-bin");
    fs::create_dir(&bin).unwrap();
    let cli = bin.join("opencode");
    fs::write(&cli, format!("#!/bin/sh\n{script}\n")).unwrap();
    fs::set_permissions(&cli, fs::Permissions::from_mode(0o700)).unwrap();
    let host = LocalReadEnvironment {
        host_id: "test-host".into(),
        home: home.clone(),
        search_paths: vec![bin],
        applications: home.join("Applications"),
    };
    (temp, host)
}

fn model(provider_id: &str, model_id: &str) -> OpenCodeModelRef {
    OpenCodeModelRef { provider_id: provider_id.into(), model_id: model_id.into() }
}

#[tokio::test]
async fn catalog_discovery_opencode_preserves_nested_model_id() {
    let (_temp, host) = host_with_cli("printf '%s\\n' provider/vendor/model provider/vendor/model");
    assert_eq!(
        discover(&host, None, future::pending()).await,
        Ok(vec![model("provider", "vendor/model")])
    );
}

#[tokio::test]
async fn catalog_discovery_opencode_sorts_and_filters_malformed_lines() {
    let (_temp, host) = host_with_cli(
        r#"printf 'zen/free\r\n/model\nprovider/\nbad provider/model\nprovider/bad model\nprovider/bad\033[0m\nprovider/good\t\n\tprovider/good\nnotice: loading\n  a.open_router-1/vendor/模型  \r\nzen/free\n'"#,
    );
    assert_eq!(
        discover(&host, None, future::pending()).await,
        Ok(vec![model("a.open_router-1", "vendor/模型"), model("zen", "free")])
    );
}

#[tokio::test]
async fn catalog_discovery_opencode_uses_fixed_command_and_host_config() {
    let (_temp, host) = host_with_cli(
        r#"
[ "$#" -eq 1 ] && [ "$1" = models ] || exit 10
[ "$OPENCODE_CONFIG_DIR" = "$HOME/custom config" ] || exit 11
[ "$OPENCODE_DISABLE_PROJECT_CONFIG" = true ] || exit 12
[ "$(/bin/pwd -P)" = "$OPENCODE_CONFIG_DIR" ] || exit 13
[ "$XDG_DATA_HOME" = "$HOME/.local/share" ] || exit 14
printf '%s\n' zen/free
"#,
    );
    fs::create_dir(host.home.join("custom config")).unwrap();
    assert_eq!(
        discover(&host, Some("~/custom config"), future::pending()).await,
        Ok(vec![model("zen", "free")])
    );
}

#[tokio::test]
async fn catalog_discovery_opencode_allows_missing_default_config() {
    let (_temp, host) = host_with_cli(
        r#"
[ "$OPENCODE_CONFIG_DIR" = "$HOME/.config/opencode" ] || exit 10
[ "$(/bin/pwd -P)" = "$HOME" ] || exit 11
printf '%s\n' zen/free
"#,
    );
    assert_eq!(discover(&host, None, future::pending()).await, Ok(vec![model("zen", "free")]));
    assert!(!host.home.join(".config").exists());
}

#[tokio::test]
async fn catalog_discovery_opencode_rejects_foreign_and_symlinked_config_before_spawn() {
    let (_temp, host) = host_with_cli(r#"touch "$HOME/started""#);
    let foreign = tempfile::tempdir().unwrap();
    symlink(foreign.path(), host.home.join(".config")).unwrap();
    for directory in [Some(foreign.path().to_str().unwrap()), None, Some("~/.config/opencode")] {
        assert_eq!(discover(&host, directory, future::pending()).await, Err("invalid_config"));
    }
    assert!(!host.home.join("started").exists());
}

#[tokio::test]
async fn catalog_discovery_opencode_requires_trusted_cli() {
    let (_temp, mut host) = host_with_cli(r#"touch "$HOME/started""#);
    host.search_paths.clear();
    assert_eq!(discover(&host, None, future::pending()).await, Err("unavailable"));
    host.search_paths.push("trusted-bin".into());
    assert_eq!(discover(&host, None, future::pending()).await, Err("unavailable"));
    assert!(!host.home.join("started").exists());
}

#[tokio::test]
async fn catalog_discovery_opencode_redacts_failed_process_output() {
    let (_temp, host) = host_with_cli("echo secret/model; echo secret-stderr >&2; exit 3");
    assert_eq!(discover(&host, None, future::pending()).await, Err("unavailable"));
}

#[tokio::test]
async fn catalog_discovery_opencode_rejects_invalid_or_oversized_output() {
    for script in [r#"printf 'provider/\377\n'"#, "/usr/bin/head -c 1048577 /dev/zero"] {
        let (_temp, host) = host_with_cli(script);
        assert_eq!(discover(&host, None, future::pending()).await, Err("invalid_response"));
    }
}

#[tokio::test]
async fn catalog_discovery_opencode_cancelled_before_spawn() {
    let (_temp, host) = host_with_cli(r#"touch "$HOME/started""#);
    assert_eq!(discover(&host, None, future::ready(())).await, Err("cancelled"));
    assert!(!host.home.join("started").exists());
}

async fn started(path: &Path) -> i32 {
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            if let Ok(pid) = fs::read_to_string(path)
                && let Some(pid) = pid.strip_suffix('\n').and_then(|line| line.parse().ok())
            {
                return pid;
            }
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    })
    .await
    .expect("CLI did not start")
}

async fn assert_process_ended(pid: i32) {
    assert!(pid > 1);
    tokio::time::timeout(Duration::from_secs(5), async {
        // SAFETY: signal 0 only queries the fixture's process ID.
        while unsafe { libc::kill(pid, 0) } == 0 {
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
        assert_eq!(std::io::Error::last_os_error().raw_os_error(), Some(libc::ESRCH));
    })
    .await
    .expect("CLI process survived");
}

#[tokio::test]
async fn catalog_discovery_opencode_cancellation_ends_process_group() {
    let (_temp, host) = host_with_cli(
        r#"
echo $$ > "$HOME/parent"
/bin/sleep 60 &
echo $! > "$HOME/child"
wait
"#,
    );
    let cancel = async {
        started(&host.home.join("child")).await;
    };
    assert_eq!(discover(&host, None, cancel).await, Err("cancelled"));
    assert_process_ended(started(&host.home.join("parent")).await).await;
    assert_process_ended(started(&host.home.join("child")).await).await;
}

#[tokio::test]
async fn catalog_discovery_opencode_enforces_twenty_second_deadline() {
    let (_temp, host) = host_with_cli(r#"echo $$ > "$HOME/parent"; exec /bin/sleep 60"#);
    let clock = Instant::now();
    assert_eq!(discover(&host, None, future::pending()).await, Err("timed_out"));
    assert!(clock.elapsed() >= Duration::from_secs(20));
    assert!(clock.elapsed() < Duration::from_secs(30));
    assert_process_ended(started(&host.home.join("parent")).await).await;
}
