use super::closure_tests::{fixtures, summarize};
use serde_json::{Value, json};
use std::fs;

#[test]
fn codex_plugin_compat_t66_rejects_missing_in_scope_evidence() {
    let temp = tempfile::tempdir().unwrap();
    let sources = temp.path().join("sources");
    fs::create_dir(&sources).unwrap();
    let output = temp.path().join("summary.json");
    let result = summarize(&fixtures().join("baseline.json"), &sources, &output);
    assert!(result.status.success(), "{}", String::from_utf8_lossy(&result.stderr));
    let summary: Value = serde_json::from_slice(&fs::read(&output).unwrap()).unwrap();
    assert_eq!(summary["items"][0]["conclusion"], "missing");
    assert_eq!(summary["items"][0]["id"], "acceptance.t02");
    assert_eq!(summary["full_compatibility_claimed"], false);
    let mut waived: Value =
        serde_json::from_slice(&fs::read(fixtures().join("complete.json")).unwrap()).unwrap();
    waived["items"][0]["platforms"]["verified"] = json!(["macos"]);
    waived["status"] = json!("waived_by_user");
    let bytes = serde_json::to_vec(&waived).unwrap();
    fs::write(sources.join("t02.json"), &bytes).unwrap();
    assert!(summarize(&fixtures().join("baseline.json"), &sources, &output).status.success());
    let summary: Value = serde_json::from_slice(&fs::read(&output).unwrap()).unwrap();
    assert_eq!(summary["items"][0]["conclusion"], "unverified");
    assert!(
        summary["items"][0]["blocking_reasons"][0]
            .as_str()
            .unwrap()
            .contains("unverified platform")
    );
    assert_eq!(summary["waivers_are_passing_evidence"], false);
    assert_eq!(fs::read(sources.join("t02.json")).unwrap(), bytes);
}

#[test]
fn codex_plugin_compat_t66_rejects_invalid_input() {
    let temp = tempfile::tempdir().unwrap();
    let bad = temp.path().join("bad.json");
    fs::write(&bad, br#"{"schema_version":1,"items":[],"items":[]}"#).unwrap();
    let output = temp.path().join("summary.json");
    fs::write(&output, b"existing report").unwrap();
    let result = summarize(&bad, temp.path(), &output);
    assert!(!result.status.success(), "duplicate keys must fail");
    assert_eq!(fs::read(&output).unwrap(), b"existing report");
}

#[test]
fn codex_plugin_compat_t66_preserves_isolation() {
    let temp = tempfile::tempdir().unwrap();
    let sources = temp.path().join("package");
    fs::create_dir(&sources).unwrap();
    let bytes = fs::read(fixtures().join("complete.json")).unwrap();
    fs::write(sources.join("t02.json"), &bytes).unwrap();
    let output = temp.path().join("summary.json");
    let baseline = fixtures().join("baseline.json");
    assert!(summarize(&baseline, &sources, &output).status.success());
    let summary: Value = serde_json::from_slice(&fs::read(&output).unwrap()).unwrap();
    assert_eq!(summary["items"][0]["conclusion"], "verified");
    assert!(
        summarize(&baseline, &sources, &sources.join("t02.json"))
            .status
            .code()
            .is_some_and(|c| c != 0)
    );
    assert_eq!(fs::read(sources.join("t02.json")).unwrap(), bytes);
    let mut other: Value = serde_json::from_slice(&bytes).unwrap();
    other["items"][0]["id"] = json!("other.source");
    fs::write(sources.join("t02.json"), serde_json::to_vec(&other).unwrap()).unwrap();
    assert!(summarize(&baseline, &sources, &output).status.success());
    let next: Value = serde_json::from_slice(&fs::read(&output).unwrap()).unwrap();
    assert_eq!(next["items"][0]["conclusion"], "missing");
    assert_eq!(next["unmapped_ids"][0], "other.source");
}
