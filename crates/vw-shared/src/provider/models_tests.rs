//! 原始目录模型元数据的反序列化测试。

use super::Model;
use serde_json::{Value, json};

#[test]
fn catalog_modelsdev_parse_distinguishes_missing_reasoning_from_false() {
    let missing: Model = serde_json::from_value(json!({})).unwrap();
    let disabled: Model = serde_json::from_value(json!({"reasoning": false})).unwrap();
    let missing = serde_json::to_value(missing).unwrap();
    let disabled = serde_json::to_value(disabled).unwrap();
    assert_eq!(missing["reasoning"], Value::Null);
    assert_eq!(disabled["reasoning"], false);
    for key in ["reasoning_options", "modalities", "cost", "canonical_model_id"] {
        assert_eq!(missing[key], Value::Null, "{key}");
    }
}

#[test]
fn catalog_modelsdev_parse_distinguishes_partial_cost_from_free_cost() {
    let model: Model = serde_json::from_value(json!({
        "cost": {"input": 0.0, "cache_read": 0.25},
        "reasoning_options": [{"type": "effort", "values": ["low", "high"]}],
        "modalities": {"input": ["text", "image"], "output": ["text"]},
        "limit": {"context": 65536},
        "status": "preview"
    }))
    .unwrap();
    let raw = serde_json::to_value(model).unwrap();
    assert_eq!(raw["cost"]["input"], 0.0);
    assert_eq!(raw["cost"]["output"], Value::Null);
    assert_eq!(raw["cost"]["cache_read"], 0.25);
    assert_eq!(raw["reasoning_options"][0]["values"], json!(["low", "high"]));
    assert_eq!(raw["modalities"]["input"], json!(["text", "image"]));
    assert_eq!(raw["modalities"]["output"], json!(["text"]));
    assert_eq!(raw["limit"]["context"], 65536);
    assert_eq!(raw["status"], "preview");
}
