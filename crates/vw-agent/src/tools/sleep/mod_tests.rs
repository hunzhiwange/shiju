//! 验证休眠时长解析的优先级、范围限制及工具规格。

use super::{Args, MAX_SLEEP_MS, SleepTool};
use crate::app::agent::tools::Tool;
use serde_json::json;

#[test]
fn resolve_duration_prefers_milliseconds_over_seconds() {
    let args = Args { duration_ms: Some(25), seconds: Some(9.0) };

    assert_eq!(SleepTool::resolve_duration(&args).unwrap(), 25);
}

#[test]
fn resolve_duration_converts_seconds_and_rounds() {
    let args = Args { duration_ms: None, seconds: Some(1.2345) };

    assert_eq!(SleepTool::resolve_duration(&args).unwrap(), 1235);
}

#[test]
fn resolve_duration_rejects_missing_non_finite_negative_and_too_large_values() {
    assert!(SleepTool::resolve_duration(&Args { duration_ms: None, seconds: None }).is_err());
    assert!(
        SleepTool::resolve_duration(&Args { duration_ms: None, seconds: Some(f64::NAN) }).is_err()
    );
    assert!(SleepTool::resolve_duration(&Args { duration_ms: None, seconds: Some(-1.0) }).is_err());
    assert!(
        SleepTool::resolve_duration(&Args { duration_ms: Some(MAX_SLEEP_MS + 1), seconds: None })
            .is_err()
    );
}

#[test]
fn schema_and_spec_expose_sleep_metadata() {
    let tool = SleepTool::new();
    let schema = tool.parameters_schema();
    let spec = tool.spec();

    assert_eq!(tool.name(), "Sleep");
    assert_eq!(schema["additionalProperties"], false);
    assert_eq!(schema["properties"]["duration_ms"]["maximum"], json!(MAX_SLEEP_MS));
    assert_eq!(spec.name, "Sleep");
    assert_eq!(spec.display_name, "Sleep");
    assert!(spec.aliases.iter().any(|alias| alias == "sleep"));
    assert!(spec.read_only);
    assert!(!spec.destructive);
    assert!(!spec.concurrency_safe);
    assert!(!spec.requires_user_interaction);
    assert!(spec.strict);
}
