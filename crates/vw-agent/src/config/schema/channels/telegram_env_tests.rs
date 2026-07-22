use super::{ChannelsConfig, TelegramConfig, resolve_telegram_allowed_users_env_refs};
use std::ffi::{OsStr, OsString};
use std::sync::{Mutex, MutexGuard};

static ENV_LOCK: Mutex<()> = Mutex::new(());

struct EnvVarGuard {
    key: &'static str,
    old_value: Option<OsString>,
}

impl EnvVarGuard {
    fn set(key: &'static str, value: impl AsRef<OsStr>) -> Self {
        let old_value = std::env::var_os(key);
        unsafe {
            std::env::set_var(key, value);
        }
        Self { key, old_value }
    }

    fn unset(key: &'static str) -> Self {
        let old_value = std::env::var_os(key);
        unsafe {
            std::env::remove_var(key);
        }
        Self { key, old_value }
    }
}

impl Drop for EnvVarGuard {
    fn drop(&mut self) {
        unsafe {
            match &self.old_value {
                Some(value) => std::env::set_var(self.key, value),
                None => std::env::remove_var(self.key),
            }
        }
    }
}

fn env_lock() -> MutexGuard<'static, ()> {
    ENV_LOCK.lock().unwrap()
}

fn telegram_channels(allowed_users: Vec<&str>) -> ChannelsConfig {
    ChannelsConfig {
        telegram: Some(TelegramConfig {
            bot_token: "token".to_string(),
            allowed_users: allowed_users.into_iter().map(ToString::to_string).collect(),
            stream_mode: Default::default(),
            draft_update_interval_ms: 1000,
            interrupt_on_new_message: false,
            mention_only: false,
            group_reply: None,
            base_url: None,
        }),
        ..Default::default()
    }
}

#[test]
fn telegram_env_refs_skip_when_telegram_channel_is_missing() {
    let mut channels = ChannelsConfig::default();

    resolve_telegram_allowed_users_env_refs(&mut channels).unwrap();

    assert!(channels.telegram.is_none());
}

#[test]
fn telegram_env_refs_trim_plain_entries_without_env_lookup() {
    let mut channels = ChannelsConfig {
        telegram: Some(TelegramConfig {
            bot_token: "token".to_string(),
            allowed_users: vec![" alice ".to_string(), "".to_string(), "123".to_string()],
            stream_mode: Default::default(),
            draft_update_interval_ms: 1000,
            interrupt_on_new_message: false,
            mention_only: false,
            group_reply: None,
            base_url: None,
        }),
        ..Default::default()
    };

    resolve_telegram_allowed_users_env_refs(&mut channels).unwrap();

    assert_eq!(
        channels.telegram.unwrap().allowed_users,
        vec!["alice".to_string(), "123".to_string()]
    );
}

#[test]
fn telegram_env_refs_expand_json_and_comma_lists() {
    let _lock = env_lock();
    let _json = EnvVarGuard::set("VW_TEST_ALLOWED_JSON", r#"["100", 200, " 300 "]"#);
    let _list = EnvVarGuard::set("VW_TEST_ALLOWED_LIST", "400, 500,,");
    let mut channels = ChannelsConfig {
        telegram: Some(TelegramConfig {
            bot_token: "token".to_string(),
            allowed_users: vec![
                "${env:VW_TEST_ALLOWED_JSON}".to_string(),
                "${env:VW_TEST_ALLOWED_LIST}".to_string(),
            ],
            stream_mode: Default::default(),
            draft_update_interval_ms: 1000,
            interrupt_on_new_message: false,
            mention_only: false,
            group_reply: None,
            base_url: None,
        }),
        ..Default::default()
    };

    resolve_telegram_allowed_users_env_refs(&mut channels).unwrap();

    assert_eq!(
        channels.telegram.unwrap().allowed_users,
        vec![
            "100".to_string(),
            "200".to_string(),
            "300".to_string(),
            "400".to_string(),
            "500".to_string()
        ]
    );
}

#[test]
fn telegram_env_refs_expand_mixed_entries_with_trimmed_env_name() {
    let _lock = env_lock();
    let _env = EnvVarGuard::set("VW_TEST_ALLOWED_SPACED", " 700, 800 ");
    let mut channels =
        telegram_channels(vec![" alice ", "${env:  VW_TEST_ALLOWED_SPACED  }", "bob"]);

    resolve_telegram_allowed_users_env_refs(&mut channels).unwrap();

    assert_eq!(
        channels.telegram.unwrap().allowed_users,
        vec!["alice".to_string(), "700".to_string(), "800".to_string(), "bob".to_string()]
    );
}

#[test]
fn telegram_env_refs_reject_invalid_env_names_and_empty_values() {
    let _lock = env_lock();
    let mut channels = ChannelsConfig {
        telegram: Some(TelegramConfig {
            bot_token: "token".to_string(),
            allowed_users: vec!["${env:BAD-NAME}".to_string()],
            stream_mode: Default::default(),
            draft_update_interval_ms: 1000,
            interrupt_on_new_message: false,
            mention_only: false,
            group_reply: None,
            base_url: None,
        }),
        ..Default::default()
    };

    assert!(
        resolve_telegram_allowed_users_env_refs(&mut channels)
            .unwrap_err()
            .to_string()
            .contains("invalid env var name")
    );

    let _env = EnvVarGuard::set("VW_TEST_ALLOWED_EMPTY", "   ");
    channels.telegram.as_mut().unwrap().allowed_users =
        vec!["${env:VW_TEST_ALLOWED_EMPTY}".to_string()];
    let err = resolve_telegram_allowed_users_env_refs(&mut channels).unwrap_err().to_string();

    assert!(err.contains("empty value"));
}

#[test]
fn telegram_env_refs_reject_unset_env_var() {
    let _lock = env_lock();
    let _env = EnvVarGuard::unset("VW_TEST_ALLOWED_UNSET");
    let mut channels = telegram_channels(vec!["${env:VW_TEST_ALLOWED_UNSET}"]);

    let err = resolve_telegram_allowed_users_env_refs(&mut channels).unwrap_err().to_string();

    assert!(err.contains("references unset environment variable VW_TEST_ALLOWED_UNSET"));
}

#[test]
fn telegram_env_refs_reject_invalid_json_array() {
    let _lock = env_lock();
    let _env = EnvVarGuard::set("VW_TEST_ALLOWED_INVALID_JSON", r#"["100""#);
    let mut channels = telegram_channels(vec!["${env:VW_TEST_ALLOWED_INVALID_JSON}"]);

    let err = resolve_telegram_allowed_users_env_refs(&mut channels).unwrap_err().to_string();

    assert!(err.contains("must be valid JSON array or comma-separated list"));
}

#[test]
fn telegram_env_refs_reject_json_array_with_unsupported_item() {
    let _lock = env_lock();
    let _env = EnvVarGuard::set("VW_TEST_ALLOWED_BAD_ITEM", r#"["100", true]"#);
    let mut channels = telegram_channels(vec!["${env:VW_TEST_ALLOWED_BAD_ITEM}"]);

    let err = resolve_telegram_allowed_users_env_refs(&mut channels).unwrap_err().to_string();

    assert!(err.contains("[1] must be string or number"));
}

#[test]
fn telegram_env_refs_reject_env_values_without_user_ids() {
    let _lock = env_lock();
    let _json = EnvVarGuard::set("VW_TEST_ALLOWED_EMPTY_JSON", r#"[" ", ""]"#);
    let mut channels = telegram_channels(vec!["${env:VW_TEST_ALLOWED_EMPTY_JSON}"]);

    let json_err = resolve_telegram_allowed_users_env_refs(&mut channels).unwrap_err().to_string();

    let _list = EnvVarGuard::set("VW_TEST_ALLOWED_EMPTY_LIST", " , , ");
    let mut channels = telegram_channels(vec!["${env:VW_TEST_ALLOWED_EMPTY_LIST}"]);

    let list_err = resolve_telegram_allowed_users_env_refs(&mut channels).unwrap_err().to_string();

    assert!(json_err.contains("produced no user IDs"));
    assert!(list_err.contains("produced no user IDs"));
}

#[test]
fn telegram_env_refs_ignore_blank_json_items_and_stringify_numbers() {
    let _lock = env_lock();
    let _env =
        EnvVarGuard::set("VW_TEST_ALLOWED_MIXED_JSON", r#"  ["", "  ", -42, 3.5, 0, " 900 "]  "#);
    let mut channels = telegram_channels(vec!["${env:VW_TEST_ALLOWED_MIXED_JSON}"]);

    resolve_telegram_allowed_users_env_refs(&mut channels).unwrap();

    assert_eq!(
        channels.telegram.unwrap().allowed_users,
        vec!["-42".to_string(), "3.5".to_string(), "0".to_string(), "900".to_string()]
    );
}

#[test]
fn telegram_env_refs_treat_incomplete_env_expression_as_plain_entry() {
    let mut channels = telegram_channels(vec!["  ${env:VW_TEST_ALLOWED_LITERAL  "]);

    resolve_telegram_allowed_users_env_refs(&mut channels).unwrap();

    assert_eq!(
        channels.telegram.unwrap().allowed_users,
        vec!["${env:VW_TEST_ALLOWED_LITERAL".to_string()]
    );
}

#[test]
fn telegram_env_refs_accept_underscore_prefixed_lowercase_env_name() {
    let _lock = env_lock();
    let _env = EnvVarGuard::set("_vw_test_allowed_users2", "901");
    let mut channels = telegram_channels(vec!["${env:_vw_test_allowed_users2}"]);

    resolve_telegram_allowed_users_env_refs(&mut channels).unwrap();

    assert_eq!(channels.telegram.unwrap().allowed_users, vec!["901".to_string()]);
}

#[test]
fn telegram_env_refs_report_original_index_for_invalid_reference() {
    let mut channels = telegram_channels(vec![" ", "alice", "${env:9INVALID}"]);

    let err = resolve_telegram_allowed_users_env_refs(&mut channels).unwrap_err().to_string();

    assert!(err.contains("allowed_users[2] has invalid env var name (9INVALID)"));
}
