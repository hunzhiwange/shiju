use super::*;
async fn collect_task_messages(task: Task<Message>) -> Vec<Message> {
    use iced_runtime::futures::futures::StreamExt;

    let Some(mut stream) = iced_runtime::task::into_stream(task.collect()) else {
        return Vec::new();
    };
    while let Some(action) = stream.next().await {
        if let iced_runtime::Action::Output(messages) = action {
            return messages;
        }
    }
    Vec::new()
}

fn permission_result(messages: Vec<Message>) -> Result<String, String> {
    let mut messages = messages.into_iter();
    let result = match messages.next() {
        Some(SettingsMessage::DialogueFlowPermissionLoaded(result)) => result,
        _ => panic!("expected a dialogue flow permission result"),
    };
    assert!(messages.next().is_none());
    result
}

#[cfg(not(target_arch = "wasm32"))]
// 数据边界：覆盖规范化、默认值以及相关状态字段的既定规则。
// 这里固定可观察结果，避免依赖处理过程中的临时实现细节。
#[tokio::test(flavor = "multi_thread")]
async fn permission_load_without_project_or_config_returns_empty_object() {
    let without_project =
        permission_result(collect_task_messages(dialogue_flow_permission_text_task(None)).await);
    assert_eq!(without_project.as_deref(), Ok("{}"));

    let project = tempfile::tempdir().expect("temporary project should be created");
    let missing_config = permission_result(
        collect_task_messages(dialogue_flow_permission_text_task(Some(
            project.path().to_string_lossy().into_owned(),
        )))
        .await,
    );
    assert_eq!(missing_config.as_deref(), Ok("{}"));
    assert!(!project.path().join("config.json").exists());
}

#[cfg(not(target_arch = "wasm32"))]
#[tokio::test(flavor = "multi_thread")]
async fn permission_load_extracts_and_pretty_prints_permission_value() {
    let project = tempfile::tempdir().expect("temporary project should be created");
    std::fs::write(
        project.path().join("config.json"),
        r#"{"permission":{"shell":{"allow":true}},"theme":"dark"}"#,
    )
    .expect("config should be written");

    let result = permission_result(
        collect_task_messages(dialogue_flow_permission_text_task(Some(
            project.path().to_string_lossy().into_owned(),
        )))
        .await,
    );

    assert_eq!(result.as_deref(), Ok("{\n  \"shell\": {\n    \"allow\": true\n  }\n}"));
}

#[cfg(not(target_arch = "wasm32"))]
#[tokio::test(flavor = "multi_thread")]
async fn permission_load_defaults_missing_permission_and_rejects_invalid_json() {
    let project = tempfile::tempdir().expect("temporary project should be created");
    let config_path = project.path().join("config.json");
    std::fs::write(&config_path, r#"{"theme":"dark"}"#).expect("config should be written");

    let missing_permission = permission_result(
        collect_task_messages(dialogue_flow_permission_text_task(Some(
            project.path().to_string_lossy().into_owned(),
        )))
        .await,
    );
    assert_eq!(missing_permission.as_deref(), Ok("{}"));

    std::fs::write(&config_path, "{invalid json").expect("invalid config should be written");
    let invalid_json = permission_result(
        collect_task_messages(dialogue_flow_permission_text_task(Some(
            project.path().to_string_lossy().into_owned(),
        )))
        .await,
    );
    assert!(invalid_json.is_err());
}

#[cfg(not(target_arch = "wasm32"))]
#[tokio::test(flavor = "multi_thread")]
async fn permission_reset_removes_permission_and_preserves_other_fields() {
    let project = tempfile::tempdir().expect("temporary project should be created");
    let config_path = project.path().join("config.json");
    std::fs::write(&config_path, r#"{"permission":{"shell":"allow"},"theme":"dark","retries":3}"#)
        .expect("config should be written");

    let result = permission_result(
        collect_task_messages(dialogue_flow_permission_reset_task(Some(
            project.path().to_string_lossy().into_owned(),
        )))
        .await,
    );
    let saved: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(config_path).expect("updated config should be readable"),
    )
    .expect("updated config should remain valid JSON");

    assert_eq!(result.as_deref(), Ok("{}"));
    assert_eq!(saved.get("theme").and_then(serde_json::Value::as_str), Some("dark"));
    assert_eq!(saved.get("retries").and_then(serde_json::Value::as_u64), Some(3));
    assert!(saved.get("permission").is_none());
}

#[cfg(not(target_arch = "wasm32"))]
// 失败与边界：覆盖非法、缺失、过期或冲突场景的既定处理规则。
// 重点确认现有状态与错误反馈不会被意外的部分更新污染。
#[tokio::test(flavor = "multi_thread")]
async fn permission_reset_replaces_non_object_root_with_empty_object() {
    let project = tempfile::tempdir().expect("temporary project should be created");
    let config_path = project.path().join("config.json");
    std::fs::write(&config_path, r#"["unexpected"]"#).expect("config should be written");

    let result = permission_result(
        collect_task_messages(dialogue_flow_permission_reset_task(Some(
            project.path().to_string_lossy().into_owned(),
        )))
        .await,
    );

    assert_eq!(result.as_deref(), Ok("{}"));
    assert_eq!(
        std::fs::read_to_string(config_path).expect("updated config should be readable"),
        "{}"
    );
}

#[cfg(not(target_arch = "wasm32"))]
#[tokio::test(flavor = "multi_thread")]
async fn permission_reset_without_config_does_not_create_file() {
    let project = tempfile::tempdir().expect("temporary project should be created");

    let result = permission_result(
        collect_task_messages(dialogue_flow_permission_reset_task(Some(
            project.path().to_string_lossy().into_owned(),
        )))
        .await,
    );

    assert_eq!(result.as_deref(), Ok("{}"));
    assert!(!project.path().join("config.json").exists());
}

#[cfg(not(target_arch = "wasm32"))]
#[tokio::test(flavor = "multi_thread")]
async fn permission_reset_without_project_returns_empty_object() {
    let result =
        permission_result(collect_task_messages(dialogue_flow_permission_reset_task(None)).await);

    assert_eq!(result.as_deref(), Ok("{}"));
}

#[cfg(not(target_arch = "wasm32"))]
#[tokio::test(flavor = "multi_thread")]
async fn permission_load_reports_non_missing_file_errors() {
    let project = tempfile::tempdir().expect("temporary project should be created");
    std::fs::create_dir(project.path().join("config.json"))
        .expect("config-shaped directory should be created");

    let result = permission_result(
        collect_task_messages(dialogue_flow_permission_text_task(Some(
            project.path().to_string_lossy().into_owned(),
        )))
        .await,
    );

    assert!(result.is_err());
}

#[cfg(not(target_arch = "wasm32"))]
#[tokio::test(flavor = "multi_thread")]
async fn permission_reset_rejects_invalid_json_without_overwriting_it() {
    let project = tempfile::tempdir().expect("temporary project should be created");
    let config_path = project.path().join("config.json");
    let invalid_json = "{invalid json";
    std::fs::write(&config_path, invalid_json).expect("invalid config should be written");

    let result = permission_result(
        collect_task_messages(dialogue_flow_permission_reset_task(Some(
            project.path().to_string_lossy().into_owned(),
        )))
        .await,
    );

    assert!(result.is_err());
    assert_eq!(
        std::fs::read_to_string(config_path).expect("original config should remain readable"),
        invalid_json
    );
}
