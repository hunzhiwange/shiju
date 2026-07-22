use crate::app::{App, message::ViewMessage, state::UsageModelInfo};

fn test_app() -> App {
    App::new().0
}

fn model_info(model_id: &str) -> UsageModelInfo {
    UsageModelInfo {
        provider_id: "provider-id".to_string(),
        provider_name: "Provider".to_string(),
        model_id: model_id.to_string(),
        model_name: "Model".to_string(),
        context_limit: 128_000,
        output_limit: 16_000,
        cost_input_per_million: 1.25,
        cost_output_per_million: 5.0,
        cost_cache_read_per_million: 0.125,
        cost_cache_write_per_million: 1.5,
    }
}

#[test]
fn usage_model_info_loaded_replaces_existing_info() {
    let mut app = test_app();
    app.usage_model_info = Some(model_info("old-model"));

    let _task =
        super::update(&mut app, ViewMessage::UsageModelInfoLoaded(Some(model_info("new-model"))));

    let info = app.usage_model_info.as_ref().expect("model info should be stored");
    assert_eq!(info.provider_id, "provider-id");
    assert_eq!(info.provider_name, "Provider");
    assert_eq!(info.model_id, "new-model");
    assert_eq!(info.model_name, "Model");
    assert_eq!(info.context_limit, 128_000);
    assert_eq!(info.output_limit, 16_000);
    assert_eq!(info.cost_input_per_million, 1.25);
    assert_eq!(info.cost_output_per_million, 5.0);
    assert_eq!(info.cost_cache_read_per_million, 0.125);
    assert_eq!(info.cost_cache_write_per_million, 1.5);
}

#[test]
fn usage_model_info_loaded_none_clears_existing_info() {
    let mut app = test_app();
    app.usage_model_info = Some(model_info("old-model"));

    let _task = super::update(&mut app, ViewMessage::UsageModelInfoLoaded(None));

    assert!(app.usage_model_info.is_none());
}

#[test]
fn session_file_path_loaded_for_active_session_stores_utf8_path() {
    let mut app = test_app();
    app.active_session_id = Some("active-session".to_string());

    let _task = super::update(
        &mut app,
        ViewMessage::UsageSessionFilePathLoaded(
            "active-session".to_string(),
            Ok(Some("/tmp/session.sqlite".into())),
        ),
    );

    assert_eq!(app.usage_session_file_path.as_deref(), Some("/tmp/session.sqlite"));
}

#[test]
fn session_file_path_loaded_for_stale_session_preserves_current_path() {
    let mut app = test_app();
    app.active_session_id = Some("active-session".to_string());
    app.usage_session_file_path = Some("/tmp/current.sqlite".to_string());

    let _task = super::update(
        &mut app,
        ViewMessage::UsageSessionFilePathLoaded(
            "stale-session".to_string(),
            Ok(Some("/tmp/stale.sqlite".into())),
        ),
    );

    assert_eq!(app.usage_session_file_path.as_deref(), Some("/tmp/current.sqlite"));
}

#[test]
fn session_file_path_loaded_empty_or_error_clears_active_path() {
    for result in [Ok(None), Err("database unavailable".to_string())] {
        let mut app = test_app();
        app.active_session_id = Some("active-session".to_string());
        app.usage_session_file_path = Some("/tmp/current.sqlite".to_string());

        let _task = super::update(
            &mut app,
            ViewMessage::UsageSessionFilePathLoaded("active-session".to_string(), result),
        );

        assert!(app.usage_session_file_path.is_none());
    }
}

#[cfg(unix)]
#[test]
fn session_file_path_loaded_non_utf8_path_clears_active_path() {
    use std::os::unix::ffi::OsStringExt;

    let mut app = test_app();
    app.active_session_id = Some("active-session".to_string());
    app.usage_session_file_path = Some("/tmp/current.sqlite".to_string());
    let non_utf8_path = std::ffi::OsString::from_vec(vec![b'/', 0xff]).into();

    let _task = super::update(
        &mut app,
        ViewMessage::UsageSessionFilePathLoaded(
            "active-session".to_string(),
            Ok(Some(non_utf8_path)),
        ),
    );

    assert!(app.usage_session_file_path.is_none());
}

#[test]
fn usage_step_toggled_collapses_expanded_step_without_loading() {
    let mut app = test_app();
    app.active_session_id = Some("active-session".to_string());
    app.usage_step_expanded.insert(7);

    let _task = super::update(&mut app, ViewMessage::UsageStepToggled(7));

    assert!(!app.usage_step_expanded.contains(&7));
    assert!(app.usage_step_raw_loading.is_empty());
}

#[test]
fn usage_step_toggled_without_active_session_only_expands_step() {
    let mut app = test_app();
    app.active_session_id = None;

    let _task = super::update(&mut app, ViewMessage::UsageStepToggled(3));

    assert!(app.usage_step_expanded.contains(&3));
    assert!(app.usage_step_raw_loading.is_empty());
}

#[test]
fn usage_step_toggled_with_cached_payload_does_not_reload() {
    let mut app = test_app();
    let key = ("active-session".to_string(), 5);
    app.active_session_id = Some(key.0.clone());
    app.usage_step_raw_payloads
        .insert(key.clone(), Ok(Some(serde_json::json!({ "cached": true }))));

    let _task = super::update(&mut app, ViewMessage::UsageStepToggled(key.1));

    assert!(app.usage_step_expanded.contains(&key.1));
    assert!(!app.usage_step_raw_loading.contains(&key));
    assert_eq!(
        app.usage_step_raw_payloads.get(&key),
        Some(&Ok(Some(serde_json::json!({ "cached": true }))))
    );
}

#[test]
fn usage_step_toggled_while_loading_does_not_duplicate_load() {
    let mut app = test_app();
    let key = ("active-session".to_string(), 9);
    app.active_session_id = Some(key.0.clone());
    app.usage_step_raw_loading.insert(key.clone());

    let _task = super::update(&mut app, ViewMessage::UsageStepToggled(key.1));

    assert!(app.usage_step_expanded.contains(&key.1));
    assert_eq!(app.usage_step_raw_loading.len(), 1);
    assert!(app.usage_step_raw_loading.contains(&key));
}

#[test]
fn usage_step_toggled_uncached_step_marks_it_loading() {
    let mut app = test_app();
    let key = ("active-session".to_string(), 11);
    app.active_session_id = Some(key.0.clone());

    let _task = super::update(&mut app, ViewMessage::UsageStepToggled(key.1));

    assert!(app.usage_step_expanded.contains(&key.1));
    assert!(app.usage_step_raw_loading.contains(&key));
    assert!(!app.usage_step_raw_payloads.contains_key(&key));
}

#[test]
fn usage_step_raw_loaded_for_active_session_caches_success_and_error() {
    let cases = [
        Ok(Some(serde_json::json!({ "request": "cached" }))),
        Err("payload unavailable".to_string()),
    ];

    for (step_index, result) in cases.into_iter().enumerate() {
        let mut app = test_app();
        let key = ("active-session".to_string(), step_index as u32);
        app.active_session_id = Some(key.0.clone());
        app.usage_step_raw_loading.insert(key.clone());

        let _task = super::update(
            &mut app,
            ViewMessage::UsageStepRawLoaded(key.0.clone(), key.1, result.clone()),
        );

        assert!(!app.usage_step_raw_loading.contains(&key));
        assert_eq!(app.usage_step_raw_payloads.get(&key), Some(&result));
    }
}

#[test]
fn usage_step_raw_loaded_caches_explicit_empty_payload() {
    let mut app = test_app();
    let key = ("active-session".to_string(), 21);
    app.active_session_id = Some(key.0.clone());
    app.usage_step_raw_loading.insert(key.clone());

    let _task =
        super::update(&mut app, ViewMessage::UsageStepRawLoaded(key.0.clone(), key.1, Ok(None)));

    assert!(!app.usage_step_raw_loading.contains(&key));
    assert_eq!(app.usage_step_raw_payloads.get(&key), Some(&Ok(None)));
}

#[test]
fn usage_step_raw_loaded_for_stale_session_discards_result_and_clears_loading() {
    let mut app = test_app();
    let key = ("stale-session".to_string(), 13);
    app.active_session_id = Some("active-session".to_string());
    app.usage_step_raw_loading.insert(key.clone());

    let _task = super::update(
        &mut app,
        ViewMessage::UsageStepRawLoaded(
            key.0.clone(),
            key.1,
            Ok(Some(serde_json::json!({ "stale": true }))),
        ),
    );

    assert!(!app.usage_step_raw_loading.contains(&key));
    assert!(!app.usage_step_raw_payloads.contains_key(&key));
}

#[test]
fn unrelated_message_preserves_usage_panel_state() {
    let mut app = test_app();
    let key = ("active-session".to_string(), 17);
    app.active_session_id = Some(key.0.clone());
    app.usage_session_file_path = Some("/tmp/current.sqlite".to_string());
    app.usage_step_expanded.insert(key.1);
    app.usage_step_raw_loading.insert(key.clone());

    let _task = super::update(&mut app, ViewMessage::OpenUsage);

    assert_eq!(app.usage_session_file_path.as_deref(), Some("/tmp/current.sqlite"));
    assert!(app.usage_step_expanded.contains(&key.1));
    assert!(app.usage_step_raw_loading.contains(&key));
}
