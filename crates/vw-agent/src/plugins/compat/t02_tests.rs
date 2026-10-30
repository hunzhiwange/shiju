use crate::plugins::load_plugin_manifest;
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn fixture_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("testdata/plugin_compat/t02")
}

fn fixture(name: &str) -> Value {
    serde_json::from_slice(&fs::read(fixture_root().join(name)).expect("fixture bytes"))
        .expect("fixture JSON")
}

fn check(baseline: &Path, evidence: &Path) -> Output {
    let script =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../scripts/plugin_compat/check_evidence.py");
    Command::new("python3")
        .arg(script)
        .arg("--baseline")
        .arg(baseline)
        .arg("--evidence")
        .arg(evidence)
        .output()
        .expect("start real evidence checker")
}

#[test]
fn codex_plugin_compat_t02_rejects_incomplete_evidence() {
    let state = tempfile::tempdir().expect("evidence state");
    let mut evidence = fixture("complete.json");
    evidence["items"][0].as_object_mut().expect("item").remove("green");
    let path = state.path().join("incomplete.json");
    fs::write(&path, serde_json::to_vec(&evidence).expect("evidence JSON"))
        .expect("write evidence");
    let output = check(&fixture_root().join("baseline.json"), &path);
    assert_eq!(
        output.status.code(),
        fixture("expected.json")["incomplete_exit"].as_i64().map(|v| v as i32)
    );
    assert!(String::from_utf8_lossy(&output.stderr).contains("green"));
}

#[test]
fn codex_plugin_compat_t02_rejects_invalid_input() {
    let state = tempfile::tempdir().expect("invalid state");
    let baseline = state.path().join("baseline.json");
    fs::write(&baseline, b"{invalid").expect("invalid JSON");
    let output = check(&baseline, &fixture_root().join("complete.json"));
    assert_eq!(
        output.status.code(),
        fixture("expected.json")["invalid_exit"].as_i64().map(|v| v as i32)
    );
    assert!(String::from_utf8_lossy(&output.stderr).contains("invalid JSON"));
    assert_eq!(fs::read(&baseline).expect("unchanged input"), b"{invalid");
}

#[test]
fn codex_plugin_compat_t02_preserves_isolation() {
    let state = tempfile::tempdir().expect("isolated workspace");
    let package = state.path().join("package");
    let relative = ".vibewindow-plugin/plugin.json";
    fs::create_dir_all(package.join(".vibewindow-plugin")).expect("manifest directory");
    let bytes = fs::read(fixture_root().join("package").join(relative)).expect("source package");
    fs::write(package.join(relative), &bytes).expect("copy original package");
    let manifest = load_plugin_manifest(&package).expect("real plugin loader");
    let expected = fixture("expected.json");
    assert_eq!(manifest.name, expected["name"].as_str().expect("expected name"));
    assert_eq!(manifest.version.as_deref(), expected["version"].as_str());
    for turn in ["first", "second"] {
        let turn_dir = state.path().join(turn);
        fs::create_dir(&turn_dir).expect("separate evidence state");
        let evidence_path = turn_dir.join("evidence.json");
        let evidence = fs::read(fixture_root().join("complete.json")).expect("complete evidence");
        fs::write(&evidence_path, &evidence).expect("write evidence outside package");
        let output = check(&fixture_root().join("baseline.json"), &evidence_path);
        assert_eq!(
            output.status.code(),
            expected["complete_exit"].as_i64().map(|v| v as i32),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(fs::read(&evidence_path).expect("evidence unchanged"), evidence);
    }
    assert_eq!(fs::read(package.join(relative)).expect("package unchanged"), bytes);
    assert_eq!(fs::read_dir(&package).expect("package entries").count(), 1);
    assert!(!package.join("evidence.json").exists());
}
