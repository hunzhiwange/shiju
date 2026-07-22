use super::*;

fn app() -> App {
    App::new().0
}

#[test]
fn storage_update_applies_each_field_and_clears_save_error() {
    let mut app = app();
    app.storage_settings.connect_timeout_secs_input = "30".to_string();
    app.storage_settings.save_error = Some("old error".to_string());

    let _ = update(
        &mut app,
        SettingsMessage::Storage(StorageMessage::ProviderChanged("postgres".to_string())),
    );
    let _ = update(
        &mut app,
        SettingsMessage::Storage(StorageMessage::DbUrlChanged(
            "postgres://localhost/vibe".to_string(),
        )),
    );
    let _ = update(
        &mut app,
        SettingsMessage::Storage(StorageMessage::SchemaChanged("archive".to_string())),
    );
    let _ = update(
        &mut app,
        SettingsMessage::Storage(StorageMessage::TableChanged("entries".to_string())),
    );
    let _ = update(
        &mut app,
        SettingsMessage::Storage(StorageMessage::ConnectTimeoutSecsChanged("45".to_string())),
    );
    let _ = update(&mut app, SettingsMessage::Storage(StorageMessage::TlsToggled(true)));

    assert_eq!(app.storage_settings.provider, "postgres");
    assert_eq!(app.storage_settings.db_url_input, "postgres://localhost/vibe");
    assert_eq!(app.storage_settings.schema, "archive");
    assert_eq!(app.storage_settings.table, "entries");
    assert_eq!(app.storage_settings.connect_timeout_secs_input, "45");
    assert!(app.storage_settings.tls);
    assert!(app.storage_settings.save_error.is_none());
}

#[test]
fn storage_save_accepts_blank_and_maximum_timeout() {
    let mut app = app();
    app.storage_settings.provider = " postgres ".to_string();
    app.storage_settings.db_url_input = " ".to_string();
    app.storage_settings.schema = " ".to_string();
    app.storage_settings.table = " ".to_string();
    app.storage_settings.connect_timeout_secs_input = " ".to_string();
    app.storage_settings.save_error = Some("old error".to_string());

    let blank_result = persist_storage_settings(&mut app);
    assert!(blank_result.is_ok());

    app.storage_settings.connect_timeout_secs_input = u64::MAX.to_string();
    let maximum_result = persist_storage_settings(&mut app);
    assert!(maximum_result.is_ok());

    let _ = update(&mut app, SettingsMessage::Storage(StorageMessage::Save));
    assert!(app.storage_settings.save_error.is_none());
}

#[test]
fn storage_update_rejects_invalid_and_overflowing_timeouts() {
    let mut app = app();

    for value in ["seconds", "18446744073709551616"] {
        let _ = update(
            &mut app,
            SettingsMessage::Storage(StorageMessage::ConnectTimeoutSecsChanged(value.to_string())),
        );

        assert_eq!(app.storage_settings.connect_timeout_secs_input, value);
        assert_eq!(
            app.storage_settings.save_error.as_deref(),
            Some("connect_timeout_secs 必须是整数秒或留空")
        );
    }
}

#[test]
fn non_storage_message_leaves_storage_state_unchanged() {
    let mut app = app();
    app.storage_settings.provider = "sqlite".to_string();
    app.storage_settings.save_error = Some("keep this error".to_string());

    let _ = update(&mut app, SettingsMessage::ToggleSettingsSidebar);

    assert_eq!(app.storage_settings.provider, "sqlite");
    assert_eq!(app.storage_settings.save_error.as_deref(), Some("keep this error"));
}

#[test]
fn valid_timeout_after_parse_error_clears_save_error() {
    let mut app = app();

    let _ = update(
        &mut app,
        SettingsMessage::Storage(StorageMessage::ConnectTimeoutSecsChanged(
            "not-seconds".to_string(),
        )),
    );
    assert!(app.storage_settings.save_error.is_some());

    let _ = update(
        &mut app,
        SettingsMessage::Storage(StorageMessage::ConnectTimeoutSecsChanged(" 75 ".to_string())),
    );

    assert_eq!(app.storage_settings.connect_timeout_secs_input, " 75 ");
    assert!(app.storage_settings.save_error.is_none());
}
