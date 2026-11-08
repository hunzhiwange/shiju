use std::{
    fs, future,
    os::unix::fs::{PermissionsExt, symlink},
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

use super::discovery_agy::discover;
use crate::apps::discovery::read::LocalReadEnvironment;
use vw_api_types::app_providers::agy::AgyProfile;

fn host_with_cli(script: &str) -> (tempfile::TempDir, LocalReadEnvironment) {
    let temp = tempfile::tempdir().unwrap();
    let home = temp.path().canonicalize().unwrap();
    let bin = home.join("trusted-bin");
    fs::create_dir(&bin).unwrap();
    let cli = bin.join("agy");
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

fn saved_key() -> AgyProfile {
    AgyProfile {
        name: "fixture".into(),
        model: Some("Unrelated saved model".into()),
        api_key: Some("fixture-key".into()),
        base_url: Some("https://models.example.test".into()),
    }
}

fn native_settings(host: &LocalReadEnvironment) -> PathBuf {
    host.home.join(".gemini/antigravity-cli/settings.json")
}

async fn marker(path: &Path) -> String {
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            if let Ok(value) = fs::read_to_string(path)
                && let Some(value) = value.strip_suffix('\n')
            {
                return value.to_owned();
            }
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    })
    .await
    .expect("CLI did not write marker")
}

async fn assert_process_ended(pid: i32) {
    assert!(pid > 1);
    tokio::time::timeout(Duration::from_secs(5), async {
        // SAFETY: signal zero only queries the fixture's process ID.
        while unsafe { libc::kill(pid, 0) } == 0 {
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
        assert_eq!(std::io::Error::last_os_error().raw_os_error(), Some(libc::ESRCH));
    })
    .await
    .expect("CLI process survived");
}

#[tokio::test]
async fn catalog_discovery_agy_returns_display_name() {
    let (_temp, host) = host_with_cli(
        r#"
[ "$#" -eq 1 ] && [ "$1" = models ] || exit 10
[ "$(/bin/pwd -P)" = "$HOME" ] || exit 11
[ -f "$HOME/.gemini/antigravity-cli/login-fixture" ] || exit 12
[ -z "${GEMINI_API_KEY+x}${GOOGLE_API_KEY+x}${GOOGLE_GEMINI_BASE_URL+x}${GOOGLE_APPLICATION_CREDENTIALS+x}" ] || exit 13
printf 'alias-cli\tGemini 3.8 Flash (High)\n'
"#,
    );
    let settings = native_settings(&host);
    fs::create_dir_all(settings.parent().unwrap()).unwrap();
    fs::write(settings.with_file_name("login-fixture"), "").unwrap();
    assert_eq!(
        discover(&host, None, future::pending()).await,
        Ok(vec!["Gemini 3.8 Flash (High)".into()])
    );
    assert!(!settings.exists());
}

#[tokio::test]
async fn catalog_discovery_agy_deduplicates_display_names_and_rejects_malformed_rows() {
    let (_temp, host) = host_with_cli(
        r#"printf 'notice\n\tMissing alias\nbad alias\tInvalid\nempty\t \ncontrol\tBad\033Name\nextra\tName\tColumn\ntrailing\tName\t\na/one:1\t模型 B \r\na.two\t Model A\na-three\t模型 B\n'"#,
    );
    assert_eq!(
        discover(&host, None, future::pending()).await,
        Ok(vec!["Model A".into(), "模型 B".into()])
    );
}

#[tokio::test]
async fn catalog_discovery_agy_rejects_bad_native_config_before_spawn() {
    let (_temp, host) = host_with_cli(r#"touch "$HOME/started""#);
    let settings = native_settings(&host);
    fs::create_dir_all(settings.parent().unwrap()).unwrap();
    for bad in ["{broken", "[]", r#"{"model":42}"#] {
        fs::write(&settings, bad).unwrap();
        assert_eq!(discover(&host, None, future::pending()).await, Err("invalid_config"));
    }
    fs::remove_file(&settings).unwrap();
    symlink(host.home.join("foreign"), &settings).unwrap();
    assert_eq!(discover(&host, None, future::pending()).await, Err("invalid_config"));
    assert!(!host.home.join("started").exists());
}

#[tokio::test]
async fn catalog_discovery_agy_uses_private_key_mode_without_editing_native_settings() {
    let (_temp, host) = host_with_cli(
        r#"
[ "$#" -eq 2 ] && [ "$2" = models ] || exit 10
case "$1" in --gemini_dir=*) dir=${1#*=};; *) exit 11;; esac
[ "$(/bin/pwd -P)" = "$dir" ] || exit 12
[ "$HOME" = "$dir" ] && [ "$XDG_CONFIG_HOME" = "$dir/.config" ] || exit 13
[ "$GEMINI_API_KEY" = fixture-key ] || exit 14
[ "$GOOGLE_GEMINI_BASE_URL" = https://models.example.test ] || exit 15
[ -z "${GOOGLE_API_KEY+x}${GOOGLE_APPLICATION_CREDENTIALS+x}" ] || exit 16
[ "$(/bin/cat "$dir/antigravity-cli/settings.json")" = '{"modelProvider":"gemini"}' ] || exit 17
printf '%s\n' "$dir" > "${0%/*}/scratch"
printf 'key-alias\tKey Mode Display\n'
"#,
    );
    let settings = native_settings(&host);
    fs::create_dir_all(settings.parent().unwrap()).unwrap();
    let original = br#"{"model":"Existing Native Display"}"#;
    fs::write(&settings, original).unwrap();

    assert_eq!(
        discover(&host, Some(&saved_key()), future::pending()).await,
        Ok(vec!["Key Mode Display".into()])
    );
    assert_eq!(fs::read(settings).unwrap(), original);
    assert!(!Path::new(&marker(&host.search_paths[0].join("scratch")).await).exists());
}

#[tokio::test]
async fn catalog_discovery_agy_rejects_bad_saved_credentials_before_spawn() {
    let (_temp, host) = host_with_cli(r#"touch "$HOME/started""#);
    for api_key in [Some("[redacted]".into()), None] {
        let profile = AgyProfile { api_key, ..saved_key() };
        assert_eq!(discover(&host, Some(&profile), future::pending()).await, Err("invalid_config"));
    }
    assert!(!host.home.join("started").exists());
}

#[tokio::test]
async fn catalog_discovery_agy_requires_a_trusted_cli() {
    let (_temp, mut host) = host_with_cli(r#"touch "$HOME/started""#);
    host.search_paths.clear();
    assert_eq!(discover(&host, None, future::pending()).await, Err("unavailable"));
    assert!(!host.home.join("started").exists());
}

#[tokio::test]
async fn catalog_discovery_agy_discards_failed_output_and_removes_scratch() {
    let (_temp, host) = host_with_cli(
        r#"
printf '%s\n' "${1#*=}" > "${0%/*}/scratch"
printf 'alias\t%s\n' "$GEMINI_API_KEY"
echo "$GOOGLE_GEMINI_BASE_URL" >&2
exit 3
"#,
    );
    assert_eq!(discover(&host, Some(&saved_key()), future::pending()).await, Err("unavailable"));
    assert!(!Path::new(&marker(&host.search_paths[0].join("scratch")).await).exists());
}

#[tokio::test]
async fn catalog_discovery_agy_rejects_invalid_or_oversized_output() {
    for script in [r#"printf 'alias\t\377\n'"#, "/usr/bin/head -c 1048577 /dev/zero"] {
        let (_temp, host) = host_with_cli(script);
        assert_eq!(discover(&host, None, future::pending()).await, Err("invalid_response"));
    }
}

#[tokio::test]
async fn catalog_discovery_agy_cancelled_before_spawn() {
    let (_temp, host) = host_with_cli(r#"touch "${0%/*}/started""#);
    assert_eq!(discover(&host, Some(&saved_key()), future::ready(())).await, Err("cancelled"));
    assert!(!host.search_paths[0].join("started").exists());
}

#[tokio::test]
async fn catalog_discovery_agy_cancellation_removes_scratch_and_stops_process_group() {
    let (_temp, host) = host_with_cli(
        r#"
printf '%s\n' "${1#*=}" > "${0%/*}/scratch"
echo $$ > "${0%/*}/parent"
/bin/sleep 60 &
echo $! > "${0%/*}/child"
wait
"#,
    );
    let bin = &host.search_paths[0];
    let cancelled = async {
        marker(&bin.join("child")).await;
        assert!(Path::new(&marker(&bin.join("scratch")).await).is_dir());
    };
    assert_eq!(discover(&host, Some(&saved_key()), cancelled).await, Err("cancelled"));
    assert!(!Path::new(&marker(&bin.join("scratch")).await).exists());
    for file in ["parent", "child"] {
        assert_process_ended(marker(&bin.join(file)).await.parse().unwrap()).await;
    }
}

#[tokio::test]
async fn catalog_discovery_agy_enforces_twenty_second_deadline_and_removes_scratch() {
    let (_temp, host) = host_with_cli(
        r#"
printf '%s\n' "${1#*=}" > "${0%/*}/scratch"
echo $$ > "${0%/*}/parent"
exec /bin/sleep 60
"#,
    );
    let clock = Instant::now();
    assert_eq!(discover(&host, Some(&saved_key()), future::pending()).await, Err("timed_out"));
    assert!(clock.elapsed() >= Duration::from_secs(20));
    assert!(clock.elapsed() < Duration::from_secs(30));
    let bin = &host.search_paths[0];
    assert!(!Path::new(&marker(&bin.join("scratch")).await).exists());
    assert_process_ended(marker(&bin.join("parent")).await.parse().unwrap()).await;
}
