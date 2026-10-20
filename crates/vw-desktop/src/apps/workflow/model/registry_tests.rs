//! 工作流模型注册表的测试，覆盖节点类型查找、别名和展示元数据。
//! 用例确保未知类型明确回退，内置注册顺序与能力集合保持稳定。
#[test]
fn test_module_is_wired() {
    let module = std::hint::black_box(module_path!());

    assert!(module.ends_with("registry_tests"));
}

use super::*;

#[test]
fn workflow_system_variables_adds_chat_variables_before_base_for_chat_modes() {
    let chat_meta = WorkflowAppMeta { mode: "advanced-chat".to_string(), ..Default::default() };
    let workflow_meta = WorkflowAppMeta { mode: " workflow ".to_string(), ..Default::default() };

    let chat_variables = workflow_system_variables(&chat_meta);
    let workflow_variables = workflow_system_variables(&workflow_meta);

    assert_eq!(
        chat_variables.iter().map(|item| item.name).collect::<Vec<_>>(),
        vec![
            "sys.dialogue_count",
            "sys.conversation_id",
            "sys.user_id",
            "sys.app_id",
            "sys.workflow_id",
            "sys.workflow_run_id",
        ]
    );
    assert_eq!(
        workflow_variables.iter().map(|item| item.name).collect::<Vec<_>>(),
        vec!["sys.user_id", "sys.app_id", "sys.workflow_id", "sys.workflow_run_id"]
    );
    assert!(chat_variables.iter().all(|item| !item.value_type.is_empty()));
    assert!(chat_variables.iter().all(|item| !item.description.is_empty()));
}

#[test]
fn pretty_block_type_maps_known_trimmed_empty_and_unknown_values() {
    let cases = [
        (" start ", "开始"),
        ("end", "结束"),
        ("answer", "回复"),
        ("llm", "LLM"),
        ("if-else", "条件分支"),
        ("code", "代码"),
        ("tool", "工具"),
        ("knowledge-retrieval", "知识检索"),
        ("question-classifier", "问题分类"),
        ("http-request", "HTTP 请求"),
        ("template-transform", "模板转换"),
        ("parameter-extractor", "参数提取"),
        ("iteration", "迭代"),
        ("loop", "循环"),
        ("variable-assigner", "变量赋值"),
        ("variable-aggregator", "变量聚合"),
        ("list-operator", "列表操作"),
        ("agent", "Agent"),
        ("document-extractor", "文档提取"),
        ("  ", "节点"),
        ("custom-node", "custom node"),
    ];

    for (input, expected) in cases {
        assert_eq!(pretty_block_type(input), expected);
    }
}

#[test]
fn supported_node_types_exposes_ordered_descriptors() {
    let descriptors = supported_node_types();

    assert_eq!(descriptors.len(), 16);
    assert_eq!(descriptors.first().map(|item| item.block_type), Some("start"));
    assert_eq!(descriptors.first().map(|item| item.label), Some("开始"));
    assert_eq!(descriptors.last().map(|item| item.block_type), Some("agent"));
    assert!(descriptors.iter().all(|item| !item.summary.is_empty()));
    assert!(descriptors.iter().all(|item| item.icon.family == "lucide"));
}

#[test]
fn workflow_node_icon_maps_known_types_and_default() {
    let cases = [
        (" start ", "play"),
        ("end", "circle-check"),
        ("answer", "message-square-text"),
        ("llm", "bot"),
        ("if-else", "git-branch"),
        ("code", "braces"),
        ("tool", "wrench"),
        ("knowledge-retrieval", "database"),
        ("question-classifier", "circle-question-mark"),
        ("http-request", "globe"),
        ("template-transform", "replace-all"),
        ("parameter-extractor", "scan-text"),
        ("document-extractor", "file-text"),
        ("iteration", "iteration-cw"),
        ("loop", "refresh-cw"),
        ("variable-assigner", "variable"),
        ("variable-aggregator", "combine"),
        ("list-operator", "list-filter"),
        ("agent", "bot-message-square"),
        ("unknown", "workflow"),
    ];

    for (input, expected_name) in cases {
        let icon = workflow_node_icon(input);
        assert_eq!(icon.family, "lucide");
        assert_eq!(icon.name, expected_name);
    }
}

#[test]
fn workflow_node_accent_color_maps_known_types_and_default() {
    let cases = [
        (" start ", Color::from_rgb8(0x3B, 0x82, 0xF6)),
        ("end", Color::from_rgb8(0x14, 0xB8, 0xA6)),
        ("answer", Color::from_rgb8(0xF5, 0x9E, 0x0B)),
        ("llm", Color::from_rgb8(0x8B, 0x5C, 0xF6)),
        ("if-else", Color::from_rgb8(0x0E, 0xAE, 0xC7)),
        ("code", Color::from_rgb8(0xF9, 0x73, 0x16)),
        ("tool", Color::from_rgb8(0x63, 0x66, 0xF1)),
        ("knowledge-retrieval", Color::from_rgb8(0xF5, 0x73, 0x2E)),
        ("question-classifier", Color::from_rgb8(0x06, 0xB6, 0xD4)),
        ("http-request", Color::from_rgb8(0xEC, 0x48, 0x99)),
        ("template-transform", Color::from_rgb8(0x8B, 0x5C, 0xF6)),
        ("parameter-extractor", Color::from_rgb8(0x4F, 0x46, 0xE5)),
        ("document-extractor", Color::from_rgb8(0x47, 0x72, 0x9D)),
        ("iteration", Color::from_rgb8(0x10, 0xB9, 0x81)),
        ("loop", Color::from_rgb8(0x38, 0xB2, 0xAC)),
        ("variable-assigner", Color::from_rgb8(0x22, 0xC5, 0x5E)),
        ("variable-aggregator", Color::from_rgb8(0x0F, 0x76, 0x94)),
        ("list-operator", Color::from_rgb8(0x25, 0x63, 0xEB)),
        ("agent", Color::from_rgb8(0xE1, 0x1D, 0x48)),
        ("unknown", Color::from_rgb8(0x64, 0x74, 0x8B)),
    ];

    for (input, expected) in cases {
        assert_eq!(workflow_node_accent_color(input), expected);
    }
}
