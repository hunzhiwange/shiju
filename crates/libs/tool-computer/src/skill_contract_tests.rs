//! Plugin examples must use the real closed protocol vocabulary.
use serde_json::Value;
use vw_api_computer_control::{ComputerControlAction, ComputerControlTarget};

const SKILL: &str = include_str!("../../../../plugins/shiju-computer-control/skills/computer-control/SKILL.md");

#[test]
fn cc_runtime_07_skill_examples_use_real_requests() {
    let mut count = 0;
    for block in SKILL.split("```json\n").skip(1) {
        let example = block.split("```").next().unwrap();
        let value: Value = serde_json::from_str(example).unwrap();
        let target: ComputerControlTarget = serde_json::from_value(value["target"].clone()).unwrap();
        assert!(matches!(target, ComputerControlTarget::NativeApp { .. }));
        let action: ComputerControlAction = serde_json::from_value(value["action"].clone()).unwrap();
        assert!(matches!(action, ComputerControlAction::NativeObserve { .. }
            | ComputerControlAction::NativeInput { .. } | ComputerControlAction::Stop));
        assert_eq!(value.as_object().unwrap().len(), 2);
        count += 1;
    }
    assert_eq!(count, 5);
}

#[test]
fn cc_runtime_07_manifest_exposes_packaged_skill() {
    let manifest: Value = serde_json::from_str(include_str!(
        "../../../../plugins/shiju-computer-control/.codex-plugin/plugin.json"
    )).unwrap();
    assert_eq!(manifest["name"], "shiju-computer-control");
    assert_eq!(manifest["skills"], "./skills");
    assert!(SKILL.starts_with("---\nname: computer-control\n"));
    assert!(manifest.get("mcpServers").is_none());
}
