//! 数据整形节点的类型、顺序和未知字段无损往返测试。

use super::*;
use crate::apps::workflow::model::supported_node_types;

fn map_value<'a>(map: &'a Mapping, key: &str) -> &'a Value {
    mapping_value(map, key).unwrap_or_else(|| panic!("missing key {key}"))
}

#[test]
fn data_nodes_round_trip_typed_operations() {
    let cases = [
        (
            "variable-assigner",
            "version: '2'\nitems:\n  - variable_selector: [conversation, count]\n    operation: +=\n    input_type: constant\n    value: 2\n    vendor_item: keep\n  - variable_selector: [conversation, label]\n    operation: over-write\n    input_type: variable\n    value: [start, query]\nvendor_root: keep\n",
        ),
        (
            "variable-aggregator",
            "variables:\n  - [start, first]\n  - [code, second]\noutput_type: array[string]\nvendor_root: keep\n",
        ),
        (
            "list-operator",
            "variable: [start, items]\nfilter_by:\n  enabled: true\n  conditions:\n    - key: score\n      comparison_operator: greater than\n      value: 10\n      vendor_condition: keep\norder_by: { enabled: true, key: score, value: desc, vendor_order: keep }\nlimit: { enabled: true, size: 5, vendor_limit: keep }\nvendor_root: keep\n",
        ),
    ];

    for (block_type, raw) in cases {
        let draft = build_node_visual_draft(block_type, raw)
            .expect("draft should parse")
            .expect("data node should be visual");
        let synced = apply_visual_draft_to_yaml(block_type, raw, Some(&draft))
            .expect("visual draft should merge");
        let value: Value = serde_yaml::from_str(&synced).expect("round-trip yaml");
        let map = value.as_mapping().expect("data map");
        assert_eq!(map_value(map, "vendor_root").as_str(), Some("keep"));

        match block_type {
            "variable-assigner" => {
                let items = map_value(map, "items").as_sequence().expect("items");
                assert_eq!(items.len(), 2);
                let first = items[0].as_mapping().expect("assignment");
                assert_eq!(map_value(first, "value").as_u64(), Some(2));
                assert_eq!(map_value(first, "vendor_item").as_str(), Some("keep"));
            }
            "variable-aggregator" => {
                let variables = map_value(map, "variables").as_sequence().expect("variables");
                assert_eq!(variables[0].as_sequence().unwrap()[1].as_str(), Some("first"));
                assert_eq!(variables[1].as_sequence().unwrap()[1].as_str(), Some("second"));
                assert_eq!(map_value(map, "output_type").as_str(), Some("array[string]"));
            }
            "list-operator" => {
                let filter = map_value(map, "filter_by").as_mapping().expect("filter");
                let condition =
                    map_value(filter, "conditions").as_sequence().unwrap()[0].as_mapping().unwrap();
                assert_eq!(map_value(condition, "value").as_u64(), Some(10));
                assert_eq!(map_value(condition, "vendor_condition").as_str(), Some("keep"));
                assert_eq!(
                    map_value(map, "order_by")
                        .as_mapping()
                        .unwrap()
                        .get("vendor_order")
                        .and_then(Value::as_str),
                    Some("keep")
                );
                assert_eq!(
                    map_value(map, "limit")
                        .as_mapping()
                        .unwrap()
                        .get("vendor_limit")
                        .and_then(Value::as_str),
                    Some("keep")
                );
            }
            _ => unreachable!(),
        }
    }
}

#[test]
fn data_nodes_defaults_and_registry_are_visual() {
    for block_type in ["variable-assigner", "variable-aggregator", "list-operator"] {
        assert!(supported_node_types().iter().any(|item| item.block_type == block_type));
        let raw = default_node_data_yaml(block_type).expect("default YAML");
        assert!(build_node_visual_draft(block_type, &raw).expect("valid YAML").is_some());
    }
}
