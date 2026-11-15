//! Existing-node fixtures cover common Dify fields and preserve vendor extensions.

use super::*;

#[test]
fn existing_nodes_cover_official_common_fields_without_data_loss() {
    let cases = [
        ("start", "variables: []\nvendor_root: keep\n"),
        (
            "llm",
            "model:\n  provider: openai\n  name: gpt\n  mode: chat\n  completion_params:\n    temperature: 0.3\nstructured_output:\n  schema:\n    type: object\nmemory:\n  window:\n    enabled: true\n    size: 7\nvision:\n  enabled: true\n  variable_selector: [start, image]\nprompt_template: []\ncontext: { enabled: false, variable_selector: [] }\nvendor_root: keep\n",
        ),
        ("answer", "answer: ok\nvendor_root: keep\n"),
        ("if-else", "cases: []\nvendor_root: keep\n"),
        (
            "knowledge-retrieval",
            "query_variable_selector: [start, query]\ndataset_ids: [kb]\nretrieval_mode: multiple\nmultiple_retrieval_config:\n  top_k: 4\n  reranking_enable: true\n  reranking_model:\n    provider: cohere\n    model: rerank-v3\n  metadata_filtering_mode: automatic\n  metadata_model_config:\n    provider: openai\n    name: gpt\nsingle_retrieval_config:\n  model: { provider: openai, name: gpt, mode: chat }\nvendor_root: keep\n",
        ),
        (
            "tool",
            "provider_id: builtin\nprovider_type: builtin\nprovider_name: builtin\ntool_name: search\ntool_parameters: { query: { type: mixed, value: hi } }\ntool_configurations: {}\nvendor_root: keep\n",
        ),
        (
            "agent",
            "agent_strategy_provider_name: builtin\nagent_strategy_name: function_calling\nagent_strategy_label: Function Calling\nagent_parameters: { instruction: { type: constant, value: help } }\noutput_schema: { type: object }\nmemory: { window: { enabled: true, size: 5 } }\nvendor_root: keep\n",
        ),
        (
            "code",
            "variables: []\ncode_language: python3\ncode: pass\noutputs: {}\nretry_config: { retry_enabled: true, max_retries: 2, retry_interval: 100 }\nerror_strategy: fail-branch\nvendor_root: keep\n",
        ),
    ];

    for (block_type, raw) in cases {
        let mut draft = build_node_visual_draft(block_type, raw)
            .unwrap_or_else(|error| panic!("{block_type} draft: {error}"))
            .expect("existing node should have a visual draft");
        match &mut draft {
            WorkflowNodeVisualDraft::Llm {
                completion_params_editor,
                structured_output_editor,
                memory_editor,
                vision_selector_input,
                ..
            } => {
                *completion_params_editor = text_editor::Content::with_text("temperature: 0.8");
                *structured_output_editor =
                    text_editor::Content::with_text("schema:\n  type: object");
                *memory_editor =
                    text_editor::Content::with_text("window:\n  enabled: true\n  size: 9");
                *vision_selector_input = "upload.image".to_string();
            }
            WorkflowNodeVisualDraft::KnowledgeRetrieval {
                reranking_model_editor,
                metadata_filtering_mode,
                metadata_model_config_editor,
                ..
            } => {
                *reranking_model_editor =
                    text_editor::Content::with_text("provider: cohere\nmodel: rerank-v4");
                *metadata_filtering_mode = "manual".to_string();
                *metadata_model_config_editor =
                    text_editor::Content::with_text("provider: openai\nname: gpt-4.1");
            }
            _ => {}
        }
        let synced = apply_visual_draft_to_yaml(block_type, raw, Some(&draft))
            .unwrap_or_else(|error| panic!("{block_type} sync: {error}"));
        let value: Value = serde_yaml::from_str(&synced).expect("synced YAML");
        let map = value.as_mapping().expect("synced mapping");
        assert_eq!(mapping_value(map, "vendor_root").and_then(Value::as_str), Some("keep"));

        if block_type == "llm" {
            let model = mapping_value(map, "model").and_then(Value::as_mapping).unwrap();
            assert_eq!(
                mapping_value(model, "completion_params")
                    .and_then(Value::as_mapping)
                    .and_then(|value| mapping_value(value, "temperature"))
                    .and_then(Value::as_f64),
                Some(0.8)
            );
            assert!(mapping_value(map, "structured_output").is_some());
            assert!(mapping_value(map, "memory").is_some());
            let vision = mapping_value(map, "vision").and_then(Value::as_mapping).unwrap();
            assert_eq!(
                mapping_value(vision, "variable_selector")
                    .and_then(Value::as_sequence)
                    .map(Vec::len),
                Some(2)
            );
        }
        if block_type == "knowledge-retrieval" {
            let multiple = mapping_value(map, "multiple_retrieval_config")
                .and_then(Value::as_mapping)
                .unwrap();
            assert!(mapping_value(multiple, "reranking_model").is_some());
            assert_eq!(
                mapping_value(multiple, "metadata_filtering_mode").and_then(Value::as_str),
                Some("manual")
            );
            assert!(mapping_value(multiple, "metadata_model_config").is_some());
        }
    }
}
