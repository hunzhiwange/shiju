//! Structural capability boundary for the editable system template.
use anyhow::{Result, ensure};
use serde_json::Value;

pub const DEFAULT_YAML: &str = include_str!("../../../resources/workflows/cloud_video_edit.yaml");

/// Business prompts and explicit model choices are editable. Execution capabilities,
/// selectors and serial delivery gates must remain in the supported graph contract.
pub(super) fn validate(source: &str) -> Result<()> {
    let mut actual: Value = serde_yaml::from_str(source)?;
    let mut expected: Value = serde_yaml::from_str(DEFAULT_YAML)?;
    fn normalize(root: &mut Value) -> Result<()> {
        let options: vw_api_types::workflow::VideoEditTemplateOptions =
            match root["workflow"].get("video_edit") {
                Some(value) => serde_json::from_value(value.clone())?,
                None => Default::default(),
            };
        let limit = options.max_parallel();
        // The desktop graph editor adds presentation state, not executable inputs.
        if let Some(workflow) = root["workflow"].as_object_mut() {
            for key in ["environment_variables", "conversation_variables"] {
                if workflow.get(key).and_then(Value::as_array).is_some_and(Vec::is_empty) {
                    workflow.remove(key);
                }
            }
        }
        if let Some(graph) = root["workflow"]["graph"].as_object_mut() {
            graph.remove("viewport");
        }
        if let Some(edges) = root["workflow"]["graph"]["edges"].as_array_mut() {
            for edge in edges {
                if let Some(object) = edge.as_object_mut() {
                    for key in ["id", "selected", "zIndex"] {
                        object.remove(key);
                    }
                    if object.get("type").and_then(Value::as_str) == Some("custom") {
                        object.remove("type");
                    }
                    if let Some(data) = object.get_mut("data").and_then(Value::as_object_mut) {
                        data.remove("sourceType");
                        data.remove("targetType");
                        if data.is_empty() {
                            object.remove("data");
                        }
                    }
                }
            }
        }
        let nodes = root["workflow"]["graph"]["nodes"]
            .as_array_mut()
            .ok_or_else(|| anyhow::anyhow!("missing nodes"))?;
        for node in nodes {
            strip_presentation(node);
            let data = &mut node["data"];
            if data["type"] == "llm" {
                normalize_llm(data)?;
            }
            if data["type"] == "iteration" {
                let n = data["max_parallel"].as_u64().unwrap_or(0);
                ensure!(n >= 1 && n <= u64::from(limit), "iteration exceeds template budget");
                data["max_parallel"] = Value::from(5);
                let children = data["graph"]["nodes"]
                    .as_array_mut()
                    .ok_or_else(|| anyhow::anyhow!("missing iteration graph"))?;
                for child in children {
                    strip_presentation(child);
                    if child["data"]["type"] == "llm" {
                        normalize_llm(&mut child["data"])?;
                    }
                }
            }
        }
        root["workflow"]["video_edit"]["max_parallel"] = Value::from(5);
        // App metadata is presentation only, never executed.
        root.as_object_mut().ok_or_else(|| anyhow::anyhow!("invalid template"))?.remove("app");
        Ok(())
    }
    fn strip_presentation(node: &mut Value) {
        if let Some(object) = node.as_object_mut() {
            for key in [
                "position",
                "positionAbsolute",
                "width",
                "height",
                "selected",
                "sourcePosition",
                "targetPosition",
                "zIndex",
            ] {
                object.remove(key);
            }
        }
        if let Some(data) = node.get_mut("data").and_then(Value::as_object_mut) {
            data.remove("title");
            data.remove("desc");
            data.remove("selected");
        }
    }
    fn normalize_llm(data: &mut Value) -> Result<()> {
        let prompts =
            data["prompt_template"].as_array().ok_or_else(|| anyhow::anyhow!("missing prompts"))?;
        ensure!(
            !prompts.is_empty()
                && prompts.iter().all(|p| matches!(
                    p["role"].as_str(),
                    Some("user" | "system" | "assistant" | "developer")
                ) && p["text"]
                    .as_str()
                    .is_some_and(|s| !s.trim().is_empty())),
            "invalid prompts"
        );
        data["prompt_template"] = Value::Null;
        data.as_object_mut().ok_or_else(|| anyhow::anyhow!("invalid model node"))?.remove("model");
        Ok(())
    }
    normalize(&mut actual)?;
    normalize(&mut expected)?;
    ensure!(
        actual == expected,
        "unsupported video template graph: capability, order or delivery gate changed"
    );
    Ok(())
}

#[cfg(test)]
#[path = "entry_presentation_tests.rs"]
mod presentation_tests;
