use std::io::Write;
use vw_agent_apps::usage::pi::{parse_file, parse_reader};
use vw_api_types::usage::{UsageCostSource, UsageProvider};

const HEADER: &str = "{\"type\":\"session\",\"id\":\"session-a\",\"timestamp\":\"2023-11-14T22:13:20Z\"}\n";

fn row(input: u64) -> String {
    format!(r#"{{"type":"message","id":"stable","timestamp":"2023-11-14T22:13:21Z","message":{{"role":"assistant","model":"requested","responseModel":"actual","usage":{{"input":{input},"output":2,"cacheRead":5,"cost":{{"total":0.000000000000000000000000001}}}}}}}}"#) + "\n"
}

#[test]
fn usage17_pi_revised_main_session_keeps_stable_identity() {
    let mut file = tempfile::NamedTempFile::new().unwrap();
    write!(file, "{HEADER}{}", row(10)).unwrap();
    let first = parse_file(file.path()).unwrap();
    std::fs::write(file.path(), format!("{HEADER}{}", row(11))).unwrap();
    let revised = parse_file(file.path()).unwrap();
    assert_ne!(first.revision, revised.revision);
    assert_eq!(first.messages.len(), 1);
    assert_eq!(revised.messages.len(), 1);
    assert_eq!(first.messages[0].request_id, revised.messages[0].request_id);
    assert_eq!(revised.messages[0].tokens.input, Some(11));
    assert_eq!(revised.messages[0].cost.source, UsageCostSource::Native);
    assert_eq!(revised.messages[0].cost.amount_usd.as_deref(), Some("0.000000000000000000000000001"));
    assert_eq!(revised.messages[0].provider, UsageProvider::Unknown);
    assert_eq!(revised.messages[0].request_model, "requested");
    assert_eq!(revised.messages[0].response_model, "actual");
}

#[test]
fn usage17_pi_defers_partial_tail_and_ignores_nested_snapshots() {
    let partial = row(10);
    let bytes = format!("{HEADER}{}", &partial[..partial.len() / 2]);
    let (messages, incomplete) = parse_reader(bytes.as_bytes()).unwrap();
    assert!(messages.is_empty());
    assert!(incomplete);
    let bytes = format!("{HEADER}{}{}", row(10), row(11));
    let (messages, incomplete) = parse_reader(bytes.as_bytes()).unwrap();
    assert!(!incomplete);
    assert_eq!(messages.len(), 1);
    let nested = format!("{HEADER}{{\"type\":\"model_usage\",\"usage\":{{\"input\":99}}}}\n{{\"type\":\"custom\",\"details\":{{\"usage\":{{\"input\":99}}}}}}\n");
    assert!(parse_reader(nested.as_bytes()).unwrap().0.is_empty());
}

#[test]
fn usage17_pi_bounds_labels_and_rejects_invalid_header() {
    assert!(parse_reader(b"{\"type\":\"message\"}\n".as_slice()).is_err());
    let mut entry: serde_json::Value = serde_json::from_str(&row(10)).unwrap();
    entry["message"]["model"] = serde_json::json!("界".repeat(600));
    entry["timestamp"] = serde_json::json!(i64::MAX);
    let bytes = format!("{HEADER}{entry}\n");
    let message = &parse_reader(bytes.as_bytes()).unwrap().0[0];
    assert!(message.request_model.len() <= 512);
    assert_eq!(message.timestamp.unwrap().0, 1_700_000_000);
}
