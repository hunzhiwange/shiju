use std::{fs, path::Path};
use serde_json::{Value, json};
use vw_agent_apps::usage::pi_children::project_files;

fn write(path: &Path, id: &str, entries: &[Value]) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    let mut text = format!("{}\n", json!({"type":"session","id":id,"timestamp":1700000000}));
    for entry in entries { text.push_str(&format!("{entry}\n")); }
    fs::write(path, text).unwrap();
}
fn assistant(input: u64, time: i64) -> Value {
    json!({"type":"message","id":"request","timestamp":time,
        "message":{"role":"assistant","usage":{"input":input}}})
}
fn projection(child: &Path, time: i64) -> Value {
    json!({"type":"message","id":"projection","timestamp":time,
        "message":{"role":"toolResult","toolName":"subagent","usage":{"input":10},
            "details":{"results":[{"sessionFile":child}]}}})
}

#[test]
fn usage17_pi_children_child_projection_is_counted_once() {
    let dir = tempfile::tempdir().unwrap();
    let parent = dir.path().join("parent.jsonl");
    let child = dir.path().join("parent/run/child.jsonl");
    let replica = dir.path().join("replica.jsonl");
    let entry = projection(&child, 1700000030);
    write(&parent, "parent", &[entry.clone()]);
    write(&replica, "parent", &[entry]);
    write(&child, "child", &[assistant(10, 1700000025)]);
    let files = vec![parent, child, replica];
    let result = project_files(&files, 1700000020).unwrap();
    assert_eq!(result.messages.len(), 1);
    assert_eq!(result.messages[0].session_id, "child");
    assert_eq!(result.messages[0].tokens.input, Some(10));
}

#[test]
fn usage17_pi_children_cutoff_and_rewrite_replace_the_projection() {
    let dir = tempfile::tempdir().unwrap();
    let parent = dir.path().join("parent.jsonl");
    let child = dir.path().join("parent/run/child.jsonl");
    write(&parent, "parent", &[projection(&child, 1700000010)]);
    write(&child, "child", &[assistant(10, 1700000005), assistant(15, 1700000030)]);
    let files = vec![parent, child.clone()];
    let first = project_files(&files, 1700000020).unwrap();
    assert_eq!(first.messages.len(), 2);
    assert_eq!(first.messages.iter().filter_map(|m| m.tokens.input).sum::<u64>(), 25);
    write(&child, "child", &[assistant(19, 1700000030)]);
    let second = project_files(&files, 1700000020).unwrap();
    assert_ne!(first.revisions[&child], second.revisions[&child]);
    assert_eq!(second.messages.iter().filter_map(|m| m.tokens.input).sum::<u64>(), 29);
}

#[test]
fn usage17_pi_children_official_requests_override_aggregate() {
    let dir = tempfile::tempdir().unwrap();
    let parent = dir.path().join("parent.jsonl");
    let child = assistant(7, 1700000030)["message"].clone();
    write(&parent, "parent", &[json!({"type":"message","id":"tool","timestamp":1700000030,
        "message":{"role":"toolResult","toolName":"subagent","details":{
            "mode":"single","results":[{"messages":[child],"usage":{"input":99,"cost":1}}]}}})]);
    let result = project_files(&[parent], 1700000020).unwrap();
    assert_eq!(result.messages.len(), 1);
    assert_eq!(result.messages[0].tokens.input, Some(7));
}

#[test]
fn usage17_pi_children_missing_child_keeps_projection() {
    let dir = tempfile::tempdir().unwrap();
    let parent = dir.path().join("parent.jsonl");
    write(&parent, "parent", &[projection(&dir.path().join("missing.jsonl"), 1700000030)]);
    assert_eq!(project_files(&[parent], 1700000020).unwrap().messages.len(), 1);
}
