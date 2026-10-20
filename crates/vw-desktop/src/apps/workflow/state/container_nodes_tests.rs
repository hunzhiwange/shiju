//! 迭代与循环容器的配置和未知字段无损往返测试。

use super::*;
use crate::apps::workflow::model::{load_document_from_value, serialize_workflow_yaml};

fn map_value<'a>(map: &'a Mapping, key: &str) -> &'a Value {
    mapping_value(map, key).unwrap_or_else(|| panic!("missing key {key}"))
}

#[test]
fn container_nodes_preserve_children_positions_and_edges() {
    let cases = [
        (
            "iteration",
            "iterator_selector: [start, items]\noutput_selector: [iteration, item]\noutput_type: array[string]\nis_parallel: true\nparallel_nums: 4\nerror_handle_mode: continue-on-error\nvendor_root: keep\n",
        ),
        (
            "loop",
            "loop_variables:\n  - id: counter\n    label: Counter\n    var_type: number\n    value: 0\n    value_type: constant\n    vendor_variable: keep\nbreak_conditions:\n  - id: done\n    varType: boolean\n    variable_selector: [check, done]\n    comparison_operator: is\n    value: true\n    vendor_condition: keep\nlogical_operator: and\nmax_loop_count: 12\nvendor_root: keep\n",
        ),
    ];

    for (block_type, raw) in cases {
        let draft = build_node_visual_draft(block_type, raw)
            .expect("container draft should parse")
            .expect("container should have a visual draft");
        let synced = apply_visual_draft_to_yaml(block_type, raw, Some(&draft))
            .expect("container draft should merge");
        let value: Value = serde_yaml::from_str(&synced).expect("round-trip yaml");
        let map = value.as_mapping().expect("data map");
        assert_eq!(map_value(map, "vendor_root").as_str(), Some("keep"));

        match block_type {
            "iteration" => {
                assert_eq!(map_value(map, "parallel_nums").as_u64(), Some(4));
                assert_eq!(map_value(map, "error_handle_mode").as_str(), Some("continue-on-error"));
            }
            "loop" => {
                assert_eq!(map_value(map, "max_loop_count").as_u64(), Some(12));
                let variable = &map_value(map, "loop_variables").as_sequence().unwrap()[0];
                assert_eq!(
                    map_value(variable.as_mapping().unwrap(), "vendor_variable").as_str(),
                    Some("keep")
                );
                let condition = &map_value(map, "break_conditions").as_sequence().unwrap()[0];
                assert_eq!(
                    map_value(condition.as_mapping().unwrap(), "vendor_condition").as_str(),
                    Some("keep")
                );
            }
            _ => unreachable!(),
        }
    }

    let raw_root: Value = serde_yaml::from_str(
        "app: {name: Containers}\nworkflow:\n  graph:\n    nodes:\n      - id: loop\n        position: {x: 10, y: 20}\n        positionAbsolute: {x: 10, y: 20}\n        width: 320\n        height: 220\n        data: {type: loop, title: Loop, loop_variables: [], break_conditions: [], logical_operator: and, max_loop_count: 10}\n      - id: loop-start\n        parentId: loop\n        position: {x: 30, y: 50}\n        positionAbsolute: {x: 30, y: 50}\n        data: {type: loop-start, title: Loop Start}\n      - id: child\n        parentId: loop\n        position: {x: 90, y: 70}\n        positionAbsolute: {x: 90, y: 70}\n        data: {type: code, title: Child}\n    edges:\n      - id: loop-internal\n        source: loop-start\n        sourceHandle: source\n        target: child\n        targetHandle: target\n        data: {sourceType: loop-start, targetType: code, isInLoop: true, vendor_edge: keep}\n",
    )
    .expect("container fixture yaml");
    let mut loaded = load_document_from_value(None, raw_root).expect("container fixture loads");
    loaded.document.node_mut("loop").unwrap().position.x += 5.0;
    for child_id in loaded.document.descendant_ids("loop") {
        loaded.document.node_mut(&child_id).unwrap().position.x += 5.0;
    }
    let saved = serialize_workflow_yaml(
        &loaded.app_meta,
        &loaded.document,
        &loaded.environment_variables,
        &loaded.conversation_variables,
        &loaded.raw_root,
        loaded.document.viewport,
    )
    .expect("container fixture saves");
    let reloaded: Value = serde_yaml::from_str(&saved).expect("saved yaml");
    let graph = &reloaded["workflow"]["graph"];
    let nodes = graph["nodes"].as_sequence().unwrap();
    let child = nodes.iter().find(|node| node["id"].as_str() == Some("child")).unwrap();
    assert_eq!(child["parentId"].as_str(), Some("loop"));
    assert_eq!(child["position"]["x"].as_f64(), Some(95.0));
    assert_eq!(child["positionAbsolute"]["x"].as_f64(), Some(95.0));
    let edge = &graph["edges"].as_sequence().unwrap()[0];
    assert_eq!(edge["sourceHandle"].as_str(), Some("source"));
    assert_eq!(edge["targetHandle"].as_str(), Some("target"));
    assert_eq!(edge["data"]["isInLoop"].as_bool(), Some(true));
    assert_eq!(edge["data"]["vendor_edge"].as_str(), Some("keep"));
}
