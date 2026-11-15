//! 运行时追踪模块的单元测试
//!
//! 本模块覆盖配置与路径解析、事件序列化、存储模式、JSONL 查询和 ID 查找。
//! 所有文件系统行为都限制在临时目录内，不触发网络或真实运行时事件。

use super::*;
use std::sync::{LazyLock, Mutex};

static TRACE_LOGGER_TEST_LOCK: LazyLock<Mutex<()>> = LazyLock::new(|| Mutex::new(()));

/// 创建用于测试的观测配置实例
///
/// 返回一个预设了测试默认值的 `ObservabilityConfig`：
/// - `backend`: "none"（禁用外部后端）
/// - `runtime_trace_mode`: "rolling"（滚动模式）
/// - `runtime_trace_max_entries`: 3（最大保留 3 条记录）
fn test_observability_config() -> ObservabilityConfig {
    ObservabilityConfig {
        backend: "none".to_string(),
        otel_endpoint: None,
        otel_service_name: None,
        runtime_trace_mode: "rolling".to_string(),
        runtime_trace_path: "state/runtime-trace.jsonl".to_string(),
        runtime_trace_max_entries: 3,
    }
}

fn test_event(id: &str, event_type: &str) -> RuntimeTraceEvent {
    RuntimeTraceEvent {
        id: id.to_string(),
        timestamp: "2026-07-16T00:00:00Z".to_string(),
        event_type: event_type.to_string(),
        channel: None,
        provider: None,
        model: None,
        turn_id: None,
        success: None,
        message: None,
        payload: serde_json::json!({ "event_id": id }),
    }
}

/// 测试：相对路径应与工作区目录拼接
///
/// # 验证内容
///
/// 当配置中的 `runtime_trace_path` 为相对路径时，
/// `resolve_trace_path` 函数应将其与工作区目录拼接，
/// 生成完整的绝对路径。
///
/// # 预期结果
///
/// 输入相对路径 "state/runtime-trace.jsonl" 时，
/// 输出应为 `<workspace>/state/runtime-trace.jsonl`。
#[test]
fn resolve_trace_path_relative_joins_workspace() {
    let cfg = test_observability_config();
    let workspace = tempfile::tempdir().unwrap();
    let path = resolve_trace_path(&cfg, workspace.path());
    assert_eq!(path, workspace.path().join("state/runtime-trace.jsonl"));
}

/// 测试：存储模式应正确解析已知值
///
/// # 验证内容
///
/// `storage_mode_from_config` 函数应将配置中的字符串值
/// 正确转换为 `RuntimeTraceStorageMode` 枚举：
///
/// | 配置值 | 枚举值 |
/// |--------|--------|
/// | "none" | `None` |
/// | "rolling" | `Rolling` |
/// | "full" | `Full` |
#[test]
fn storage_mode_parses_known_values() {
    let mut cfg = test_observability_config();

    // 测试 "none" 模式
    cfg.runtime_trace_mode = "none".into();
    assert_eq!(storage_mode_from_config(&cfg), RuntimeTraceStorageMode::None);

    // 测试 "rolling" 模式
    cfg.runtime_trace_mode = "rolling".into();
    assert_eq!(storage_mode_from_config(&cfg), RuntimeTraceStorageMode::Rolling);

    // 测试 "full" 模式
    cfg.runtime_trace_mode = "full".into();
    assert_eq!(storage_mode_from_config(&cfg), RuntimeTraceStorageMode::Full);
}

#[test]
fn storage_mode_trims_ignores_case_and_rejects_unknown_values() {
    assert_eq!(RuntimeTraceStorageMode::from_raw("  RoLlInG\t"), RuntimeTraceStorageMode::Rolling);
    assert_eq!(RuntimeTraceStorageMode::from_raw(" FULL "), RuntimeTraceStorageMode::Full);
    assert_eq!(RuntimeTraceStorageMode::from_raw("unsupported"), RuntimeTraceStorageMode::None);
    assert_eq!(RuntimeTraceStorageMode::from_raw("  "), RuntimeTraceStorageMode::None);
}

#[test]
fn storage_mode_unknown_and_blank_configs_fall_back_to_none() {
    let mut cfg = test_observability_config();
    cfg.runtime_trace_mode = "unsupported".to_string();
    assert_eq!(storage_mode_from_config(&cfg), RuntimeTraceStorageMode::None);

    cfg.runtime_trace_mode = "  ".to_string();
    assert_eq!(storage_mode_from_config(&cfg), RuntimeTraceStorageMode::None);
}

#[test]
fn resolve_trace_path_blank_value_uses_workspace_default() {
    let mut cfg = test_observability_config();
    cfg.runtime_trace_path = " \t ".to_string();
    let workspace = tempfile::tempdir().unwrap();

    assert_eq!(
        resolve_trace_path(&cfg, workspace.path()),
        workspace.path().join(DEFAULT_TRACE_REL_PATH)
    );
}

#[test]
fn resolve_trace_path_absolute_value_is_preserved() {
    let mut cfg = test_observability_config();
    let workspace = tempfile::tempdir().unwrap();
    let configured = workspace.path().join("absolute-trace.jsonl");
    cfg.runtime_trace_path = configured.to_string_lossy().into_owned();

    assert_eq!(resolve_trace_path(&cfg, Path::new("ignored-workspace")), configured);
}

#[test]
fn runtime_trace_event_serialization_omits_absent_optional_fields() {
    let value = serde_json::to_value(test_event("event-1", "test")).unwrap();

    for field in ["channel", "provider", "model", "turn_id", "success", "message"] {
        assert!(value.get(field).is_none(), "field {field} should be omitted");
    }
    assert_eq!(value["payload"], serde_json::json!({ "event_id": "event-1" }));
}

#[test]
fn runtime_trace_event_deserialization_defaults_missing_payload() {
    let event: RuntimeTraceEvent = serde_json::from_value(serde_json::json!({
        "id": "event-1",
        "timestamp": "2026-07-16T00:00:00Z",
        "event_type": "test"
    }))
    .unwrap();

    assert_eq!(event.payload, Value::Null);
    assert!(event.channel.is_none());
    assert!(event.message.is_none());
}

#[test]
fn none_mode_append_has_no_filesystem_side_effect() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("nested/trace.jsonl");
    let logger = RuntimeTraceLogger::new(RuntimeTraceStorageMode::None, 10, path.clone());

    logger.append(&test_event("event-1", "test")).unwrap();

    assert!(!path.exists());
    assert!(!tmp.path().join("nested").exists());
}

#[test]
fn full_mode_append_creates_parent_and_preserves_all_events() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("nested/trace.jsonl");
    let logger = RuntimeTraceLogger::new(RuntimeTraceStorageMode::Full, 1, path.clone());

    logger.append(&test_event("event-1", "first")).unwrap();
    logger.append(&test_event("event-2", "second")).unwrap();

    let events = load_events(&path, 10, None, None).unwrap();
    assert_eq!(
        events.iter().map(|event| event.id.as_str()).collect::<Vec<_>>(),
        ["event-2", "event-1"]
    );
}

#[test]
fn append_returns_error_when_trace_path_is_directory() {
    let tmp = tempfile::tempdir().unwrap();
    let logger =
        RuntimeTraceLogger::new(RuntimeTraceStorageMode::Full, 10, tmp.path().to_path_buf());

    assert!(logger.append(&test_event("event-1", "test")).is_err());
}

#[cfg(unix)]
#[test]
fn append_restricts_trace_permissions_to_owner() {
    use std::os::unix::fs::PermissionsExt;

    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("trace.jsonl");
    let logger = RuntimeTraceLogger::new(RuntimeTraceStorageMode::Full, 10, path.clone());

    logger.append(&test_event("event-1", "test")).unwrap();

    let mode = fs::metadata(path).unwrap().permissions().mode() & 0o777;
    assert_eq!(mode, 0o600);
}

/// 测试：滚动模式应仅保留最新的事件条目
///
/// # 验证内容
///
/// 在 `Rolling` 存储模式下，当日志条目数超过 `max_entries` 限制时，
/// 系统应自动删除最旧的条目，仅保留最新的 N 条记录。
///
/// # 测试场景
///
/// 1. 创建最大容量为 2 的滚动日志器
/// 2. 依次写入 5 条事件（id-0 到 id-4）
/// 3. 读取日志文件内容
///
/// # 预期结果
///
/// 仅保留最新的 2 条记录：
/// - `event-4`（最新）
/// - `event-3`（次新）
///
/// 较旧的 `event-0`、`event-1`、`event-2` 应被自动清理。
#[test]
fn rolling_mode_keeps_latest_entries() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("trace.jsonl");
    // 创建容量为 2 的滚动模式日志器
    let logger = RuntimeTraceLogger::new(RuntimeTraceStorageMode::Rolling, 2, path.clone());

    // 写入 5 条事件，超过容量限制
    for i in 0..5 {
        let event = RuntimeTraceEvent {
            id: format!("id-{i}"),
            timestamp: Utc::now().to_rfc3339(),
            event_type: "test".into(),
            channel: None,
            provider: None,
            model: None,
            turn_id: None,
            success: None,
            message: Some(format!("event-{i}")),
            payload: serde_json::json!({ "i": i }),
        };
        logger.append(&event).unwrap();
    }

    // 读取日志，验证仅保留最新的 2 条
    let events = load_events(&path, 10, None, None).unwrap();
    assert_eq!(events.len(), 2);
    // 验证顺序：最新的在前
    assert_eq!(events[0].message.as_deref(), Some("event-4"));
    assert_eq!(events[1].message.as_deref(), Some("event-3"));
}

#[test]
fn rolling_mode_zero_capacity_still_keeps_latest_event() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("trace.jsonl");
    let logger = RuntimeTraceLogger::new(RuntimeTraceStorageMode::Rolling, 0, path.clone());

    logger.append(&test_event("event-1", "test")).unwrap();
    logger.append(&test_event("event-2", "test")).unwrap();

    let events = load_events(&path, 10, None, None).unwrap();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].id, "event-2");
}

#[test]
fn rolling_trim_discards_blank_lines_when_rewriting() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("trace.jsonl");
    fs::write(&path, "old\n\n middle \n\t\n latest \n").unwrap();
    let logger = RuntimeTraceLogger::new(RuntimeTraceStorageMode::Rolling, 2, path.clone());

    logger.trim_to_last_entries().unwrap();

    assert_eq!(fs::read_to_string(path).unwrap(), "middle\nlatest\n");
}

#[test]
fn load_events_missing_path_returns_empty() {
    let tmp = tempfile::tempdir().unwrap();

    assert!(load_events(&tmp.path().join("missing.jsonl"), 10, None, None).unwrap().is_empty());
}

#[test]
fn load_events_skips_blank_and_malformed_lines_and_returns_latest_first() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("trace.jsonl");
    let older = serde_json::to_string(&test_event("older", "test")).unwrap();
    let newer = serde_json::to_string(&test_event("newer", "test")).unwrap();
    fs::write(&path, format!("\n{older}\n{{not-json}}\n\n{newer}\n")).unwrap();

    let events = load_events(&path, 10, None, None).unwrap();

    assert_eq!(
        events.iter().map(|event| event.id.as_str()).collect::<Vec<_>>(),
        ["newer", "older"]
    );
}

#[test]
fn load_events_filters_event_type_case_insensitively() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("trace.jsonl");
    let logger = RuntimeTraceLogger::new(RuntimeTraceStorageMode::Full, 10, path.clone());
    logger.append(&test_event("event-1", "tool_call")).unwrap();
    logger.append(&test_event("event-2", "llm_response")).unwrap();

    let events = load_events(&path, 10, Some("  TOOL_CALL "), None).unwrap();

    assert_eq!(events.len(), 1);
    assert_eq!(events[0].id, "event-1");
}

#[test]
fn load_events_contains_searches_supported_fields_case_insensitively() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("trace.jsonl");
    let logger = RuntimeTraceLogger::new(RuntimeTraceStorageMode::Full, 10, path.clone());
    logger.append(&test_event("empty-fields", "other")).unwrap();

    let mut searchable = test_event("searchable", "trace_event");
    searchable.channel = Some("Slack".to_string());
    searchable.provider = Some("OpenRouter".to_string());
    searchable.model = Some("Model-X".to_string());
    searchable.message = Some("Diagnostic summary".to_string());
    searchable.payload = serde_json::json!({ "detail": "payload-token" });
    logger.append(&searchable).unwrap();

    for needle in
        [" TRACE_EVENT ", " DIAGNOSTIC ", " PAYLOAD-TOKEN ", " SLACK ", " OPENROUTER ", " MODEL-X "]
    {
        let events = load_events(&path, 10, None, Some(needle)).unwrap();
        assert_eq!(events.len(), 1, "needle {needle} should match");
        assert_eq!(events[0].id, "searchable");
    }
}

#[test]
fn load_events_blank_filters_do_not_exclude_events() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("trace.jsonl");
    let logger = RuntimeTraceLogger::new(RuntimeTraceStorageMode::Full, 10, path.clone());
    logger.append(&test_event("event-1", "first")).unwrap();
    logger.append(&test_event("event-2", "second")).unwrap();

    let events = load_events(&path, 10, Some(" \t "), Some("  ")).unwrap();

    assert_eq!(events.len(), 2);
}

#[test]
fn load_events_zero_limit_returns_no_events() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("trace.jsonl");
    let logger = RuntimeTraceLogger::new(RuntimeTraceStorageMode::Full, 10, path.clone());
    logger.append(&test_event("event-1", "test")).unwrap();

    assert!(load_events(&path, 0, None, None).unwrap().is_empty());
}

#[test]
fn load_events_positive_limit_keeps_latest_events() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("trace.jsonl");
    let logger = RuntimeTraceLogger::new(RuntimeTraceStorageMode::Full, 10, path.clone());
    logger.append(&test_event("event-1", "test")).unwrap();
    logger.append(&test_event("event-2", "test")).unwrap();
    logger.append(&test_event("event-3", "test")).unwrap();

    let events = load_events(&path, 2, None, None).unwrap();

    assert_eq!(
        events.iter().map(|event| event.id.as_str()).collect::<Vec<_>>(),
        ["event-3", "event-2"]
    );
}

#[test]
fn load_events_returns_read_error_for_directory_path() {
    let tmp = tempfile::tempdir().unwrap();

    assert!(load_events(tmp.path(), 10, None, None).is_err());
}

/// 测试：按 ID 查找事件应返回匹配结果
///
/// # 验证内容
///
/// `find_event_by_id` 函数应能根据事件 ID 在日志文件中
/// 精确查找并返回匹配的事件记录。
///
/// # 测试场景
///
/// 1. 创建 Full 模式的日志器（保留所有记录）
/// 2. 写入一条具有特定 ID 的事件
/// 3. 使用该 ID 进行查找
///
/// # 预期结果
///
/// 查找结果应为 `Some(event)`，且事件的 ID 与目标 ID 一致。
#[test]
fn find_event_by_id_returns_match() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("trace.jsonl");
    // 创建 Full 模式日志器，保留所有记录
    let logger = RuntimeTraceLogger::new(RuntimeTraceStorageMode::Full, 100, path.clone());

    // 创建并写入目标事件
    let target_id = "target-event";
    let event = RuntimeTraceEvent {
        id: target_id.into(),
        timestamp: Utc::now().to_rfc3339(),
        event_type: "tool_call_result".into(),
        channel: Some("telegram".into()),
        provider: Some("openrouter".into()),
        model: Some("x".into()),
        turn_id: Some("turn-1".into()),
        success: Some(false),
        message: Some("boom".into()),
        payload: serde_json::json!({ "error": "boom" }),
    };
    logger.append(&event).unwrap();

    // 按 ID 查找并验证
    let found = find_event_by_id(&path, target_id).unwrap();
    assert!(found.is_some());
    assert_eq!(found.unwrap().id, target_id);
}

#[test]
fn find_event_by_id_missing_path_returns_none() {
    let tmp = tempfile::tempdir().unwrap();

    assert!(find_event_by_id(&tmp.path().join("missing.jsonl"), "target").unwrap().is_none());
}

#[test]
fn find_event_by_id_skips_invalid_lines_and_returns_latest_duplicate() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("trace.jsonl");
    let mut older = test_event("target", "test");
    older.message = Some("older".to_string());
    let mut newer = test_event("target", "test");
    newer.message = Some("newer".to_string());
    let other = test_event("other", "test");
    fs::write(
        &path,
        format!(
            "{}\n{}\n{}\n\n{{not-json}}\n",
            serde_json::to_string(&older).unwrap(),
            serde_json::to_string(&other).unwrap(),
            serde_json::to_string(&newer).unwrap()
        ),
    )
    .unwrap();

    let found = find_event_by_id(&path, "target").unwrap().unwrap();

    assert_eq!(found.message.as_deref(), Some("newer"));
}

#[test]
fn find_event_by_id_returns_none_when_no_valid_event_matches() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("trace.jsonl");
    let other = serde_json::to_string(&test_event("other", "test")).unwrap();
    fs::write(&path, format!("{other}\n{{not-json}}\n")).unwrap();

    assert!(find_event_by_id(&path, "target").unwrap().is_none());
}

#[test]
fn find_event_by_id_returns_read_error_for_directory_path() {
    let tmp = tempfile::tempdir().unwrap();

    assert!(find_event_by_id(tmp.path(), "target").is_err());
}

#[test]
fn configured_logger_records_complete_event_and_none_disables_writes() {
    let _guard = TRACE_LOGGER_TEST_LOCK.lock().unwrap_or_else(|error| error.into_inner());
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("nested/runtime-trace.jsonl");
    let mut cfg = test_observability_config();
    cfg.runtime_trace_mode = "full".to_string();
    cfg.runtime_trace_path = path.to_string_lossy().into_owned();

    init_from_config(&cfg, tmp.path());
    record_event(
        "tool_call",
        Some("slack"),
        Some("openrouter"),
        Some("claude-sonnet"),
        Some("turn-1"),
        Some(true),
        Some("completed"),
        serde_json::json!({ "tool": "shell" }),
    );

    let events = load_events(&path, 10, None, None).unwrap();
    assert_eq!(events.len(), 1);
    let event = &events[0];
    assert!(!event.id.is_empty());
    assert!(!event.timestamp.is_empty());
    assert_eq!(event.event_type, "tool_call");
    assert_eq!(event.channel.as_deref(), Some("slack"));
    assert_eq!(event.provider.as_deref(), Some("openrouter"));
    assert_eq!(event.model.as_deref(), Some("claude-sonnet"));
    assert_eq!(event.turn_id.as_deref(), Some("turn-1"));
    assert_eq!(event.success, Some(true));
    assert_eq!(event.message.as_deref(), Some("completed"));
    assert_eq!(event.payload, serde_json::json!({ "tool": "shell" }));

    cfg.runtime_trace_path = tmp.path().to_string_lossy().into_owned();
    init_from_config(&cfg, tmp.path());
    record_event("write_error", None, None, None, None, Some(false), None, Value::Null);
    assert!(tmp.path().is_dir());

    cfg.runtime_trace_mode = "none".to_string();
    cfg.runtime_trace_path = path.to_string_lossy().into_owned();
    init_from_config(&cfg, tmp.path());
    record_event("ignored", None, None, None, None, None, None, Value::Null);

    assert_eq!(load_events(&path, 10, None, None).unwrap().len(), 1);
}
