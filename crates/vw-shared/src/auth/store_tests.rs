use std::collections::HashMap;
use std::fs;

use serde_json::json;

use super::{all_from, get_from, remove_from, resolve_filepath, set_to, write_auth_file};
use crate::auth::{ApiInfo, Info, OauthInfo, WellKnownInfo};

fn api_info(key: &str) -> Info {
    Info::Api(ApiInfo { key: key.to_string() })
}

fn oauth_info(refresh: &str) -> Info {
    Info::Oauth(OauthInfo {
        refresh: refresh.to_string(),
        access: "access-token".to_string(),
        expires: 123,
        account_id: Some("account-1".to_string()),
        enterprise_url: None,
    })
}

fn assert_api_key(info: Option<Info>, expected: &str) {
    match info {
        Some(Info::Api(api)) => assert_eq!(api.key, expected),
        other => panic!("expected api info, got {other:?}"),
    }
}

#[test]
fn resolve_filepath_prefers_legacy_auth_file_when_present() {
    let temp = tempfile::tempdir().expect("temp dir should be created");
    let home = temp.path().join("home");
    let data = temp.path().join("data");
    let legacy = vw_config_types::paths::home_config_dir(&home).join("auth.json");
    let primary = data.join("auth.json");

    fs::create_dir_all(legacy.parent().unwrap()).expect("legacy parent should be created");
    fs::create_dir_all(primary.parent().unwrap()).expect("primary parent should be created");
    fs::write(&legacy, "{}").expect("legacy file should be written");
    fs::write(&primary, "{}").expect("primary file should be written");

    assert_eq!(resolve_filepath(&home, &data), legacy);
}

#[test]
fn resolve_filepath_uses_primary_auth_file_when_legacy_missing() {
    let temp = tempfile::tempdir().expect("temp dir should be created");
    let home = temp.path().join("home");
    let data = temp.path().join("data");
    let primary = data.join("auth.json");

    fs::create_dir_all(primary.parent().unwrap()).expect("primary parent should be created");
    fs::write(&primary, "{}").expect("primary file should be written");

    assert_eq!(resolve_filepath(&home, &data), primary);
}

#[test]
fn resolve_filepath_falls_back_to_legacy_path_when_no_file_exists() {
    let temp = tempfile::tempdir().expect("temp dir should be created");
    let home = temp.path().join("home");
    let data = temp.path().join("data");

    assert_eq!(
        resolve_filepath(&home, &data),
        vw_config_types::paths::home_config_dir(&home).join("auth.json")
    );
}

#[test]
fn all_from_returns_empty_for_unreadable_missing_or_invalid_content() {
    let temp = tempfile::tempdir().expect("temp dir should be created");
    let missing = temp.path().join("missing.json");
    let invalid_json = temp.path().join("invalid.json");
    let non_object = temp.path().join("non_object.json");

    fs::write(&invalid_json, "{").expect("invalid json should be written");
    fs::write(&non_object, "[]").expect("non object json should be written");

    assert!(all_from(&missing).is_empty());
    assert!(all_from(&invalid_json).is_empty());
    assert!(all_from(&non_object).is_empty());
}

#[test]
fn all_from_returns_empty_when_path_is_directory() {
    let temp = tempfile::tempdir().expect("temp dir should be created");
    let path = temp.path().join("auth.json");
    fs::create_dir_all(&path).expect("auth directory should be created");

    assert!(all_from(&path).is_empty());
}

#[test]
fn all_from_reads_valid_entries_and_ignores_malformed_entries() {
    let temp = tempfile::tempdir().expect("temp dir should be created");
    let path = temp.path().join("auth.json");

    let content = json!({
        "api": {
            "type": "api",
            "key": "api-key"
        },
        "oauth": {
            "type": "oauth",
            "refresh": "refresh-token",
            "access": "access-token",
            "expires": 42,
            "accountId": "account-1",
            "enterpriseUrl": null
        },
        "wellknown": {
            "type": "wellknown",
            "key": "wellknown-key",
            "token": "wellknown-token"
        },
        "bad": {
            "type": "api"
        }
    });
    fs::write(&path, content.to_string()).expect("auth file should be written");

    let data = all_from(&path);

    assert_eq!(data.len(), 3);
    assert_api_key(data.get("api").cloned(), "api-key");
    match data.get("oauth") {
        Some(Info::Oauth(oauth)) => {
            assert_eq!(oauth.refresh, "refresh-token");
            assert_eq!(oauth.account_id.as_deref(), Some("account-1"));
        }
        other => panic!("expected oauth info, got {other:?}"),
    }
    match data.get("wellknown") {
        Some(Info::Wellknown(wellknown)) => {
            assert_eq!(wellknown.key, "wellknown-key");
            assert_eq!(wellknown.token, "wellknown-token");
        }
        other => panic!("expected wellknown info, got {other:?}"),
    }
    assert!(!data.contains_key("bad"));
}

#[test]
fn get_from_returns_matching_provider_only() {
    let temp = tempfile::tempdir().expect("temp dir should be created");
    let path = temp.path().join("auth.json");

    set_to(&path, "openai", &api_info("openai-key")).expect("auth should be written");

    assert_api_key(get_from(&path, "openai"), "openai-key");
    assert!(get_from(&path, "anthropic").is_none());
}

#[test]
fn set_to_creates_parent_directory_and_preserves_existing_entries() {
    let temp = tempfile::tempdir().expect("temp dir should be created");
    let path = temp.path().join("nested").join("auth.json");

    set_to(&path, "openai", &api_info("old-key")).expect("first auth should be written");
    set_to(&path, "anthropic", &oauth_info("refresh-token"))
        .expect("second auth should be written");
    set_to(&path, "openai", &api_info("new-key")).expect("auth should be overwritten");

    let data = all_from(&path);
    assert_eq!(data.len(), 2);
    assert_api_key(data.get("openai").cloned(), "new-key");
    match data.get("anthropic") {
        Some(Info::Oauth(oauth)) => assert_eq!(oauth.refresh, "refresh-token"),
        other => panic!("expected oauth info, got {other:?}"),
    }
}

#[test]
fn set_to_replaces_malformed_file_with_requested_entry() {
    let temp = tempfile::tempdir().expect("temp dir should be created");
    let path = temp.path().join("auth.json");
    fs::write(&path, "{").expect("malformed auth file should be written");

    set_to(&path, "openai", &api_info("replacement-key"))
        .expect("replacement auth should be written");

    let data = all_from(&path);
    assert_eq!(data.len(), 1);
    assert_api_key(data.get("openai").cloned(), "replacement-key");
}

#[test]
fn set_to_round_trips_oauth_boundary_fields() {
    let temp = tempfile::tempdir().expect("temp dir should be created");
    let path = temp.path().join("auth.json");
    let info = Info::Oauth(OauthInfo {
        refresh: "refresh-value".to_string(),
        access: "access-value".to_string(),
        expires: i64::MIN,
        account_id: None,
        enterprise_url: Some("https://enterprise.example".to_string()),
    });

    set_to(&path, "oauth-provider", &info).expect("oauth auth should be written");

    match get_from(&path, "oauth-provider") {
        Some(Info::Oauth(oauth)) => {
            assert_eq!(oauth.refresh, "refresh-value");
            assert_eq!(oauth.access, "access-value");
            assert_eq!(oauth.expires, i64::MIN);
            assert!(oauth.account_id.is_none());
            assert_eq!(oauth.enterprise_url.as_deref(), Some("https://enterprise.example"));
        }
        other => panic!("expected oauth info, got {other:?}"),
    }
}

#[test]
fn remove_from_deletes_matching_entry_and_keeps_others() {
    let temp = tempfile::tempdir().expect("temp dir should be created");
    let path = temp.path().join("auth.json");

    set_to(&path, "openai", &api_info("openai-key")).expect("openai auth should be written");
    set_to(
        &path,
        "known",
        &Info::Wellknown(WellKnownInfo {
            key: "wellknown-key".to_string(),
            token: "wellknown-token".to_string(),
        }),
    )
    .expect("wellknown auth should be written");

    remove_from(&path, "openai").expect("openai auth should be removed");
    remove_from(&path, "missing").expect("missing auth removal should still write");

    let data = all_from(&path);
    assert_eq!(data.len(), 1);
    assert!(!data.contains_key("openai"));
    match data.get("known") {
        Some(Info::Wellknown(wellknown)) => assert_eq!(wellknown.token, "wellknown-token"),
        other => panic!("expected wellknown info, got {other:?}"),
    }
}

#[test]
fn remove_from_missing_file_creates_empty_store() {
    let temp = tempfile::tempdir().expect("temp dir should be created");
    let path = temp.path().join("nested").join("auth.json");

    remove_from(&path, "missing").expect("empty auth store should be written");

    assert!(path.is_file());
    assert!(all_from(&path).is_empty());
    let value: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(&path).expect("empty auth store should be readable"),
    )
    .expect("empty auth store should contain valid json");
    assert_eq!(value, json!({}));
}

#[cfg(unix)]
#[test]
fn set_to_writes_auth_file_with_user_only_permissions() {
    use std::os::unix::fs::PermissionsExt;

    let temp = tempfile::tempdir().expect("temp dir should be created");
    let path = temp.path().join("auth.json");

    set_to(&path, "openai", &api_info("openai-key")).expect("auth should be written");

    let mode =
        fs::metadata(&path).expect("auth metadata should be readable").permissions().mode() & 0o777;
    assert_eq!(mode, 0o600);
}

#[cfg(unix)]
#[test]
fn set_to_restricts_permissions_on_existing_auth_file() {
    use std::os::unix::fs::PermissionsExt;

    let temp = tempfile::tempdir().expect("temp dir should be created");
    let path = temp.path().join("auth.json");
    fs::write(&path, "{}").expect("auth file should be written");
    fs::set_permissions(&path, fs::Permissions::from_mode(0o644))
        .expect("broad permissions should be set");

    set_to(&path, "openai", &api_info("openai-key")).expect("auth should be written");

    let mode =
        fs::metadata(&path).expect("auth metadata should be readable").permissions().mode() & 0o777;
    assert_eq!(mode, 0o600);
}

#[test]
fn set_to_returns_error_when_parent_path_is_a_file() {
    let temp = tempfile::tempdir().expect("temp dir should be created");
    let parent_file = temp.path().join("not-a-directory");
    let path = parent_file.join("auth.json");
    fs::write(&parent_file, "").expect("parent file should be written");

    let err = set_to(&path, "openai", &api_info("openai-key")).expect_err("write should fail");

    assert!(
        matches!(err.kind(), std::io::ErrorKind::AlreadyExists | std::io::ErrorKind::NotADirectory),
        "unexpected error kind: {:?}",
        err.kind()
    );
}

#[test]
fn set_to_returns_error_when_auth_path_is_directory() {
    let temp = tempfile::tempdir().expect("temp dir should be created");
    let path = temp.path().join("auth.json");
    fs::create_dir_all(&path).expect("auth directory should be created");

    let err = set_to(&path, "openai", &api_info("openai-key")).expect_err("write should fail");

    assert_eq!(err.kind(), std::io::ErrorKind::IsADirectory);
}

#[test]
fn write_auth_file_returns_error_for_empty_path_without_parent() {
    let err = write_auth_file(std::path::Path::new(""), &HashMap::new())
        .expect_err("empty path write should fail");

    assert_eq!(err.kind(), std::io::ErrorKind::NotFound);
}
